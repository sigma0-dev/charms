pub mod bin;
pub mod tx;
pub mod v0;

use crate::tx::extract_spell;
use bitcoin::hashes::Hash;
use charms_data::{App, Charm, Data, Transaction, TxId, UtxoId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const V0: u32 = 0u32;
pub const CURRENT_VERSION: u32 = V0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellProverInput {
    pub self_spell_vk: String,
    pub prev_txs: Vec<bitcoin::Transaction>,
    pub spell: NormalizedSpell,
    /// indices of apps in the spell that have contract proofs
    pub app_contract_proofs: BTreeSet<usize>, // proofs are provided in input stream data
}

pub type NormalizedCharm = BTreeMap<usize, Data>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    pub fn prev_txids(&self) -> BTreeSet<TxId> {
        let Some(ins) = &self.ins else { unreachable!() };
        ins.iter()
            .chain(self.refs.iter())
            .map(|utxo_id| utxo_id.0)
            .collect()
    }
}

pub type Proof = Box<[u8]>;

/// Can be committed as public input.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedSpell {
    pub version: u32,
    pub tx: NormalizedTransaction,
    /// Maps all `App`s in the transaction to (potentially empty) data.
    pub app_public_inputs: BTreeMap<App, Data>,
}

impl NormalizedSpell {
    pub fn well_formed(
        &self,
        prev_spells: &BTreeMap<TxId, (Option<NormalizedSpell>, usize)>,
    ) -> bool {
        if self.version != CURRENT_VERSION {
            return false;
        }
        let created_by_prev_spells = |utxo_id: &UtxoId| -> bool {
            prev_spells
                .get(&utxo_id.0)
                .and_then(|(_, num_outs)| Some(utxo_id.1 as usize <= *num_outs))
                == Some(true)
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
        if !tx_ins.iter().all(created_by_prev_spells)
            || !self.tx.refs.iter().all(created_by_prev_spells)
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
        prev_spells: &BTreeMap<TxId, (Option<NormalizedSpell>, usize)>,
    ) -> Transaction {
        let from_utxo_id = |utxo_id: &UtxoId| -> (UtxoId, Charm) {
            let (prev_spell_opt, _) = &prev_spells[&utxo_id.0];
            let charm = prev_spell_opt
                .as_ref()
                .map(|prev_spell| prev_spell.to_charm(&prev_spell.tx.outs[utxo_id.1 as usize]))
                .unwrap_or_default();
            (utxo_id.clone(), charm)
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
        prev_txs: &Vec<bitcoin::Transaction>,
        app_contract_proofs: &Vec<(App, Box<dyn AppContractProof>)>,
        spell_vk: &String,
    ) -> bool {
        let prev_spells = prev_spells(prev_txs, spell_vk);
        if !self.well_formed(&prev_spells) {
            eprintln!("not well formed");
            return false;
        }
        let prev_txids = self.tx.prev_txids();
        if prev_txids.len() != prev_spells.len() {
            eprintln!("prev_txids.len() != prev_spell_proofs.len()");
            return false;
        }

        let apps = self.apps();
        if apps.len() != app_contract_proofs.len() {
            eprintln!("apps.len() != app_contract_proofs.len()");
            return false;
        }
        if !apps
            .iter()
            .zip(app_contract_proofs)
            .all(|(app0, (app, proof))| {
                app == app0
                    && proof.verify(app, &self.to_tx(&prev_spells), &self.app_public_inputs[app])
            })
        {
            eprintln!("app_contract_proofs verification failed");
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

pub fn prev_spells(
    prev_txs: &Vec<bitcoin::Transaction>,
    spell_vk: &str,
) -> BTreeMap<TxId, (Option<NormalizedSpell>, usize)> {
    prev_txs
        .iter()
        .map(|tx| {
            (
                TxId(tx.compute_txid().to_byte_array()),
                (
                    extract_spell(tx, spell_vk).ok().map(|(spell, _)| spell),
                    tx.output.len(),
                ),
            )
        })
        .collect()
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
