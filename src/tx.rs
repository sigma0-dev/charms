use crate::{
    script::{control_block, data_script, taproot_spend_info},
    spell::Spell,
    SPELL_VK,
};
use bitcoin::{
    self,
    absolute::LockTime,
    consensus::encode::deserialize_hex,
    key::Secp256k1,
    secp256k1::{schnorr, Keypair, Message},
    sighash::{Prevouts, SighashCache},
    taproot,
    taproot::LeafVersion,
    transaction::Version,
    Amount, FeeRate, OutPoint, ScriptBuf, TapLeafHash, TapSighashType, Transaction, TxIn, TxOut,
    Txid, Weight, Witness, XOnlyPublicKey,
};
use charms_spell_checker::{NormalizedSpell, Proof};
use rand::thread_rng;
use std::collections::BTreeMap;

/// `add_spell` adds `spell` to `tx`:
/// 1. it builds `commit_tx` transaction which creates a *committed spell* Tapscript output
/// 2. then appends an input spending the *committed spell* to `tx`, and adds a witness for it.
///
/// `fee_rate` is used to compute the amount of sats necessary to fund the commit and spell
/// transactions.
///
/// Return `[commit_tx, tx]`.
///
/// Both `commit_tx` and `tx` need to be signed.
pub fn add_spell(
    tx: Transaction,
    spell_data: &[u8],
    funding_out_point: OutPoint,
    funding_output_value: Amount,
    change_script_pubkey: ScriptBuf,
    fee_rate: FeeRate,
    prev_txs: &BTreeMap<Txid, Transaction>,
) -> [Transaction; 2] {
    let secp256k1 = Secp256k1::new();
    let keypair = Keypair::new(&secp256k1, &mut thread_rng());
    let (public_key, _) = XOnlyPublicKey::from_keypair(&keypair);

    let script = data_script(public_key, &spell_data);

    let commit_tx = create_commit_tx(
        funding_out_point,
        funding_output_value,
        public_key,
        &script,
        fee_rate,
    );
    let commit_txout = &commit_tx.output[0];

    let tx_amount_in = tx_total_amount_in(prev_txs, &tx);
    let change_amount = compute_change_amount(
        fee_rate,
        script.len(),
        &tx,
        tx_amount_in + commit_txout.value,
    );

    let mut tx = tx;
    modify_tx(
        &mut tx,
        commit_tx.compute_txid(),
        change_script_pubkey,
        change_amount,
    );
    let spell_input = tx.input.len() - 1;

    let signature = create_tx_signature(keypair, &mut tx, spell_input, &commit_txout, &script);

    append_witness_data(
        &mut tx.input[spell_input].witness,
        public_key,
        script,
        signature,
    );

    [commit_tx, tx]
}

/// fee covering only the marginal cost of spending the committed spell output.
fn compute_change_amount(
    fee_rate: FeeRate,
    script_len: usize,
    tx: &Transaction,
    total_amount_in: Amount,
) -> Amount {
    // script input: (41 * 4) + (L + 99) = 164 + L + 99 = L + 263 wu
    // change output: 42 * 4 = 168 wu
    let added_weight =
        Weight::from_witness_data_size(script_len as u64) + Weight::from_wu(263 + 168);

    let total_tx_weight = tx.weight() + added_weight;
    let fee = fee_rate.fee_wu(total_tx_weight).unwrap();

    let tx_amount_out = tx.output.iter().map(|tx_out| tx_out.value).sum::<Amount>();

    total_amount_in - tx_amount_out - fee
}

fn create_commit_tx(
    funding_out_point: OutPoint,
    funding_output_value: Amount,
    public_key: XOnlyPublicKey,
    script: &ScriptBuf,
    fee_rate: FeeRate,
) -> Transaction {
    let fee = fee_rate.fee_vb(111).unwrap(); // tx is 111 vbytes when spending a Taproot output

    let commit_tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: funding_out_point,
            script_sig: Default::default(),
            sequence: Default::default(),
            witness: Default::default(),
        }],
        output: vec![TxOut {
            value: funding_output_value - fee,
            script_pubkey: ScriptBuf::new_p2tr_tweaked(
                taproot_spend_info(public_key, script.clone()).output_key(),
            ),
        }],
    };

    commit_tx
}

fn modify_tx(
    tx: &mut Transaction,
    commit_txid: Txid,
    change_script_pubkey: ScriptBuf,
    change_amount: Amount,
) {
    tx.input.push(TxIn {
        previous_output: OutPoint {
            txid: commit_txid,
            vout: 0,
        },
        script_sig: Default::default(),
        sequence: Default::default(),
        witness: Witness::new(),
    });
    tx.output.push(TxOut {
        value: change_amount,
        script_pubkey: change_script_pubkey,
    });

    if change_amount >= Amount::from_sat(546) {
        // dust limit
    }
}

fn create_tx_signature(
    keypair: Keypair,
    tx: &mut Transaction,
    input_index: usize,
    prev_out: &TxOut,
    script: &ScriptBuf,
) -> schnorr::Signature {
    let mut sighash_cache = SighashCache::new(tx);
    let sighash = sighash_cache
        .taproot_script_spend_signature_hash(
            input_index,
            &Prevouts::One(input_index, prev_out),
            TapLeafHash::from_script(script, LeafVersion::TapScript),
            TapSighashType::AllPlusAnyoneCanPay,
        )
        .unwrap();
    let secp256k1 = Secp256k1::new();
    let signature = secp256k1.sign_schnorr(
        &Message::from_digest_slice(sighash.as_ref())
            .expect("should be cryptographically secure hash"),
        &keypair,
    );

    signature
}

fn append_witness_data(
    witness: &mut Witness,
    public_key: XOnlyPublicKey,
    script: ScriptBuf,
    signature: schnorr::Signature,
) {
    witness.push(
        taproot::Signature {
            signature,
            sighash_type: TapSighashType::AllPlusAnyoneCanPay,
        }
        .to_vec(),
    );
    witness.push(script.clone());
    witness.push(control_block(public_key, script).serialize());
}

pub fn norm_spell_and_proof(tx: &Transaction) -> Option<(NormalizedSpell, Proof)> {
    charms_spell_checker::tx::extract_spell(&tx, SPELL_VK).ok()
}

pub fn spell(tx: &Transaction) -> Option<Spell> {
    match norm_spell_and_proof(tx) {
        Some((norm_spell, _proof)) => Some(Spell::denormalized(&norm_spell)),
        None => None,
    }
}

pub fn txs_by_txid(prev_txs: Vec<String>) -> anyhow::Result<BTreeMap<Txid, Transaction>> {
    prev_txs
        .iter()
        .map(|prev_tx| {
            let prev_tx = deserialize_hex::<Transaction>(prev_tx)?;

            Ok((prev_tx.compute_txid(), prev_tx))
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()
}

pub fn tx_total_amount_in(prev_txs: &BTreeMap<Txid, Transaction>, tx: &Transaction) -> Amount {
    tx.input
        .iter()
        .map(|tx_in| (tx_in.previous_output.txid, tx_in.previous_output.vout))
        .map(|(tx_id, i)| prev_txs[&tx_id].output[i as usize].value)
        .sum::<Amount>()
}

pub fn tx_total_amount_out(tx: &Transaction) -> Amount {
    tx.output.iter().map(|tx_out| tx_out.value).sum::<Amount>()
}
