use crate::tx::extract_and_verify_spell;
use bitcoin::hashes::Hash;
use charms_data::{App, Charms, Data, Transaction, TxId, UtxoId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod tx;

/// Version `0` of the protocol.
pub const V0: u32 = 0u32;
/// Verification key for version `0` of the `charms-spell-checker` binary.
pub const V0_SPELL_VK: &str = "0x00e9398ac819e6dd281f81db3ada3fe5159c3cc40222b5ddb0e7584ed2327c5d";
/// Verification key for version `0` of the `charms-spell-checker` binary.
pub const V1_SPELL_VK: &str = "0x009f38f590ebca4c08c1e97b4064f39e4cd336eea4069669c5f5170a38a1ff97";
/// Version `1` of the protocol.
pub const V1: u32 = 1u32;
/// Version `2` of the protocol.
pub const V2: u32 = 2u32;
/// Current version of the protocol.
pub const CURRENT_VERSION: u32 = V2;

/// Maps the index of the charm's app (in [`NormalizedSpell`].`app_public_inputs`) to the charm's
/// data.
pub type NormalizedCharms = BTreeMap<usize, Data>;

/// Normalized representation of a Charms transaction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedTransaction {
    /// (Optional) input UTXO list. Is None when serialized in the transaction: the transaction
    /// already lists all inputs. **Must** be in the order of the transaction inputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ins: Option<Vec<UtxoId>>,
    /// Reference UTXO list. **May** be empty.
    pub refs: BTreeSet<UtxoId>,
    /// Output charms. **Must** be in the order of the transaction outputs.
    /// When proving correctness of a spell, we can't know the transaction ID yet.
    /// We only know the index of each output charm.
    /// **Must** be in the order of the hosting transaction's outputs.
    /// **Must not** be larger than the number of outputs in the hosting transaction.
    pub outs: Vec<NormalizedCharms>,
}

impl NormalizedTransaction {
    /// Return a sorted set of transaction IDs of the inputs.
    pub fn prev_txids(&self) -> Option<BTreeSet<&TxId>> {
        self.ins
            .as_ref()
            .map(|ins| ins.iter().map(|utxo_id| &utxo_id.0).collect())
    }
}

/// Proof of correctness of a spell.
pub type Proof = Box<[u8]>;

/// Normalized representation of a spell.
/// Can be committed as public input.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedSpell {
    /// Protocol version.
    pub version: u32,
    /// Transaction data.
    pub tx: NormalizedTransaction,
    /// Maps all `App`s in the transaction to (potentially empty) public input data.
    pub app_public_inputs: BTreeMap<App, Data>,
}

/// Extract spells from previous transactions.
pub fn prev_spells(
    prev_txs: &Vec<bitcoin::Transaction>,
    spell_vk: &str,
) -> BTreeMap<TxId, (Option<NormalizedSpell>, usize)> {
    prev_txs
        .iter()
        .map(|tx| {
            let tx_id = TxId(tx.compute_txid().to_byte_array());
            (
                tx_id,
                (
                    extract_and_verify_spell(tx, spell_vk)
                        .map_err(|e| {
                            eprintln!("no correct spell in tx {}: {}", tx_id, e);
                        })
                        .ok(),
                    tx.output.len(),
                ),
            )
        })
        .collect()
}

/// Check if the spell is well-formed.
pub fn well_formed(
    spell: &NormalizedSpell,
    prev_spells: &BTreeMap<TxId, (Option<NormalizedSpell>, usize)>,
) -> bool {
    if spell.version != CURRENT_VERSION {
        eprintln!(
            "spell version {} is not the current version {}",
            spell.version, CURRENT_VERSION
        );
        return false;
    }
    let created_by_prev_spells = |utxo_id: &UtxoId| -> bool {
        prev_spells
            .get(&utxo_id.0)
            .and_then(|(_, num_tx_outs)| Some(utxo_id.1 as usize <= *num_tx_outs))
            == Some(true)
    };
    if !spell
        .tx
        .outs
        .iter()
        .all(|n_charm| n_charm.keys().all(|i| i < &spell.app_public_inputs.len()))
    {
        eprintln!("charm app index higher than app_public_inputs.len()");
        return false;
    }
    // check that UTXOs we're spending or referencing in this tx
    // are created by pre-req transactions
    let Some(tx_ins) = &spell.tx.ins else {
        eprintln!("no tx.ins");
        return false;
    };
    if !tx_ins.iter().all(created_by_prev_spells)
        || !spell.tx.refs.iter().all(created_by_prev_spells)
    {
        eprintln!("input or reference UTXOs are not created by prev transactions");
        return false;
    }
    true
}

/// Return the list of apps in the spell.
pub fn apps(spell: &NormalizedSpell) -> Vec<App> {
    spell.app_public_inputs.keys().cloned().collect()
}

/// Convert normalized spell to [`charms_data::Transaction`].
pub fn to_tx(
    spell: &NormalizedSpell,
    prev_spells: &BTreeMap<TxId, (Option<NormalizedSpell>, usize)>,
) -> Transaction {
    let from_utxo_id = |utxo_id: &UtxoId| -> (UtxoId, Charms) {
        let (prev_spell_opt, _) = &prev_spells[&utxo_id.0];
        let charms = prev_spell_opt
            .as_ref()
            .and_then(|prev_spell| {
                prev_spell
                    .tx
                    .outs
                    .get(utxo_id.1 as usize)
                    .map(|n_charms| charms(prev_spell, n_charms))
            })
            .unwrap_or_default();
        (utxo_id.clone(), charms)
    };

    let from_normalized_charms =
        |n_charms: &NormalizedCharms| -> Charms { charms(spell, n_charms) };

    let Some(tx_ins) = &spell.tx.ins else {
        unreachable!("self.tx.ins MUST be Some at this point");
    };
    Transaction {
        ins: tx_ins.iter().map(from_utxo_id).collect(),
        refs: spell.tx.refs.iter().map(from_utxo_id).collect(),
        outs: spell.tx.outs.iter().map(from_normalized_charms).collect(),
    }
}

/// Return [`charms_data::Charms`] for the given [`NormalizedCharms`].
pub fn charms(spell: &NormalizedSpell, n_charms: &NormalizedCharms) -> Charms {
    let apps = apps(spell);
    n_charms
        .iter()
        .map(|(&i, data)| (apps[i].clone(), data.clone()))
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellProverInput {
    pub self_spell_vk: String,
    pub prev_txs: Vec<bitcoin::Transaction>,
    pub spell: NormalizedSpell,
    /// indices of apps in the spell that have contract proofs
    pub app_contract_proofs: BTreeSet<usize>, // proofs are provided in input stream data
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
