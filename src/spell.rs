use crate::{app, SPELL_CHECKER_BINARY};
use anyhow::{anyhow, ensure, Error};
use charms_data::{App, Data, UtxoId, VkHash};
use ciborium::Value;
use serde::{Deserialize, Serialize};
use sp1_sdk::{HashableKey, ProverClient, SP1Stdin};
use spell_checker::{
    NormalizedCharm, NormalizedSpell, NormalizedTransaction, Proof, SpellProverInput,
};
use std::collections::{BTreeMap, BTreeSet};

/// Charm as represented in a spell.
/// Map of `$TICKER: data`
pub type KeyedCharm = BTreeMap<String, Value>;

/// UTXO as represented in a spell.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Utxo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utxo_id: Option<UtxoId>,
    pub charm: KeyedCharm,
}

/// Defines how spells are represented on the wire,
/// in both human-friendly (JSON/YAML) and machine-friendly (CBOR) formats.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spell {
    pub version: u32,

    pub apps: BTreeMap<String, App>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_inputs: Option<BTreeMap<String, Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_inputs: Option<BTreeMap<String, Value>>,

    pub ins: Vec<Utxo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refs: Option<Vec<Utxo>>,
    pub outs: Vec<Utxo>,

    /// folded proof of all validation predicates plus all pre-requisite spells
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<Box<[u8]>>,
}

impl Spell {
    pub fn new() -> Self {
        Self {
            version: 0,
            apps: BTreeMap::new(),
            public_inputs: None,
            private_inputs: None,
            ins: vec![],
            refs: None,
            outs: vec![],
            proof: None,
        }
    }

    pub fn normalized(&self) -> anyhow::Result<(NormalizedSpell, BTreeMap<App, Data>)> {
        let empty_map = BTreeMap::new();
        let keyed_public_inputs = self.public_inputs.as_ref().unwrap_or(&empty_map);

        let keyed_apps = &self.apps;
        let apps: BTreeSet<App> = keyed_apps.values().cloned().collect();
        let app_to_index: BTreeMap<App, usize> = apps.iter().cloned().zip(0..).collect();
        ensure!(apps.len() == keyed_apps.len(), "duplicate apps");

        let app_public_inputs: BTreeMap<App, Data> = keyed_apps
            .iter()
            .map(|(k, app)| {
                (
                    app.clone(),
                    keyed_public_inputs
                        .get(k)
                        .map(|v| Data::from(v))
                        .unwrap_or_default(),
                )
            })
            .collect();

        let ins: Vec<UtxoId> = self
            .ins
            .iter()
            .map(|utxo| utxo.utxo_id.clone().ok_or(anyhow!("missing input utxo_id")))
            .collect::<Result<_, _>>()?;
        ensure!(
            ins.iter().collect::<BTreeSet<_>>().len() == ins.len(),
            "duplicate inputs"
        );
        let ins = Some(ins);

        let empty_vec = vec![];
        let self_refs = self.refs.as_ref().unwrap_or(&empty_vec);
        let refs: BTreeSet<UtxoId> = self_refs
            .iter()
            .map(|utxo| utxo.utxo_id.clone().ok_or(anyhow!("missing input utxo_id")))
            .collect::<Result<_, _>>()?;
        ensure!(refs.len() == self_refs.len(), "duplicate reference inputs");

        let outs: Vec<NormalizedCharm> = self
            .outs
            .iter()
            .map(|utxo| {
                let charm = utxo
                    .charm
                    .iter()
                    .map(|(k, v)| {
                        let app = keyed_apps.get(k).ok_or(anyhow!("missing app key"))?;
                        let i: usize = *app_to_index
                            .get(app)
                            .expect("app should be in app_to_index");
                        Ok((i, Data::from(v)))
                    })
                    .collect::<Result<NormalizedCharm, Error>>()?;
                Ok(charm)
            })
            .collect::<Result<_, Error>>()?;

        let norm_spell = NormalizedSpell {
            version: self.version,
            tx: NormalizedTransaction { ins, refs, outs },
            app_public_inputs,
        };

        let keyed_private_inputs = self.private_inputs.as_ref().unwrap_or(&empty_map);
        let app_private_inputs = keyed_private_inputs
            .iter()
            .map(|(k, v)| {
                let app = keyed_apps.get(k).ok_or(anyhow!("missing app key"))?;
                Ok((app.clone(), Data::from(v)))
            })
            .collect::<Result<_, Error>>()?;

        Ok((norm_spell, app_private_inputs))
    }
}

pub fn prove(
    norm_spell: NormalizedSpell,
    app_binaries: &BTreeMap<VkHash, Vec<u8>>,
    app_private_inputs: BTreeMap<App, Data>,
    prev_txs: Vec<bitcoin::Transaction>,
    spell_vk: &str,
) -> anyhow::Result<(NormalizedSpell, Proof)> {
    let client = ProverClient::new();
    let (pk, vk) = client.setup(SPELL_CHECKER_BINARY);
    let mut stdin = SP1Stdin::new();

    let prev_spells = spell_checker::prev_spells(&prev_txs, spell_vk);

    let prover_input = SpellProverInput {
        self_spell_vk: vk.bytes32(),
        prev_txs,
        spell: norm_spell.clone(),
        app_contract_proofs: norm_spell
            .app_public_inputs
            .iter()
            .zip(0..)
            .filter_map(|((app, _), i)| (app_binaries.get(&app.vk_hash).map(|_| i as usize)))
            .collect(),
    };
    let input_vec: Vec<u8> = {
        let mut buf = vec![];
        ciborium::into_writer(&prover_input, &mut buf)?;
        buf
    };

    dbg!(input_vec.len());

    stdin.write_vec(input_vec);

    let tx = norm_spell.to_tx(&prev_spells);
    let app_public_inputs = &norm_spell.app_public_inputs;

    app::Prover::new().prove(
        app_binaries,
        tx,
        app_public_inputs,
        app_private_inputs,
        &mut stdin,
    )?;

    let proof = client.prove(&pk, stdin).groth16().run()?;
    let proof = proof.bytes().into_boxed_slice();

    let mut norm_spell = norm_spell;
    norm_spell.tx.ins = None;

    Ok((norm_spell, proof))
}

#[cfg(test)]
mod test {
    use super::*;
    use charms_data::*;

    use proptest::prelude::*;

    use ciborium::Value;
    use hex;

    #[test]
    fn deserialize_keyed_charm() {
        let y = r#"
$TOAD_SUB: 10
$TOAD: 9
"#;

        let charm = serde_yaml::from_str::<KeyedCharm>(y).unwrap();
        dbg!(&charm);

        let utxo_id =
            UtxoId::from_str("f72700ac56bd4dd61f2ccb4acdf21d0b11bb294fc3efa9012b77903932197d2f:2")
                .unwrap();
        let mut buf = vec![];
        ciborium::ser::into_writer(&utxo_id, &mut buf).unwrap();

        let utxo_id_value: Value = ciborium::de::from_reader(buf.as_slice()).unwrap();

        let utxo_id: UtxoId = dbg!(utxo_id_value).deserialized().unwrap();
        dbg!(utxo_id);
    }

    #[test]
    fn empty_postcard() {
        use postcard;

        let value: Vec<u8> = vec![];
        let buf = postcard::to_stdvec(&value).unwrap();
        dbg!(buf.len());
        dbg!(buf);

        let mut cbor_buf = vec![];
        let value: Vec<u8> = vec![];
        ciborium::into_writer(&value, &mut cbor_buf).unwrap();
        dbg!(cbor_buf.len());
        dbg!(cbor_buf);
    }
}
