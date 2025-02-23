pub mod app;
pub mod bin;

use crate::app::AppContractVK;
use charms_client::NormalizedSpell;
use charms_data::App;

/// Check if the spell is correct.
pub(crate) fn is_correct(
    spell: &NormalizedSpell,
    prev_txs: &Vec<bitcoin::Transaction>,
    app_contract_vks: &Vec<(App, AppContractVK)>,
    spell_vk: &String,
) -> bool {
    let prev_spells = charms_client::prev_spells(prev_txs, spell_vk);
    if !charms_client::well_formed(spell, &prev_spells) {
        eprintln!("not well formed");
        return false;
    }
    let Some(prev_txids) = spell.tx.prev_txids() else {
        unreachable!("the spell is well formed: tx.ins MUST be Some");
    };
    if prev_txids != prev_spells.keys().collect() {
        eprintln!("spell.tx.prev_txids() != prev_spells.keys()");
        return false;
    }

    let apps = charms_client::apps(spell);
    if apps.len() != app_contract_vks.len() {
        eprintln!("apps.len() != app_contract_proofs.len()");
        return false;
    }
    if !apps
        .iter()
        .zip(app_contract_vks)
        .all(|(app0, (app, proof))| {
            app == app0
                && proof.verify(
                    app,
                    &charms_client::to_tx(spell, &prev_spells),
                    &spell.app_public_inputs[app],
                )
        })
    {
        eprintln!("app_contract_proofs verification failed");
        return false;
    }

    true
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
