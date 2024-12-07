use crate::{
    v0::{V0AppContractProof, V0SpellProof},
    AppContractProof, NormalizedSpell, SpellProof, SpellProverInput, CURRENT_VERSION,
};
use charms_data::App;

pub fn main() {
    // Read an input to the program.
    // let input: SpellProverInput = sp1_zkvm::io::read();
    let input_vec = sp1_zkvm::io::read_vec();

    dbg!(input_vec.len());

    let input: SpellProverInput = ciborium::from_reader(input_vec.as_slice()).unwrap();

    dbg!(&input);

    let output = run(input);

    eprintln!("about to commit");

    // Commit to the public values of the program.
    sp1_zkvm::io::commit(&output);
}

pub fn run(input: SpellProverInput) -> (String, NormalizedSpell) {
    let SpellProverInput {
        self_spell_vk,
        spell,
        prev_spell_proofs,
        app_contract_proofs,
    } = input;

    let prev_spell_proofs = prev_spell_proofs
        .into_iter()
        .map(|(txid, (n_spell, proof_data))| {
            let spell_proof = to_spell_proof(n_spell.version, self_spell_vk.clone(), proof_data);
            (txid, (n_spell, spell_proof))
        })
        .collect();

    let app_contract_proofs = app_contract_proofs
        .into_iter()
        .map(|(app, proof_opt)| {
            let app_contract_proof = to_app_contract_proof(&app, proof_opt);
            (app, app_contract_proof)
        })
        .collect();

    // Check the spell that we're proving is correct.
    assert!(spell.is_correct(&prev_spell_proofs, &app_contract_proofs));

    eprintln!("Spell is correct!");

    (self_spell_vk, spell)
}

/// Get spell VK and proof for the **given version** of spell prover.
///
/// We're passing `self_spell_vk` because we can't have the VK for the
/// current version as a constant.
pub fn to_spell_proof(
    version: u32,
    self_spell_vk: String,
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
fn to_app_contract_proof(app: &App, has_proof: bool) -> Box<dyn AppContractProof> {
    // It only makes sense for the **current version** of spell prover, so we don't need to pass
    // version.
    let app_contract_proof = match has_proof {
        false => V0AppContractProof { vk: None },
        true => {
            let vk: [u32; 8] = unsafe {
                let vk: [u8; 32] = app.vk_hash.0; // app.vk_hash is the VK of the app contract in V0
                std::mem::transmute(vk)
            };
            V0AppContractProof { vk: Some(vk) }
        }
    };
    Box::new(app_contract_proof)
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
