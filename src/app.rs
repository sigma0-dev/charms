use anyhow::ensure;
use charms_data::{util, App, Data, Transaction, B32};
use sp1_prover::components::CpuProverComponents;
use sp1_sdk::{HashableKey, ProverClient, SP1Proof, SP1ProofMode, SP1Stdin, SP1VerifyingKey};
use std::{collections::BTreeMap, mem};

pub struct Prover {
    pub client: Box<dyn sp1_sdk::Prover<CpuProverComponents>>,
}

impl Prover {
    pub fn vk(&self, binary: &[u8]) -> [u8; 32] {
        let (_pk, vk) = self.client.setup(&binary);
        app_vk(vk)
    }
}

fn app_vk(sp1_vk: SP1VerifyingKey) -> [u8; 32] {
    unsafe {
        let vk: [u32; 8] = sp1_vk.hash_u32();
        mem::transmute(vk)
    }
}

impl Prover {
    pub fn new() -> Self {
        Self {
            client: Box::new(ProverClient::builder().cpu().build()),
        }
    }

    pub(crate) fn prove(
        &self,
        app_binaries: &BTreeMap<B32, Vec<u8>>,
        tx: Transaction,
        app_public_inputs: &BTreeMap<App, Data>,
        app_private_inputs: BTreeMap<App, Data>,
        spell_stdin: &mut SP1Stdin,
    ) -> anyhow::Result<()> {
        let pk_vks = app_binaries
            .iter()
            .map(|(vk_hash, binary)| {
                let (pk, vk) = self.client.setup(binary);
                (vk_hash, (pk, vk))
            })
            .collect::<BTreeMap<_, _>>();

        for (app, x) in app_public_inputs {
            let Some((pk, vk)) = pk_vks.get(&app.vk) else {
                eprintln!("app binary not present: {:?}", app);
                continue;
            };
            let mut app_stdin = SP1Stdin::new();
            let empty = Data::empty();
            let w = app_private_inputs.get(app).unwrap_or(&empty);
            app_stdin.write_vec(util::write(&(app, &tx, x, w))?);
            let app_proof = self
                .client
                .prove(pk, &app_stdin, SP1ProofMode::Compressed)?;

            let SP1Proof::Compressed(compressed_proof) = app_proof.proof else {
                unreachable!()
            };
            dbg!(app);
            eprintln!("app proof generated!");
            spell_stdin.write_proof(*compressed_proof, vk.vk.clone());
        }

        Ok(())
    }

    pub fn run(
        &self,
        app_binary: &[u8],
        app: &App,
        tx: &Transaction,
        x: &Data,
        w: &Data,
    ) -> anyhow::Result<()> {
        let (_pk, vk) = self.client.setup(app_binary);
        ensure!(app.vk == B32(app_vk(vk)), "app.vk mismatch");

        let mut app_stdin = SP1Stdin::new();
        app_stdin.write_vec(util::write(&(app, tx, x, w))?);
        let (committed_values, _report) = self.client.execute(app_binary, &app_stdin)?;
        let com: (App, Transaction, Data) = util::read(committed_values.to_vec().as_slice())?;
        ensure!(
            (&com.0, &com.1, &com.2) == (app, tx, x),
            "committed data mismatch"
        );
        Ok(())
    }
}
