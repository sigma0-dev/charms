use crate::{
    app,
    cli::{SpellProveParams, SpellRenderParams},
    spell,
    spell::Spell,
    tx::{add_spell, txs_by_txid},
    utils, SPELL_VK,
};
use anyhow::{anyhow, ensure, Result};
use bitcoin::{
    consensus::encode::{deserialize_hex, serialize_hex},
    hashes::Hash,
    Amount, FeeRate, OutPoint, Transaction, Txid,
};
use charms_data::{util, TxId, UtxoId, B32};
use charms_spell_checker::NormalizedSpell;
use std::{collections::BTreeMap, path::PathBuf, str::FromStr};

pub fn prove(
    SpellProveParams {
        spell,
        tx,
        prev_txs,
        app_bins,
        funding_utxo_id,
        funding_utxo_value,
        change_address,
        fee_rate,
    }: SpellProveParams,
) -> Result<()> {
    utils::logger::setup_logger();

    // Parse funding UTXO early: to fail fast
    let funding_utxo = crate::cli::tx::parse_outpoint(&funding_utxo_id)?;

    ensure!(fee_rate >= 1.0, "fee rate must be >= 1.0");

    let spell: Spell = serde_yaml::from_slice(&std::fs::read(spell)?)?;

    let tx = deserialize_hex::<Transaction>(&tx)?;
    let prev_txs = txs_by_txid(prev_txs)?;
    ensure!(tx
        .input
        .iter()
        .all(|input| prev_txs.contains_key(&input.previous_output.txid)));

    let transactions = do_prove(
        spell,
        tx,
        app_bins,
        prev_txs,
        funding_utxo,
        funding_utxo_value,
        change_address,
        fee_rate,
    )?;

    // Convert transactions to hex and create JSON array
    let hex_txs: Vec<String> = transactions.iter().map(|tx| serialize_hex(tx)).collect();

    // Print JSON array of transaction hexes
    println!("{}", serde_json::to_string(&hex_txs)?);

    Ok(())
}

pub fn do_prove(
    spell: Spell,
    tx: Transaction,
    app_bins: Vec<PathBuf>,
    prev_txs: BTreeMap<Txid, Transaction>,
    funding_utxo: OutPoint,
    funding_utxo_value: u64,
    change_address: String,
    fee_rate: f64,
) -> Result<[Transaction; 2]> {
    let (mut norm_spell, app_private_inputs) = spell.normalized()?;
    align_spell_to_tx(&mut norm_spell, &tx)?;

    let app_prover = app::Prover::new();

    let binaries = app_bins
        .iter()
        .map(|path| {
            let binary = std::fs::read(path)?;
            let vk_hash = app_prover.vk(&binary);
            Ok((B32(vk_hash), binary))
        })
        .collect::<Result<_>>()?;

    let (norm_spell, proof) = spell::prove(
        norm_spell,
        &binaries,
        app_private_inputs,
        prev_txs.values().cloned().collect(),
        SPELL_VK,
    )?;

    // Serialize spell into CBOR
    let spell_data = util::write(&(&norm_spell, &proof))?;

    // Parse amount
    let funding_utxo_value = Amount::from_sat(funding_utxo_value);

    // Parse change address into ScriptPubkey
    let change_script_pubkey = bitcoin::Address::from_str(&change_address)?
        .assume_checked()
        .script_pubkey();

    // Parse fee rate
    let fee_rate = FeeRate::from_sat_per_kwu((fee_rate * 250.0) as u64);

    // Call the add_spell function
    let transactions = add_spell(
        tx,
        &spell_data,
        funding_utxo,
        funding_utxo_value,
        change_script_pubkey,
        fee_rate,
        &prev_txs,
    );
    Ok(transactions)
}

fn align_spell_to_tx(norm_spell: &mut NormalizedSpell, tx: &Transaction) -> Result<()> {
    let spell_ins = norm_spell.tx.ins.as_ref().ok_or(anyhow!("no inputs"))?;

    ensure!(
        spell_ins.len() <= tx.input.len(),
        "spell inputs exceed transaction inputs"
    );
    ensure!(
        norm_spell.tx.outs.len() <= tx.output.len(),
        "spell outputs exceed transaction outputs"
    );

    for i in 0..spell_ins.len() {
        let utxo_id = &spell_ins[i];
        let out_point = tx.input[i].previous_output;
        ensure!(
            utxo_id.0 == TxId(out_point.txid.to_byte_array()),
            "input {} txid mismatch: {} != {}",
            i,
            utxo_id.0,
            out_point.txid
        );
        ensure!(
            utxo_id.1 == out_point.vout,
            "input {} vout mismatch: {} != {}",
            i,
            utxo_id.1,
            out_point.vout
        );
    }

    for i in spell_ins.len()..tx.input.len() {
        let out_point = tx.input[i].previous_output;
        let utxo_id = UtxoId(TxId(out_point.txid.to_byte_array()), out_point.vout);
        norm_spell.tx.ins.get_or_insert_with(Vec::new).push(utxo_id);
    }

    Ok(())
}

pub(crate) fn render(_params: SpellRenderParams) -> Result<()> {
    todo!()
}
