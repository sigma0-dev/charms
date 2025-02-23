use crate::{app::AppContractVK, is_correct};
use charms_client::{NormalizedSpell, SpellProverInput};
use charms_data::{util, App};

pub fn main() {
    // Read an input to the program.
    let input_vec = sp1_zkvm::io::read_vec();
    let input: SpellProverInput = util::read(input_vec.as_slice()).unwrap();

    let output = run(input);

    eprintln!("about to commit");

    // Commit to the public values of the program.
    let output_vec = util::write(&output).unwrap();
    sp1_zkvm::io::commit_slice(output_vec.as_slice());
}

pub(crate) fn run(input: SpellProverInput) -> (String, NormalizedSpell) {
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
            let app_contract_proof = to_app_contract_vk(app, app_contract_proofs.contains(&i));
            (app.clone(), app_contract_proof)
        })
        .collect();

    // Check the spell that we're proving is correct.
    assert!(is_correct(
        &spell,
        &prev_txs,
        &app_contract_proofs,
        &self_spell_vk
    ));

    eprintln!("Spell is correct!");

    (self_spell_vk, spell)
}

/// Get app contract VK and proof for the given app.
fn to_app_contract_vk(app: &App, has_proof: bool) -> AppContractVK {
    // It only makes sense for the **current version** of spell prover, so we don't need to pass
    // version.
    let app_contract_proof = match has_proof {
        false => AppContractVK { vk: None },
        true => {
            let vk: [u32; 8] = unsafe {
                let vk: [u8; 32] = app.vk.0;
                std::mem::transmute(vk)
            };
            AppContractVK { vk: Some(vk) }
        }
    };
    app_contract_proof
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
