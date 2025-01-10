use anyhow::{anyhow, ensure, Result};
use ark_std::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use ciborium::Value;
use core::{convert::TryInto, fmt, fmt::Display};
use serde::{
    de,
    de::{DeserializeOwned, SeqAccess, Visitor},
    ser::SerializeTuple,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::cmp::Ordering;

#[macro_export]
macro_rules! check {
    ($condition:expr) => {
        if !$condition {
            eprintln!("condition does not hold: {}", stringify!($condition));
            return false;
        }
    };
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    /// Input UTXOs.
    pub ins: BTreeMap<UtxoId, Charms>,
    /// Reference UTXOs.
    pub refs: BTreeMap<UtxoId, Charms>,
    /// Output charms.
    pub outs: Vec<Charms>,
}

/// Charm is essentially an app-level UTXO that can carry tokens, NFTs, arbitrary app state.
/// Structurally it is a sorted map of `app -> app_state`
pub type Charms = BTreeMap<App, Data>;

#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[derive(Clone, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct UtxoId(pub TxId, pub u32);

impl UtxoId {
    pub fn to_bytes(&self) -> [u8; 36] {
        let mut bytes = [0u8; 36];
        bytes[..32].copy_from_slice(&self.0 .0); // Copy TxId
        bytes[32..].copy_from_slice(&self.1.to_le_bytes()); // Copy index as little-endian
        bytes
    }

    pub fn from_bytes(bytes: [u8; 36]) -> Self {
        let mut txid_bytes = [0u8; 32];
        txid_bytes.copy_from_slice(&bytes[..32]);
        let index = u32::from_le_bytes(bytes[32..].try_into().unwrap());
        UtxoId(TxId(txid_bytes), index)
    }

    pub fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("expected format: txid_hex:index"));
        }

        let txid = TxId::from_str(parts[0])?;

        let index = parts[1]
            .parse::<u32>()
            .map_err(|e| anyhow!("invalid index: {}", e))?;

        Ok(UtxoId(txid, index))
    }

    fn to_string_internal(&self) -> String {
        format!("{}:{}", self.0.to_string(), self.1)
    }
}

impl Display for UtxoId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_string_internal().fmt(f)
    }
}

impl fmt::Debug for UtxoId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UtxoId({})", self.to_string_internal())
    }
}

impl Serialize for UtxoId {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
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
                UtxoId::from_str(value).map_err(E::custom)
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

#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[derive(Clone, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct App {
    pub tag: char,
    pub identity: B32,
    pub vk: B32,
}

impl Display for App {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}/{}", self.tag, self.identity, self.vk)
    }
}

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "App({}/{}/{})", self.tag, self.identity, self.vk)
    }
}

impl Serialize for App {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            let mut s = serializer.serialize_tuple(3)?;
            s.serialize_element(&self.tag)?;
            s.serialize_element(&self.identity)?;
            s.serialize_element(&self.vk)?;
            s.end()
        }
    }
}

impl<'de> Deserialize<'de> for App {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AppVisitor;

        impl<'de> Visitor<'de> for AppVisitor {
            type Value = App;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string in format 'tag_char/identity_hex/vk_hex' or a struct with tag, identity and vk fields")
            }

            // Handle human-readable format ("tag_char/identity_hex/vk_hex")
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // Split the string at '/'
                let parts: Vec<&str> = value.split('/').collect();
                if parts.len() != 3 {
                    return Err(E::custom("expected format: tag_char/identity_hex/vk_hex"));
                }

                // Decode the hex strings
                let tag: char = {
                    let mut chars = parts[0].chars();
                    let Some(tag) = chars.next() else {
                        return Err(E::custom("expected tag"));
                    };
                    let None = chars.next() else {
                        return Err(E::custom("tag must be a single character"));
                    };
                    tag
                };

                let identity = B32::from_str(parts[1]).map_err(E::custom)?;

                let vk = B32::from_str(parts[2]).map_err(E::custom)?;

                Ok(App { tag, identity, vk })
            }

            fn visit_seq<A>(self, mut seq: A) -> core::result::Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let tag = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::missing_field("tag"))?;
                let id = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::missing_field("id"))?;
                let vk_hash = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::missing_field("vk_hash"))?;

                Ok(App {
                    tag,
                    identity: id,
                    vk: vk_hash,
                })
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(AppVisitor)
        } else {
            deserializer.deserialize_tuple(3, AppVisitor)
        }
    }
}

#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct TxId(pub [u8; 32]);

impl TxId {
    pub fn from_str(s: &str) -> Result<Self> {
        ensure!(s.len() == 64, "expected 64 hex characters");
        let bytes = hex::decode(s).map_err(|e| anyhow!("invalid txid hex: {}", e))?;
        let mut txid: [u8; 32] = bytes.try_into().unwrap();
        txid.reverse();
        Ok(TxId(txid))
    }

    fn to_string_internal(&self) -> String {
        let mut txid = self.0;
        txid.reverse();
        hex::encode(&txid)
    }
}

impl Display for TxId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_string_internal().fmt(f)
    }
}

impl fmt::Debug for TxId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TxId({})", self.to_string_internal())
    }
}

impl Serialize for TxId {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for TxId {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TxIdVisitor;

        impl<'de> Visitor<'de> for TxIdVisitor {
            type Value = TxId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string of 64 hex characters or a byte array of 32 bytes")
            }

            // Handle human-readable format ("txid_hex")
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                TxId::from_str(value).map_err(E::custom)
            }

            // Handle non-human-readable byte format [u8; 32]
            fn visit_bytes<E>(self, v: &[u8]) -> core::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(TxId(v.try_into().map_err(|e| {
                    E::custom(format!("invalid txid bytes: {}", e))
                })?))
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(TxIdVisitor)
        } else {
            deserializer.deserialize_bytes(TxIdVisitor)
        }
    }
}

#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct B32(pub [u8; 32]);

impl B32 {
    pub fn from_str(s: &str) -> Result<Self> {
        ensure!(s.len() == 64, "expected 64 hex characters");
        let bytes = hex::decode(s).map_err(|e| anyhow!("invalid hex: {}", e))?;
        let hash: [u8; 32] = bytes.try_into().unwrap();
        Ok(B32(hash))
    }
}

impl AsRef<[u8]> for B32 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Display for B32 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        hex::encode(&self.0).fmt(f)
    }
}

impl fmt::Debug for B32 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VkHash({})", hex::encode(&self.0))
    }
}

#[derive(Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Data(Value);

impl Eq for Data {}

impl Ord for Data {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .partial_cmp(&other.0)
            .expect("Value comparison should have succeeded")
    }
}

impl Data {
    pub fn empty() -> Self {
        Self(Value::Null)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_null()
    }

    pub fn value<T: DeserializeOwned>(&self) -> Result<T> {
        self.0
            .deserialized()
            .map_err(|e| anyhow!("deserialization error: {}", e))
    }

    pub fn byte_repr(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        ciborium::into_writer(&self, &mut buf).expect("serialization should have succeeded");
        buf
    }
}

impl<T> From<&T> for Data
where
    T: Serialize,
{
    fn from(value: &T) -> Self {
        Self(Value::serialized(value).expect("serialization should have succeeded"))
    }
}

impl Default for Data {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Data({})", format!("{:?}", &self.0))
    }
}

pub const TOKEN: char = 't';
pub const NFT: char = 'n';

pub fn token_amounts_balanced(app: &App, tx: &Transaction) -> bool {
    match (
        sum_token_amount(app, tx.ins.values()),
        sum_token_amount(app, tx.outs.iter()),
    ) {
        (Ok(amount_in), Ok(amount_out)) => amount_in == amount_out,
        (..) => false,
    }
}

pub fn nft_state_preserved(app: &App, tx: &Transaction) -> bool {
    let nft_states_in = app_state_multiset(app, tx.ins.values());
    let nft_states_out = app_state_multiset(app, tx.outs.iter());

    nft_states_in == nft_states_out
}

pub fn app_state_multiset<'a>(
    app: &App,
    strands: impl Iterator<Item = &'a Charms>,
) -> BTreeMap<&'a Data, usize> {
    strands
        .filter_map(|strand| strand.get(app))
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

pub fn sum_token_amount<'a>(
    self_app: &App,
    strands: impl Iterator<Item = &'a Charms>,
) -> Result<u64> {
    strands.fold(Ok(0u64), |amount, strand| match strand.get(self_app) {
        Some(state) => Ok(amount? + state.value::<u64>()?),
        None => amount,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::Value;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn doesnt_crash(s in "\\PC*") {
            let _ = TxId::from_str(&s);
        }

        #[test]
        fn txid_roundtrip(txid: TxId) {
            let s = txid.to_string();
            let txid2 = TxId::from_str(&s).unwrap();
            prop_assert_eq!(txid, txid2);
        }

        #[test]
        fn vk_serde_roundtrip(vk: B32) {
            let bytes = postcard::to_stdvec(&vk).unwrap();
            let deserialize_result = postcard::from_bytes::<B32>(&bytes);
            let vk2 = deserialize_result.unwrap();
            prop_assert_eq!(vk, vk2);
        }

        #[test]
        fn app_serde_roundtrip(app: App) {
            let bytes = postcard::to_stdvec(&app).unwrap();
            let deserialize_result = postcard::from_bytes::<App>(&bytes);
            let app2 = deserialize_result.unwrap();
            prop_assert_eq!(app, app2);
        }

        #[test]
        fn utxo_id_serde_roundtrip(utxo_id: UtxoId) {
            let bytes = postcard::to_stdvec(&utxo_id).unwrap();
            let deserialize_result = postcard::from_bytes::<UtxoId>(&bytes);
            let utxo_id2 = deserialize_result.unwrap();
            prop_assert_eq!(utxo_id, utxo_id2);
        }

        #[test]
        fn tx_id_serde_roundtrip(tx_id: TxId) {
            let bytes = postcard::to_stdvec(&tx_id).unwrap();
            let deserialize_result = postcard::from_bytes::<TxId>(&bytes);
            let tx_id2 = deserialize_result.unwrap();
            prop_assert_eq!(tx_id, tx_id2);
        }
    }

    #[test]
    fn minimal_txid() {
        let tx_id_bytes: [u8; 32] = [&[1u8], [0u8; 31].as_ref()].concat().try_into().unwrap();
        let tx_id = TxId(tx_id_bytes);
        let tx_id_str = tx_id.to_string();
        let tx_id_str_expected = "0000000000000000000000000000000000000000000000000000000000000001";
        assert_eq!(tx_id_str, tx_id_str_expected);
    }

    #[test]
    fn data_dbg() {
        let v = 42u64;
        let data: Data = Data::from(&v);
        assert_eq!(format!("{:?}", data), format!("Data({:?})", Value::from(v)));

        let data = Data::empty();
        assert_eq!(format!("{:?}", data), "Data(Null)");

        let vec1: Vec<u64> = vec![];
        let data: Data = Data::from(&vec1);
        assert_eq!(format!("{:?}", data), "Data(Array([]))");
    }

    #[test]
    fn dummy() {}
}
