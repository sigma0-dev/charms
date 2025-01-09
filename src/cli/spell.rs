use crate::{
    app,
    cli::{SpellProveParams, SpellRenderParams},
    spell,
    spell::Spell,
    tx::add_spell,
    SPELL_VK,
};
use anyhow::{anyhow, ensure, Result};
use bitcoin::{
    consensus::encode::{deserialize_hex, serialize_hex},
    hashes::Hash,
    Amount, FeeRate, Transaction,
};
use charms_data::{TxId, UtxoId, VK};
use charms_spell_checker::NormalizedSpell;
use std::{collections::BTreeMap, str::FromStr};

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
    dbg!(&tx);
    dbg!(&prev_txs);
    dbg!(&app_bins);

    sp1_sdk::utils::setup_logger(); // TODO configure the logger to print to stderr (vs stdout)

    let spell: Spell = serde_yaml::from_slice(&std::fs::read(spell)?)?;

    let tx = deserialize_hex::<Transaction>(&tx)?;

    let (mut norm_spell, app_private_inputs) = spell.normalized()?;
    align_spell_to_tx(&mut norm_spell, &tx)?;

    let prev_txs = prev_txs
        .iter()
        .map(|prev_tx| {
            let prev_tx = deserialize_hex::<Transaction>(prev_tx)?;

            Ok((TxId(prev_tx.compute_txid().to_byte_array()), prev_tx))
        })
        .collect::<Result<BTreeMap<_, _>>>()?
        .into_values()
        .collect();

    let app_prover = app::Prover::new();

    let binaries = app_bins
        .iter()
        .map(|path| {
            let binary = std::fs::read(path)?;
            let vk_hash = app_prover.vk(&binary);
            Ok((VK(vk_hash), binary))
        })
        .collect::<Result<_>>()?;

    let (norm_spell, proof) = spell::prove(
        norm_spell,
        &binaries,
        app_private_inputs,
        prev_txs,
        SPELL_VK,
    )?;

    // ciborium::into_writer(&(&norm_spell, &proof), std::io::stdout())?;

    // Serialize spell into CBOR
    let mut spell_data = vec![];
    ciborium::ser::into_writer(&(&norm_spell, &proof), &mut spell_data)?;

    // Parse funding UTXO
    let funding_utxo = crate::cli::tx::parse_outpoint(&funding_utxo_id)?;

    // Parse amount
    let funding_utxo_value = Amount::from_sat(funding_utxo_value);

    // Parse change address into ScriptPubkey
    let change_script_pubkey = bitcoin::Address::from_str(&change_address)?
        .assume_checked()
        .script_pubkey();

    // Parse fee rate
    let fee_rate = FeeRate::from_sat_per_kwu((fee_rate * 1000.0 / 4.0) as u64);

    // Call the add_spell function
    let transactions = add_spell(
        tx,
        &spell_data,
        funding_utxo,
        funding_utxo_value,
        change_script_pubkey,
        fee_rate,
    );

    // Convert transactions to hex and create JSON array
    let hex_txs: Vec<String> = transactions.iter().map(|tx| serialize_hex(tx)).collect();

    // Print JSON array of transaction hexes
    println!("{}", serde_json::to_string(&hex_txs)?);

    Ok(())
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
