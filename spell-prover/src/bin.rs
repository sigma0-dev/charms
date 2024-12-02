use sha2::{Digest, Sha256};
use crate::{
    v0::{V0AppContractProof, V0SpellProof},
    AppContractProof, SpellProof, SpellProverInput, V0,
};
use charms_data::AppId;

pub fn main() {
    // Read an input to the program.
    let SpellProverInput {
        self_spell_vk,
        spell,
        pre_req_spell_proofs,
        app_contract_proofs,
    } = sp1_zkvm::io::read();

    let cm = {
        let committed_input_data = bincode::serialize(&(&pre_req_spell_proofs, &app_contract_proofs)).unwrap();
        Sha256::digest(&committed_input_data)
    };

    let pre_req_spell_proofs = pre_req_spell_proofs
        .into_iter()
        .map(|(txid, (spell, cm, proof_data))| {
            let spell_proof = to_spell_proof(spell.version, self_spell_vk.clone(), proof_data);
            (txid, (spell, cm, spell_proof))
        })
        .collect();

    let app_contract_proofs = app_contract_proofs
        .into_iter()
        .map(|(app_id, proof_data)| {
            let app_contract_proof = to_app_contract_proof(&app_id, proof_data);
            (app_id, app_contract_proof)
        })
        .collect();

    // Check the spell that we're proving is correct.
    assert!(spell.is_correct(&pre_req_spell_proofs, &app_contract_proofs));

    // Commit to the public values of the program.
    sp1_zkvm::io::commit(&(&self_spell_vk, &spell, cm.as_slice()));
}

pub fn to_spell_proof(
    version: u32,
    self_spell_vk: [u8; 32],
    proof_data: Option<Box<[u8]>>,
) -> Box<dyn SpellProof> {
    match version {
        V0 => Box::new(V0SpellProof {
            vk: self_spell_vk,
            proof: proof_data,
        }),
        _ => unreachable!(),
    }
}

fn to_app_contract_proof<'a>(
    app_id: &AppId,
    proof_data: Option<Box<[u8]>>,
) -> Box<dyn AppContractProof> {
    let app_contract_proof = proof_data.map_or(
        V0AppContractProof {
            vk: None,
            proof: None,
        },
        |proof_data| V0AppContractProof {
            vk: Some(app_id.vk_hash.0),
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
