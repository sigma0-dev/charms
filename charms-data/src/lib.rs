#![no_std]
extern crate alloc;

use alloc::{vec, vec::Vec};
use anyhow::{bail, Result};
use ark_std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub ins: Vec<Utxo>,
    pub refs: Vec<Utxo>,
    pub outs: Vec<Utxo>,
}

/// Charm is essentially an app-level UTXO that can carry tokens, NFTs, arbitrary app state.
/// Structurally it is a sorted map of `app_id -> app_state`
pub type Charm = BTreeMap<AppId, AppState>;

pub type Witness = BTreeMap<AppId, WitnessData>;

pub type VKs = BTreeMap<VkHash, VK>;

// App UTXO as presented to the validation predicate.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Utxo {
    pub id: Option<UtxoId>,
    pub charm: Charm,
}

impl Utxo {
    #[inline]
    pub fn get(&self, key: &AppId) -> Option<&AppState> {
        self.charm.get(key)
    }
}

#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Default, Clone, Debug, Serialize, Deserialize)]
pub struct UtxoId {
    pub txid: TxId,
    pub vout: u32,
}

impl UtxoId {
    pub fn new(txid: [u8; 32], vout: u32) -> Self {
        Self { txid, vout }
    }

    pub fn empty() -> Self {
        Self {
            txid: [0u8; 32],
            vout: 0,
        }
    }
}

#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Default, Clone, Debug, Serialize, Deserialize)]
pub struct AppId {
    pub tag: Vec<u8>,
    pub prefix: Vec<u8>,
    pub vk_hash: VkHash,
}

pub type AppState = Data;

type TxId = [u8; 32];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WitnessData {
    pub proof: Data,
    pub public_input: Data,
}

pub type VkHash = [u8; 32];

pub type VK = Data;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Data(pub Vec<u8>);

impl Data {
    pub fn empty() -> Self {
        Self(vec![])
    }
}

impl TryFrom<&Data> for u64 {
    type Error = anyhow::Error;

    fn try_from(value: &Data) -> Result<Self> {
        if value.0.len() > 8 {
            bail!("Data too long to convert to u64");
        }
        Ok(u64::from_le_bytes(value.clone().0.try_into().unwrap()))
    }
}

impl From<u64> for Data {
    fn from(value: u64) -> Self {
        Self(value.to_le_bytes().to_vec())
    }
}

pub const TOKEN: &[u8] = b"TOKEN";
pub const NFT: &[u8] = b"NFT";

pub fn token_amounts_balanced(app_id: &AppId, tx: &Transaction) -> bool {
    match (
        sum_token_amount(app_id, &tx.ins),
        sum_token_amount(app_id, &tx.outs),
    ) {
        (Ok(amount_in), Ok(amount_out)) => amount_in == amount_out,
        (..) => false,
    }
}

pub fn nft_state_preserved(app_id: &AppId, tx: &Transaction) -> bool {
    let nft_states_in = app_state_multiset(app_id, &tx.ins);
    let nft_states_out = app_state_multiset(app_id, &tx.outs);

    nft_states_in == nft_states_out
}

pub fn app_state_multiset<'a>(
    app_id: &AppId,
    utxos: &'a Vec<Utxo>,
) -> BTreeMap<&'a AppState, usize> {
    utxos
        .iter()
        .filter_map(|utxo| utxo.get(app_id))
        .fold(BTreeMap::new(), |mut r, s| {
            match r.get_mut(&s) {
                Some(count) => *count += 1,
                None => {
                    r.insert(s, 1);
                }
            }
            r
        })
}

pub fn sum_token_amount(self_app_id: &AppId, utxos: &[Utxo]) -> Result<u64> {
    let mut in_amount: u64 = 0;
    for utxo in utxos {
        // We only care about UTXOs that have our token.
        if let Some(state) = utxo.get(self_app_id) {
            let utxo_amount: u64 = state.try_into()?;
            in_amount += utxo_amount;
        }
    }
    Ok(in_amount)
}

mod tests {

    use super::*;

    pub fn zk_meme_token_policy(
        self_app_id: &AppId,
        tx: &Transaction,
        x: &Data,
        w: &Data,
    ) -> Result<()> {
        assert_eq!(self_app_id.tag, TOKEN);

        let in_amount = sum_token_amount(self_app_id, &tx.ins)?;
        let out_amount = sum_token_amount(self_app_id, &tx.outs)?;

        // is_meme_token_creator is a function that checks that
        // the spender is the creator of this meme token.
        // In our policy, the token creator can mint and burn tokens at will.
        assert!(in_amount == out_amount || is_meme_token_creator(x, w)?);

        Ok(())
    }

    fn is_meme_token_creator(_x: &Data, _w: &Data) -> Result<bool> {
        // todo!("check the signature in the witness")
        Ok(false)
    }

    #[test]
    fn test_zk_meme_token_validator() {
        let token_app_id = AppId {
            tag: TOKEN.to_vec(),
            prefix: vec![],
            vk_hash: [0u8; 32],
        };

        let ins = vec![Utxo {
            id: Some(UtxoId::empty()),
            charm: Charm::from([(token_app_id.clone(), 1u64.into())]),
        }];
        let outs = vec![Utxo {
            id: None,
            charm: Charm::from([(token_app_id.clone(), 1u64.into())]),
        }];

        let tx = Transaction {
            ins,
            refs: vec![],
            outs,
        };

        assert!(zk_meme_token_policy(&token_app_id, &tx, &Data::empty(), &Data::empty()).is_ok());
    }
}
