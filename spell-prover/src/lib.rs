pub mod bin;
pub mod v0;

use charms_data::{App, Charm, Data, Transaction, TxId, UtxoId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const V0: u32 = 0u32;
pub const CURRENT_VERSION: u32 = V0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellProverInput {
    pub self_spell_vk: [u8; 32],
    pub pre_req_spell_proofs: Vec<(TxId, (NormalizedSpell, Option<Box<[u8]>>))>,
    pub spell: NormalizedSpell,
    pub app_contract_proofs: Vec<(App, Option<Box<[u8]>>)>,
}

pub type NormalizedCharm = BTreeMap<usize, Data>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedTransaction {
    /// Input UTXO list. **May** theoretically be empty.
    /// **Must** be in the order of the hosting transaction's inputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ins: Option<Vec<UtxoId>>,
    /// Reference UTXO list. **May** be empty.
    pub refs: BTreeSet<UtxoId>,
    /// When proving correctness of a spell, we can't know the transaction ID yet.
    /// We only know the index of each output charm.
    /// **Must** be in the order of the hosting transaction's outputs.
    pub outs: Vec<NormalizedCharm>,
}

impl NormalizedTransaction {
    pub fn pre_req_txids(&self) -> BTreeSet<TxId> {
        let Some(ins) = &self.ins else { unreachable!() };
        ins.iter()
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
    /// Maps all `App`s in the transaction to (potentially empty) data.
    pub app_public_inputs: BTreeMap<App, Data>,
}

impl NormalizedSpell {
    pub fn well_formed(
        &self,
        pre_req_spell_proofs: &BTreeMap<TxId, (NormalizedSpell, Box<dyn SpellProof>)>,
    ) -> bool {
        if self.version != CURRENT_VERSION {
            return false;
        }
        let created_by_pre_req_spells = |utxo_id: &UtxoId| -> bool {
            pre_req_spell_proofs
                .get(&utxo_id.0)
                .and_then(|(pre_req_spell, _)| pre_req_spell.tx.outs.get(utxo_id.1 as usize))
                .is_some()
        };
        if self.tx.ins.is_none() {
            return false;
        }
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
        let Some(tx_ins) = &self.tx.ins else {
            unreachable!()
        };
        if !tx_ins.iter().all(created_by_pre_req_spells)
            || !self.tx.refs.iter().all(created_by_pre_req_spells)
        {
            return false;
        }
        true
    }

    pub fn apps(&self) -> Vec<App> {
        self.app_public_inputs.keys().cloned().collect()
    }

    pub fn to_tx(
        &self,
        pre_req_spell_proofs: &BTreeMap<TxId, (NormalizedSpell, Box<dyn SpellProof>)>,
    ) -> Transaction {
        let from_utxo_id = |utxo_id: &UtxoId| -> (UtxoId, Charm) {
            let pre_req_spell = &pre_req_spell_proofs[&utxo_id.0].0;
            (
                utxo_id.clone(),
                pre_req_spell.to_charm(&pre_req_spell.tx.outs[utxo_id.1 as usize]),
            )
        };

        let from_normalized_charm = |n_charm: &NormalizedCharm| -> Charm { self.to_charm(n_charm) };

        let Some(tx_ins) = &self.tx.ins else {
            unreachable!()
        };
        Transaction {
            ins: tx_ins.iter().map(from_utxo_id).collect(),
            refs: self.tx.refs.iter().map(from_utxo_id).collect(),
            outs: self.tx.outs.iter().map(from_normalized_charm).collect(),
        }
    }

    pub fn is_correct(
        &self,
        pre_req_spell_proofs: &BTreeMap<TxId, (NormalizedSpell, Box<dyn SpellProof>)>,
        app_contract_proofs: &Vec<(App, Box<dyn AppContractProof>)>,
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

        let apps = self.apps();
        if apps.len() != app_contract_proofs.len() {
            return false;
        }
        if !apps
            .iter()
            .zip(app_contract_proofs)
            .all(|(app0, (app, proof))| {
                app == app0
                    && proof.verify(
                        app,
                        &self.to_tx(pre_req_spell_proofs),
                        &self.app_public_inputs[app],
                    )
            })
        {
            return false;
        }

        true
    }

    fn to_charm(&self, n_charm: &NormalizedCharm) -> Charm {
        let apps = self.apps();
        n_charm
            .iter()
            .map(|(&i, data)| (apps[i].clone(), data.clone()))
            .collect()
    }
}

pub trait SpellProof {
    /// Verify the proof that the spell is correct.
    fn verify(&self, n_spell: &NormalizedSpell) -> bool;
}

pub trait AppContractProof {
    /// Verify the proof that the app contract is satisfied by the transaction and public input.
    fn verify(&self, app: &App, tx: &Transaction, x: &Data) -> bool;
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
