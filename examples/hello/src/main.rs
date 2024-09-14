use app_utxo_data::{AppId, Data, Transaction, Utxo, UtxoId, TOKEN};
use jolt::{Jolt, RV32IJoltVM};
use std::collections::BTreeMap;

pub fn main() {
    let (program, prep) = guest::preprocess_zk_meme_token_policy();

    let token_app_id = AppId {
        tag: TOKEN.to_vec(),
        prefix: vec![],
        vk_hash: [0u8; 32],
    };

    let ins = vec![Utxo {
        id: Some(UtxoId::empty()),
        amount: 1,
        state: BTreeMap::from([(token_app_id.clone(), 1u64.into())]),
    }];
    let outs = vec![Utxo {
        id: None,
        amount: 1,
        state: BTreeMap::from([(token_app_id.clone(), 1u64.into())]),
    }];

    let tx = Transaction {
        ins,
        refs: vec![],
        outs,
    };

    let (output, proof) = guest::prove_zk_meme_token_policy(
        program,
        prep.clone(),
        token_app_id,
        tx,
        Data::empty(),
        Data::empty(),
    );
    let is_valid = RV32IJoltVM::verify(prep, proof.proof, proof.commitments).is_ok();

    dbg!(output);
    dbg!(is_valid);
}
