pub mod sp1v3;

use charms_data::{AppId, Data, Transaction, TxId};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spell {
    pub tx: Transaction,
    pub public_inputs: BTreeMap<AppId, Data>,
}

impl Spell {
    pub fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).unwrap()
    }
}

pub trait SpellProof {
    /// Verify the proof that the spell is correct.
    fn verify(&self, spell: &Spell) -> bool;
}

pub trait AppContractProof {
    /// Verify the proof that the app contract is satisfied by the transaction and public input.
    fn verify(&self, app_id: &AppId, tx: &Transaction, x: &Data) -> bool;
}

pub fn check(
    spell: &Spell,
    pre_req_spell_proofs: &BTreeMap<TxId, (Box<dyn SpellProof>, Spell)>,
    app_contract_proofs: &BTreeMap<AppId, Box<dyn AppContractProof>>,
) -> bool {
    let pre_req_txids = spell.tx.pre_req_txids();
    if pre_req_txids.len() != pre_req_spell_proofs.len() {
        return false;
    }
    if !pre_req_txids
        .iter()
        .zip(pre_req_spell_proofs)
        .all(|(txid0, (txid, (proof, spell)))| txid == txid0 && proof.verify(&spell))
    {
        return false;
    }

    let app_ids = spell.tx.app_ids();
    if app_ids.len() != app_contract_proofs.len() {
        return false;
    }
    let empty_data = Data::default();
    if !app_ids
        .iter()
        .zip(app_contract_proofs)
        .all(|(app_id0, (app_id, proof))| {
            app_id == app_id0
                && proof.verify(
                    app_id,
                    &spell.tx,
                    &spell.public_inputs.get(app_id).unwrap_or(&empty_data),
                )
        })
    {
        return false;
    }

    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {}
}
