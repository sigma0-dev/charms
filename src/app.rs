use charms_data::{Data, TxId, VkHash};
use sp1_sdk::{HashableKey, ProverClient, SP1Proof, SP1Stdin};
use spell_prover::NormalizedSpell;
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
        norm_spell: &NormalizedSpell,
        prev_spells: &BTreeMap<TxId, NormalizedSpell>,
        spell_stdin: &mut SP1Stdin,
    ) {
        let tx = norm_spell.to_tx(prev_spells);

        let pk_vks = app_binaries
            .iter()
            .map(|(vk_hash, binary)| {
                let (pk, vk) = self.client.setup(binary);
                (vk_hash, (pk, vk))
            })
            .collect::<BTreeMap<_, _>>();

        norm_spell.app_public_inputs.iter().for_each(|(app, x)| {
            let (pk, vk) = pk_vks.get(&app.vk_hash).unwrap();
            let mut app_stdin = SP1Stdin::new();
            app_stdin.write(&(app, &tx, x, Data::empty())); // TODO write private input instead of empty data
            let app_proof = self.client.prove(pk, app_stdin).compressed().run().unwrap();

            let SP1Proof::Compressed(compressed_proof) = app_proof.proof else {
                unreachable!()
            };
            spell_stdin.write_proof(*compressed_proof, vk.vk.clone());
        });
    }
}
