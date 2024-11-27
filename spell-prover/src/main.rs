#![no_main]
sp1_zkvm::entrypoint!(main);

use charms_data::{AppId, TxId};
use serde::{Deserialize, Serialize};
use spell_checker::{check, v0::V0SpellProof, AppContractProof, SpellData, SpellProof};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellProverInput {
    pub v0_spell_vk: String,
    pub spell: SpellData,
    pub pre_req_spell_proofs: Vec<(TxId, (SpellData, Option<Box<[u8]>>))>,
    pub app_contract_proofs: Vec<(AppId, Option<String>, Option<Box<[u8]>>)>,
}

pub fn main() {
    // Read an input to the program.
    let SpellProverInput {
        v0_spell_vk,
        spell,
        pre_req_spell_proofs,
        app_contract_proofs,
    } = sp1_zkvm::io::read();

    let pre_req_spell_proofs = pre_req_spell_proofs
        .into_iter()
        .map(|(txid, (spell, proof_data))| {
            let spell_proof = to_spell_proof(&v0_spell_vk, &spell, &proof_data);
            (txid, (spell, spell_proof))
        })
        .collect();

    // Check the spell that we're proving is correct.
    assert!(check(&spell, &pre_req_spell_proofs, todo!()));

    // Commit to the public values of the program.
    sp1_zkvm::io::commit(&spell);
}

pub fn to_spell_proof<'a>(
    v0_spell_vk: &'a str,
    spell: &SpellData,
    proof_data: &Option<Box<[u8]>>,
) -> Box<dyn SpellProof + 'a> {
    match spell.version {
        0u32 => Box::new(V0SpellProof {
            vk_bytes32: v0_spell_vk,
            proof: proof_data.clone(),
        }),
        _ => unreachable!(),
    }
}

pub fn to_app_contract_proof() -> Box<dyn AppContractProof> {
    todo!()
}

mod test {
    use super::*;

    #[test]
    fn dummy() {}
}
