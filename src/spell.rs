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

    #[test]
    fn deserialize_compact_charm() {
        let y = r#"
$TOAD_SUB: 10
$TOAD: 9
"#;

        let charm = serde_yaml::from_str::<CompactCharm>(y).unwrap();
        dbg!(charm);
    }
}
