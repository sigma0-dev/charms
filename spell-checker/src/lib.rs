pub mod v0;

use charms_data::{AppId, Charm, Data, Transaction, TxId, Utxo, UtxoId, VkHash};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellProverInput {
    pub self_spell_vk: String,
    pub pre_req_spell_proofs: Vec<(TxId, (SpellData, Option<Box<[u8]>>))>,
    pub app_vks: BTreeMap<VkHash, String>,
    pub spell: SpellData,
    pub app_contract_proofs: Vec<(AppId, Option<Box<[u8]>>)>,
}

pub type CharmData = BTreeMap<usize, Data>;

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
    pub fn well_formed(&self) -> bool {
        let is_well_formed = |charm_data: &CharmData| -> bool {
            charm_data
                .iter()
                .all(|(i, _)| i < &self.public_inputs.len())
        };
        match self.version {
            0u32 => {}
            _ => return false,
        }
        if !self.tx.ins.iter().all(|utxo| is_well_formed(&utxo.charm)) {
            return false;
        }
        if !self.tx.refs.iter().all(|utxo| is_well_formed(&utxo.charm)) {
            return false;
        }
        if !self.tx.outs.iter().all(|charm| is_well_formed(charm)) {
            return false;
        }

        true
    }

    pub fn app_ids(&self) -> Vec<AppId> {
        self.public_inputs.keys().cloned().collect()
    }

    pub fn to_tx(&self) -> Transaction {
        let app_ids = self.app_ids();

        let to_charm = |charm_data: &CharmData| -> Charm {
            charm_data
                .iter()
                .map(|(&i, data)| (app_ids[i].clone(), data.clone()))
                .collect()
        };

        let from_utxo_data = |utxo_data: &UtxoData| -> Utxo {
            Utxo {
                id: Some(utxo_data.id.clone()),
                charm: to_charm(&utxo_data.charm),
            }
        };

        let from_charm_data = |charm_data: &CharmData| -> Utxo {
            Utxo {
                id: None,
                charm: to_charm(charm_data),
            }
        };

        Transaction {
            ins: self.tx.ins.iter().map(from_utxo_data).collect(),
            refs: self.tx.refs.iter().map(from_utxo_data).collect(),
            outs: self.tx.outs.iter().map(from_charm_data).collect(),
        }
    }

    pub fn is_correct(
        &self,
        pre_req_spell_proofs: &Vec<(TxId, (SpellData, Box<dyn SpellProof>))>,
        app_contract_proofs: &Vec<(AppId, Box<dyn AppContractProof>)>,
    ) -> bool {
        if !self.well_formed() {
            return false;
        }
        let pre_req_txids = self.tx.pre_req_txids();
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

        let app_ids = self.app_ids();
        if app_ids.len() != app_contract_proofs.len() {
            return false;
        }
        if !app_ids
            .iter()
            .zip(app_contract_proofs)
            .all(|(app_id0, (app_id, proof))| {
                app_id == app_id0
                    && proof.verify(app_id, &self.to_tx(), &self.public_inputs[app_id])
            })
        {
            return false;
        }

        true
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

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
