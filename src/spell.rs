use charms_data::{AppId, Data, UtxoId};
use ciborium::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Args {
    pub x: Data,
    pub w: Data,
}

pub trait CompactAppState: Serialize {}

/// Charm as represented in a spell.
/// Map of `$TICKER: <app_state>`
pub type CompactCharm = BTreeMap<String, Value>;

/// UTXO as represented in a spell.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompactUtxo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<UtxoId>,
    pub charm: CompactCharm,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spell {
    pub app_ids: BTreeMap<String, AppId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<BTreeMap<String, Args>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ins: Option<Vec<CompactUtxo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refs: Option<Vec<CompactUtxo>>,
    pub outs: Vec<CompactUtxo>,

    /// folded proof of all validation predicates plus all pre-requisite spells
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<Data>,

    /// mapping of sha256(risc-v-binary) -> tar --xz -cf risc-v-binary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binaries: Option<BTreeMap<String, Data>>,
}

#[cfg(test)]
mod test {
    use crate::spell::CompactCharm;
    use charms_data::{AppId, AppState, Charm, Data, Transaction, Utxo, UtxoId, VkHash, TOKEN};
    use hex;
    use std::str::FromStr;

    #[test]
    fn deserialize_compact_charm() {
        let y = r#"
$TOAD_SUB: 10
$TOAD: 9
"#;

        let charm = serde_yaml::from_str::<CompactCharm>(y).unwrap();
        dbg!(charm);
    }

    #[test]
    fn app_id_postcard() {
        use postcard;

        let app_id_orig = AppId {
            tag: TOKEN.to_vec(),
            id: UtxoId::default(),
            vk_hash: VkHash::default(),
        };

        let tx_orig = Transaction {
            ins: vec![Utxo {
                id: Some(UtxoId::default()),
                charm: Charm::from([(app_id_orig.clone(), 1u64.into())]),
            }],
            refs: vec![],
            outs: vec![Utxo {
                id: None,
                charm: Charm::from([(app_id_orig.clone(), 1u64.into())]),
            }],
        };

        let utxo_orig = Utxo {
            id: Some(UtxoId::default()),
            charm: Charm::from([(app_id_orig.clone(), 1u64.into())]),
        };

        let app_state_orig: AppState = 1u64.into();

        let mut buf = [0u8; 4096];
        let writer: &mut [u8] = &mut buf;

        let writer = postcard::to_io(&app_id_orig, writer).unwrap();
        let writer = postcard::to_io(&tx_orig, writer).unwrap();
        let writer = postcard::to_io(&utxo_orig, writer).unwrap();
        let writer = postcard::to_io(&app_state_orig, writer).unwrap();
        let writer = postcard::to_io(&Data::empty(), writer).unwrap();
        let writer = postcard::to_io(&Data::empty(), writer).unwrap();

        let buf: &mut [u8] = &mut buf;

        let (app_id, buf) = postcard::take_from_bytes::<AppId>(buf).unwrap();
        let (tx, buf) = postcard::take_from_bytes::<Transaction>(buf).unwrap();
        let (utxo, buf) = postcard::take_from_bytes::<Utxo>(buf).unwrap();
        let (app_state, buf) = postcard::take_from_bytes::<AppState>(buf).unwrap();
        let (x, buf) = postcard::take_from_bytes::<Data>(buf).unwrap();
        let (w, buf) = postcard::take_from_bytes::<Data>(buf).unwrap();

        assert_eq!(app_id, app_id_orig);
        assert_eq!(tx, tx_orig);
        assert_eq!(utxo, utxo_orig);
        assert_eq!(app_state, app_state_orig);
        assert_eq!(x, Data::empty());
        assert_eq!(w, Data::empty());

        let hex_bytes =
            hex::decode("f72700ac56bd4dd61f2ccb4acdf21d0b11bb294fc3efa9012b77903932197d2f")
                .unwrap();
        let utxo_id_orig = UtxoId(hex_bytes.try_into().unwrap(), 0);

        let mut buf = [0u8; 100];

        let buf = postcard::to_slice(&utxo_id_orig, &mut buf).unwrap();
        dbg!(buf.len());

        let (utxo_id, buf) = postcard::take_from_bytes::<UtxoId>(buf).unwrap();
        assert_eq!(utxo_id, utxo_id_orig);
    }
}
