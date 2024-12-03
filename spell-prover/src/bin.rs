use crate::{
    v0::{V0AppContractProof, V0SpellProof},
    AppContractProof, SpellProof, SpellProverInput, CURRENT_VERSION,
};
use charms_data::App;

pub fn main() {
    // Read an input to the program.
    let SpellProverInput {
        self_spell_vk,
        spell,
        pre_req_spell_proofs,
        app_contract_proofs,
    } = sp1_zkvm::io::read();

    let pre_req_spell_proofs = pre_req_spell_proofs
        .into_iter()
        .map(|(txid, (n_spell, proof_data))| {
            let spell_proof = to_spell_proof(n_spell.version, self_spell_vk.clone(), proof_data);
            (txid, (n_spell, spell_proof))
        })
        .collect();

    let app_contract_proofs = app_contract_proofs
        .into_iter()
        .map(|(app, proof_data)| {
            let app_contract_proof = to_app_contract_proof(&app, proof_data);
            (app, app_contract_proof)
        })
        .collect();

    // Check the spell that we're proving is correct.
    assert!(spell.is_correct(&pre_req_spell_proofs, &app_contract_proofs));

    // Commit to the public values of the program.
    sp1_zkvm::io::commit(&(&self_spell_vk, &spell));
}

/// Get spell VK and proof for the **given version** of spell prover.
///
/// We're passing `self_spell_vk` because we can't have the VK for the
/// current version as a constant.
pub fn to_spell_proof(
    version: u32,
    self_spell_vk: [u8; 32],
    proof_data: Option<Box<[u8]>>,
) -> Box<dyn SpellProof> {
    match version {
        CURRENT_VERSION => Box::new(V0SpellProof {
            vk: self_spell_vk,
            proof: proof_data,
        }),
        _ => unreachable!(),
    }
}

/// Get app contract VK and proof for the given app.
fn to_app_contract_proof(app: &App, proof_data: Option<Box<[u8]>>) -> Box<dyn AppContractProof> {
    // It only makes sense for the **current version** of spell prover, so we don't need to pass
    // version.
    let app_contract_proof = proof_data.map_or(
        V0AppContractProof {
            vk: None,
            proof: None,
        },
        |proof_data| V0AppContractProof {
            vk: Some(app.vk_hash.0), // app.vk_hash is the VK of the app contract in V0
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
