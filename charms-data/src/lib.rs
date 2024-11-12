#![no_std]
extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use anyhow::{ensure, Error, Result};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::collections::BTreeMap;
use core::fmt;
use serde::{
    de,
    de::{MapAccess, Visitor},
    ser,
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
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

// UTXO as presented to the validation predicate.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct UtxoId(pub TxId, pub u32);

impl UtxoId {
    pub fn to_bytes(&self) -> [u8; 36] {
        let mut bytes = [0u8; 36];
        bytes[..32].copy_from_slice(&self.0); // Copy TxId
        bytes[32..].copy_from_slice(&self.1.to_le_bytes()); // Copy index as little-endian
        bytes
    }

    pub fn from_bytes(bytes: [u8; 36]) -> Self {
        let mut txid = [0u8; 32];
        txid.copy_from_slice(&bytes[..32]);
        let index = u32::from_le_bytes(bytes[32..].try_into().unwrap());
        UtxoId(txid, index)
    }
}

impl Serialize for UtxoId {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&format!("{}:{}", hex::encode(self.0), self.1))
        } else {
            serializer.serialize_bytes(self.to_bytes().as_ref())
        }
    }
}

impl<'de> Deserialize<'de> for UtxoId {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct UtxoIdVisitor;

        impl<'de> Visitor<'de> for UtxoIdVisitor {
            type Value = UtxoId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string in format 'txid_hex:index' or a tuple (TxId, u32)")
            }

            // Handle human-readable format ("txid_hex:index")
            fn visit_str<E>(self, value: &str) -> Result<UtxoId, E>
            where
                E: de::Error,
            {
                // Split at ':'
                let parts: Vec<&str> = value.split(':').collect();
                if parts.len() != 2 {
                    return Err(E::custom("expected format: txid_hex:index"));
                }

                // Decode txid hex
                let txid_bytes = hex::decode(parts[0])
                    .map_err(|e| E::custom(format!("invalid txid hex: {}", e)))?;

                // Convert tx_bytes into TxId array
                let txid = txid_bytes
                    .try_into()
                    .map_err(|e| E::custom(format!("invalid txid bytes: {:?}", e)))?;

                // Parse index
                let index = parts[1]
                    .parse::<u32>()
                    .map_err(|e| E::custom(format!("invalid index: {}", e)))?;

                Ok(UtxoId(txid, index))
            }

            // Handle non-human-readable byte format [u8; 36]
            fn visit_bytes<E>(self, v: &[u8]) -> core::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(UtxoId::from_bytes(v.try_into().map_err(|e| {
                    E::custom(format!("invalid utxo_id bytes: {}", e))
                })?))
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(UtxoIdVisitor)
        } else {
            deserializer.deserialize_bytes(UtxoIdVisitor)
        }
    }
}

impl TryFrom<&[u8]> for UtxoId {
    type Error = Error;

    fn try_from(bs: &[u8]) -> Result<Self, Self::Error> {
        ensure!(bs.len() == 36);

        let txid = bs[0..32].try_into().unwrap();
        let vout = u32::from_le_bytes(bs[32..36].try_into().unwrap());

        Ok(Self(txid, vout))
    }
}

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct AppId {
    pub tag: Vec<u8>,
    pub id: UtxoId,
    pub vk_hash: VkHash,
}

impl Serialize for AppId {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let tag_s = String::from_utf8(self.tag.clone())
                .map_err(|e| ser::Error::custom(format!("invalid utf-8 in tag: {}", e)))?;
            let id_s = format!("{}:{}", hex::encode(self.id.0), self.id.1);
            let vk_hash = hex::encode(&self.vk_hash.0);
            serializer.serialize_str(&format!("{}/{}/{}", tag_s, id_s, vk_hash))
        } else {
            let mut s = serializer.serialize_struct("AppId", 3)?;
            s.serialize_field("tag", &self.tag)?;
            s.serialize_field("id", &self.id)?;
            s.serialize_field("vk_hash", &self.vk_hash)?;
            s.end()
        }
    }
}

impl<'de> Deserialize<'de> for AppId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AppIdVisitor;

        impl<'de> Visitor<'de> for AppIdVisitor {
            type Value = AppId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string in format 'tag_hex/txid_hex:index_int/vk_hash_hex' or a struct with tag, utxo_id and vk_hash fields")
            }

            // Handle human-readable format ("tag_hex/vk_hash_hex")
            fn visit_str<E>(self, value: &str) -> Result<AppId, E>
            where
                E: de::Error,
            {
                // Split the string at '/'
                let parts: Vec<&str> = value.split('/').collect();
                if parts.len() != 3 {
                    return Err(E::custom(
                        "expected format: tag_hex/txid_hex:index_int/vk_hash_hex",
                    ));
                }

                // Decode the hex strings
                let tag = parts[0].as_bytes().to_vec();

                let id = {
                    let utxo_id_parts: Vec<&str> = parts[1].split(':').collect();
                    if utxo_id_parts.len() != 2 {
                        return Err(E::custom("expected utxo_id format: txid_hex:index"));
                    }
                    let txid_bytes = hex::decode(utxo_id_parts[0])
                        .map_err(|e| E::custom(format!("invalid txid hex: {}", e)))?;

                    let txid = txid_bytes
                        .try_into()
                        .map_err(|e| E::custom(format!("invalid txid bytes: {:?}", e)))?;

                    let index = utxo_id_parts[1]
                        .parse::<u32>()
                        .map_err(|e| E::custom(format!("invalid index: {}", e)))?;

                    UtxoId(txid, index)
                };

                let vk_hash_bytes = hex::decode(parts[2])
                    .map_err(|e| E::custom(format!("invalid vk_hash hex: {}", e)))?;

                // Convert vk_hash bytes to VkHash
                let vk_hash = VkHash(
                    vk_hash_bytes
                        .try_into()
                        .map_err(|e| E::custom(format!("invalid vk_hash: {:?}", e)))?,
                );

                Ok(AppId { tag, id, vk_hash })
            }

            // Handle non-human-readable struct format
            fn visit_map<V>(self, mut map: V) -> Result<AppId, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut tag = None;
                let mut id = None;
                let mut vk_hash = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "tag" => {
                            if tag.is_some() {
                                return Err(de::Error::duplicate_field("tag"));
                            }
                            tag = Some(map.next_value()?);
                        }
                        "id" => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        "vk_hash" => {
                            if vk_hash.is_some() {
                                return Err(de::Error::duplicate_field("vk_hash"));
                            }
                            vk_hash = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(&key, &["tag", "vk_hash"]));
                        }
                    }
                }

                let tag = tag.ok_or_else(|| de::Error::missing_field("tag"))?;
                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                let vk_hash = vk_hash.ok_or_else(|| de::Error::missing_field("vk_hash"))?;

                Ok(AppId { tag, id, vk_hash })
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(AppIdVisitor)
        } else {
            deserializer.deserialize_map(AppIdVisitor)
        }
    }
}

pub type AppState = Data;

type TxId = [u8; 32];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WitnessData {
    pub proof: Data,
    pub public_input: Data,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct VkHash(pub [u8; 32]);

pub type VK = Data;

#[derive(
    Clone,
    Debug,
    Default,
    Hash,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Serialize,
    Deserialize,
    CanonicalSerialize,
    CanonicalDeserialize,
)]
pub struct Data(pub Vec<u8>);

impl Data {
    pub fn empty() -> Self {
        Self(vec![])
    }
}

impl TryFrom<&Data> for u64 {
    type Error = Error;

    fn try_from(value: &Data) -> Result<Self> {
        ensure!(value.0.len() <= 8);
        Ok(u64::from_le_bytes(value.clone().0.try_into().unwrap()))
    }
}

impl From<u64> for Data {
    fn from(value: u64) -> Self {
        Self(value.to_le_bytes().to_vec())
    }
}

pub const TOKEN: &[u8] = b"token";
pub const NFT: &[u8] = b"nft";

pub fn token_amounts_balanced(app_id: &AppId, tx: &Transaction) -> Option<bool> {
    match (
        sum_token_amount(app_id, &tx.ins),
        sum_token_amount(app_id, &tx.outs),
    ) {
        (Ok(amount_in), Ok(amount_out)) => Some(amount_in == amount_out),
        (..) => None,
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

#[cfg(test)]
mod tests {
    use super::*;

    pub fn zk_meme_token_policy(app_id: &AppId, tx: &Transaction, x: &Data, w: &Data) {
        assert_eq!(app_id.tag, TOKEN);

        // is_meme_token_creator is a function that checks that
        // the spender is the creator of this meme token.
        // In our policy, the token creator can mint and burn tokens at will.
        assert!(token_amounts_balanced(&app_id, &tx).unwrap() || is_meme_token_creator(x, w));
    }

    fn is_meme_token_creator(_x: &Data, _w: &Data) -> bool {
        // TODO check the signature in the witness
        false
    }

    #[test]
    fn test_zk_meme_token_validator() {
        let token_app_id = AppId {
            tag: TOKEN.to_vec(),
            id: Default::default(),
            vk_hash: Default::default(),
        };

        let ins = vec![Utxo {
            id: Some(UtxoId::default()),
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

        let empty = Data::empty();
        zk_meme_token_policy(&token_app_id, &tx, &empty, &empty); // pass if no panic
    }
}
