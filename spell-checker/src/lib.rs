pub mod v0;

use charms_data::{AppId, Data, Transaction, TxId, Utxo, UtxoId};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type CharmData = BTreeMap<u32, Data>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UtxoData {
    pub id: UtxoId,
    pub charm: CharmData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionData {
    pub ins: Vec<UtxoData>,
    pub refs: Vec<UtxoData>,
    /// When proving correctness of a spell, we can't know the transaction ID yet.
    /// We only know the index of each output charm.
    pub outs: Vec<CharmData>,
}

impl TransactionData {
    pub fn pre_req_txids(&self) -> BTreeSet<TxId> {
        let mut txids = BTreeSet::new();
        for utxo in self.ins.iter().chain(self.refs.iter()) {
            txids.insert(utxo.id.0);
        }
        txids
    }
}

/// Can be committed as public input.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellData {
    pub version: u32,
    pub tx: TransactionData,
    /// Maps all `AppId`s in the transaction to (potentially empty) data.
    pub public_inputs: BTreeMap<AppId, Data>,
}

impl SpellData {
    pub fn app_ids(&self) -> Vec<AppId> {
        self.public_inputs.keys().cloned().collect()
    }

    pub fn to_tx(&self) -> Transaction {
        todo!()
    }
}

pub trait SpellProof {
    /// Verify the proof that the spell is correct.
    fn verify(&self, spell: &SpellData) -> bool;
}

pub trait AppContractProof {
    /// Verify the proof that the app contract is satisfied by the transaction and public input.
    fn verify(&self, app_id: &AppId, tx: &Transaction, x: &Data) -> bool;
}

pub fn check(
    spell: &SpellData,
    pre_req_spell_proofs: &Vec<(TxId, (SpellData, Box<dyn SpellProof>))>,
    app_contract_proofs: &Vec<(AppId, Box<dyn AppContractProof>)>,
) -> bool {
    let pre_req_txids = spell.tx.pre_req_txids();
    if pre_req_txids.len() != pre_req_spell_proofs.len() {
        return false;
    }
    if !pre_req_txids
        .iter()
        .zip(pre_req_spell_proofs)
        .all(|(txid0, (txid, (spell, proof)))| txid == txid0 && proof.verify(&spell))
    {
        return false;
    }

    let app_ids = spell.app_ids();
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
                    &spell.to_tx(),
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
    fn dummy() {}
}
