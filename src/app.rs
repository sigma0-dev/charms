use charms_data::{App, Data, Transaction, VkHash};
use sp1_sdk::{HashableKey, ProverClient, SP1Proof, SP1Stdin};
use std::{collections::BTreeMap, mem};

pub struct Prover {
    pub client: ProverClient,
}

impl Prover {
    pub fn vk(&self, binary: &[u8]) -> [u8; 32] {
        let (_pk, vk) = self.client.setup(&binary);
        unsafe {
            let vk: [u32; 8] = vk.hash_u32();
            mem::transmute(vk)
        }
    }
}

impl Prover {
    pub fn new() -> Self {
        Self {
            client: ProverClient::local(),
        }
    }

    pub fn prove(
        &self,
        app_binaries: &BTreeMap<VkHash, Vec<u8>>,
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
            let Some((pk, vk)) = pk_vks.get(&app.vk_hash) else {
                eprintln!("app binary not present: {:?}", app);
                continue;
            };
            let mut app_stdin = SP1Stdin::new();
            let empty = Data::empty();
            let w = app_private_inputs.get(app).unwrap_or(&empty);
            app_stdin.write(&(app, &tx, x, w));
            let app_proof = self.client.prove(pk, app_stdin).compressed().run()?;

            let SP1Proof::Compressed(compressed_proof) = app_proof.proof else {
                unreachable!()
            };
            dbg!(app);
            eprintln!("app proof generated!");
            spell_stdin.write_proof(*compressed_proof, vk.vk.clone());
        }

        Ok(())
    }
}
