pub mod bin;
pub mod v0;

use charms_data::{AppId, Charm, Data, Transaction, TxId, Utxo, UtxoId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const V0: u32 = 0u32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellProverInput {
    pub self_spell_vk: [u8; 32],
    pub pre_req_spell_proofs: Vec<(TxId, (NormalizedSpell, Option<Box<[u8]>>))>,
    pub spell: NormalizedSpell,
    pub app_contract_proofs: Vec<(AppId, Option<Box<[u8]>>)>,
}

pub type NormalizedCharm = BTreeMap<usize, Data>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedTransaction {
    pub ins: Vec<UtxoId>,
    pub refs: Vec<UtxoId>,
    /// When proving correctness of a spell, we can't know the transaction ID yet.
    /// We only know the index of each output charm.
    pub outs: Vec<NormalizedCharm>,
}

impl NormalizedTransaction {
    pub fn pre_req_txids(&self) -> BTreeSet<TxId> {
        self.ins
            .iter()
            .chain(self.refs.iter())
            .map(|utxo_id| utxo_id.0)
            .collect()
    }
}

/// Can be committed as public input.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedSpell {
    pub version: u32,
    pub tx: NormalizedTransaction,
    /// Maps all `AppId`s in the transaction to (potentially empty) data.
    pub app_public_inputs: BTreeMap<AppId, Data>,
}

impl NormalizedSpell {
    pub fn well_formed(
        &self,
        pre_req_spell_proofs: &BTreeMap<TxId, (NormalizedSpell, Box<dyn SpellProof>)>,
    ) -> bool {
        let created_by_pre_req_spells = |utxo_id: &UtxoId| -> bool {
            pre_req_spell_proofs
                .get(&utxo_id.0)
                .and_then(|(pre_req_spell, _)| pre_req_spell.tx.outs.get(utxo_id.1 as usize))
                .is_some()
        };
        match self.version {
            V0 => {
                if !self
                    .tx
                    .outs
                    .iter()
                    .all(|n_charm| n_charm.keys().all(|i| i < &self.app_public_inputs.len()))
                {
                    return false;
                }
                // check that UTXOs we're spending or referencing in this tx
                // are created by pre-req transactions
                if !self.tx.ins.iter().all(created_by_pre_req_spells)
                    || !self.tx.refs.iter().all(created_by_pre_req_spells)
                {
                    return false;
                }
                true
            }
            _ => false,
        }
    }

    pub fn app_ids(&self) -> Vec<AppId> {
        self.app_public_inputs.keys().cloned().collect()
    }

    pub fn to_tx(
        &self,
        pre_req_spell_proofs: &BTreeMap<TxId, (NormalizedSpell, Box<dyn SpellProof>)>,
    ) -> Transaction {
        let from_utxo_id = |utxo_id: &UtxoId| -> Utxo {
            let pre_req_spell_data = &pre_req_spell_proofs[&utxo_id.0].0;
            let pre_req_charm_data = &pre_req_spell_data.tx.outs[utxo_id.1 as usize];
            Utxo {
                id: Some(utxo_id.clone()),
                charm: pre_req_spell_data.to_charm(pre_req_charm_data),
            }
        };

        let from_normalized_charm = |n_charm: &NormalizedCharm| -> Utxo {
            Utxo {
                id: None,
                charm: self.to_charm(n_charm),
            }
        };

        Transaction {
            ins: self.tx.ins.iter().map(from_utxo_id).collect(),
            refs: self.tx.refs.iter().map(from_utxo_id).collect(),
            outs: self.tx.outs.iter().map(from_normalized_charm).collect(),
        }
    }

    pub fn is_correct(
        &self,
        pre_req_spell_proofs: &BTreeMap<TxId, (NormalizedSpell, Box<dyn SpellProof>)>,
        app_contract_proofs: &Vec<(AppId, Box<dyn AppContractProof>)>,
    ) -> bool {
        if !self.well_formed(pre_req_spell_proofs) {
            return false;
        }
        let pre_req_txids = self.tx.pre_req_txids();
        if pre_req_txids.len() != pre_req_spell_proofs.len() {
            return false;
        }
        if !pre_req_txids
            .iter()
            .zip(pre_req_spell_proofs)
            .all(|(txid0, (txid, (n_spell, proof)))| txid == txid0 && proof.verify(n_spell))
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
                    && proof.verify(
                        app_id,
                        &self.to_tx(pre_req_spell_proofs),
                        &self.app_public_inputs[app_id],
                    )
            })
        {
            return false;
        }

        true
    }

    fn to_charm(&self, n_charm: &NormalizedCharm) -> Charm {
        let app_ids = self.app_ids();
        n_charm
            .iter()
            .map(|(&i, data)| (app_ids[i].clone(), data.clone()))
            .collect()
    }
}

pub trait SpellProof {
    /// Verify the proof that the spell is correct.
    fn verify(&self, n_spell: &NormalizedSpell) -> bool;
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
