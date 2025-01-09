use crate::{
    cli::WalletListParams,
    spell::{str_index, KeyedCharms, Spell},
    tx,
};
use anyhow::{ensure, Result};
use bitcoin::{hashes::Hash, Transaction};
use charms_data::{App, TxId, UtxoId};
use ciborium::Value;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    process::{Command, Stdio},
};

#[derive(Debug, Deserialize)]
struct BListUnspentItem {
    txid: String,
    vout: u32,
    amount: f64,
    solvable: bool,
}

#[derive(Debug, Serialize)]
struct OutputWithCharms {
    sats: u64,
    charms: BTreeMap<String, Value>,
}

type ParsedCharms = BTreeMap<App, Value>;

#[derive(Debug, Serialize)]
struct AppsAndCharmsOutputs {
    apps: BTreeMap<String, App>,
    outputs: BTreeMap<UtxoId, OutputWithCharms>,
}

pub fn list(params: WalletListParams) -> Result<()> {
    let b_cli = Command::new("bitcoin-cli")
        .args(&["listunspent"])
        .stdout(Stdio::piped())
        .spawn()?;
    let output = b_cli.wait_with_output()?;
    let b_list_unspent: Vec<BListUnspentItem> = serde_json::from_slice(&output.stdout)?;

    let unspent_charms_outputs = outputs_with_charms(b_list_unspent)?;

    match params.json {
        true => Ok(serde_json::to_writer_pretty(
            std::io::stdout(),
            &unspent_charms_outputs,
        )?),
        false => Ok(serde_yaml::to_writer(
            std::io::stdout(),
            &unspent_charms_outputs,
        )?),
    }
}

fn outputs_with_charms(b_list_unspent: Vec<BListUnspentItem>) -> Result<AppsAndCharmsOutputs> {
    let txid_set = b_list_unspent
        .iter()
        .map(|item| item.txid.clone())
        .collect::<BTreeSet<_>>();
    let spells = txs_with_spells(txid_set.into_iter())?;
    let utxos_with_charms: BTreeMap<UtxoId, (u64, ParsedCharms)> =
        utxos_with_charms(spells, b_list_unspent);
    let apps = collect_apps(&utxos_with_charms);

    Ok(AppsAndCharmsOutputs {
        apps: enumerate_apps(&apps),
        outputs: pretty_outputs(utxos_with_charms, &apps),
    })
}

fn txs_with_spells(txid_iter: impl Iterator<Item = String>) -> Result<BTreeMap<TxId, Spell>> {
    let txs_with_spells = txid_iter
        .map(|txid| {
            let tx: Transaction = get_tx(&txid)?;
            Ok(tx)
        })
        .map(|tx_result: Result<Transaction>| {
            let tx = tx_result?;
            let spell_opt = tx::spell(&tx);
            Ok(spell_opt.map(|spell| (TxId(tx.compute_txid().to_byte_array()), spell)))
        })
        .filter_map(|tx_result| match tx_result {
            Ok(Some(v)) => Some(Ok(v)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        })
        .collect::<Result<_>>()?;

    Ok(txs_with_spells)
}

fn utxos_with_charms(
    spells: BTreeMap<TxId, Spell>,
    b_list_unspent: Vec<BListUnspentItem>,
) -> BTreeMap<UtxoId, (u64, ParsedCharms)> {
    b_list_unspent
        .iter()
        .filter(|item| item.solvable)
        .filter_map(|item| {
            let txid = TxId::from_str(&item.txid).expect("txids from bitcoin-cli should be valid");
            let i = item.vout;
            let sats = (item.amount * 100000000f64) as u64;
            spells
                .get(&txid)
                .and_then(|spell| spell.outs.get(i as usize).map(|utxo| (utxo, &spell.apps)))
                .and_then(|(utxo, apps)| {
                    utxo.charms
                        .as_ref()
                        .map(|keyed_charms| (keyed_charms, apps))
                })
                .map(|(keyed_charms, apps)| {
                    (UtxoId(txid, i), (sats, parsed_charms(keyed_charms, apps)))
                })
        })
        .collect()
}

fn parsed_charms(keyed_charms: &KeyedCharms, apps: &BTreeMap<String, App>) -> ParsedCharms {
    keyed_charms
        .iter()
        .filter_map(|(k, v)| apps.get(k).map(|app| (app.clone(), v.clone())))
        .collect()
}

fn collect_apps(
    strings_of_charms: &BTreeMap<UtxoId, (u64, ParsedCharms)>,
) -> BTreeMap<App, String> {
    let apps: BTreeSet<App> = strings_of_charms
        .iter()
        .flat_map(|(_utxo, (_sats, charms))| charms.keys())
        .cloned()
        .collect();
    apps.into_iter()
        .zip(0..)
        .map(|(app, i)| (app, str_index(&i)))
        .collect()
}

fn enumerate_apps(apps: &BTreeMap<App, String>) -> BTreeMap<String, App> {
    apps.iter()
        .map(|(app, i)| (i.clone(), app.clone()))
        .collect()
}

fn pretty_outputs(
    utxos_with_charms: BTreeMap<UtxoId, (u64, ParsedCharms)>,
    apps: &BTreeMap<App, String>,
) -> BTreeMap<UtxoId, OutputWithCharms> {
    utxos_with_charms
        .into_iter()
        .map(|(utxo_id, (sats, charms))| {
            let charms = charms
                .iter()
                .map(|(app, value)| (apps[app].clone(), value.clone()))
                .collect();
            (utxo_id.clone(), OutputWithCharms { sats, charms })
        })
        .collect()
}

fn get_tx(txid: &str) -> Result<Transaction> {
    let b_cli = Command::new("bitcoin-cli")
        .args(&["getrawtransaction", txid])
        .stdout(Stdio::piped())
        .spawn()?;
    let output = b_cli.wait_with_output()?;
    ensure!(
        output.status.success(),
        "bitcoin-cli getrawtransaction failed"
    );
    let tx_hex = String::from_utf8(output.stdout)?;
    let tx_hex = tx_hex.trim();
    let tx = bitcoin::consensus::encode::deserialize_hex(&(tx_hex))?;
    Ok(tx)
}
