use crate::{v0::V0AppContractProof, AppContractProof, NormalizedSpell, SpellProverInput};
use charms_data::App;

pub fn main() {
    // Read an input to the program.
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
        prev_txs,
        spell,
        app_contract_proofs,
    } = input;

    let app_contract_proofs = spell
        .app_public_inputs
        .iter()
        .zip(0..)
        .map(|((app, _), i)| {
            let app_contract_proof = to_app_contract_proof(app, app_contract_proofs.contains(&i));
            (app.clone(), app_contract_proof)
        })
        .collect();

    // Check the spell that we're proving is correct.
    assert!(spell.is_correct(&prev_txs, &app_contract_proofs, &self_spell_vk));

    eprintln!("Spell is correct!");

    (self_spell_vk, spell)
}

/// Get app contract VK and proof for the given app.
fn to_app_contract_proof(app: &App, has_proof: bool) -> Box<dyn AppContractProof> {
    // It only makes sense for the **current version** of spell prover, so we don't need to pass
    // version.
    let app_contract_proof = match has_proof {
        false => V0AppContractProof { vk: None },
        true => {
            let vk: [u32; 8] = unsafe {
                let vk: [u8; 32] = app.vk.0; // app.vk_hash is the VK of the app contract in V0
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
