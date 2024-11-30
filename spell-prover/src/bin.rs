use std::collections::BTreeMap;
use charms_data::{AppId, VkHash};
use crate::{AppContractProof, SpellProof, SpellProverInput};
use crate::v0::{V0AppContractProof, V0SpellProof};

pub fn main() {
    // Read an input to the program.
    let SpellProverInput {
        self_spell_vk,
        app_vks,
        spell,
        pre_req_spell_proofs,
        app_contract_proofs,
    } = sp1_zkvm::io::read();

    let pre_req_spell_proofs = pre_req_spell_proofs
        .into_iter()
        .map(|(txid, (spell, proof_data))| {
            let spell_proof = to_spell_proof(spell.version, self_spell_vk.clone(), proof_data);
            (txid, (spell, spell_proof))
        })
        .collect();

    let app_contract_proof_mapping = |(app_id, proof_data)| {
        let app_contract_proof = to_app_contract_proof(&app_vks, &app_id, proof_data);
        (app_id, app_contract_proof)
    };
    let app_contract_proofs = app_contract_proofs
        .into_iter()
        .map(app_contract_proof_mapping)
        .collect();

    // Check the spell that we're proving is correct.
    assert!(spell.is_correct(&pre_req_spell_proofs, &app_contract_proofs));

    // Commit to the public values of the program.
    sp1_zkvm::io::commit(&(&self_spell_vk, &spell));
}

pub fn to_spell_proof(
    version: u32,
    self_spell_vk: String,
    proof_data: Option<Box<[u8]>>,
) -> Box<dyn SpellProof> {
    match version {
        0u32 => Box::new(V0SpellProof {
            vk_bytes32: self_spell_vk,
            proof: proof_data,
        }),
        _ => unreachable!(),
    }
}

fn to_app_contract_proof<'a>(
    app_vks: &BTreeMap<VkHash, String>,
    app_id: &AppId,
    proof_data: Option<Box<[u8]>>,
) -> Box<dyn AppContractProof> {
    let app_contract_proof = proof_data.map_or(
        V0AppContractProof {
            vk_bytes32: None,
            proof: None,
        },
        |proof_data| V0AppContractProof {
            vk_bytes32: Some(app_vks[&app_id.vk_hash].clone()),
            proof: Some(proof_data),
        },
    );
    Box::new(app_contract_proof)
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}