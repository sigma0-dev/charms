use app_utxo_data::{Data, StateKey, Transaction, Utxo, UtxoId, TOKEN};

pub fn main() {
    let (prove, verify) = guest::build_zk_meme_token_policy();

    let token_state_key = StateKey {
        tag: TOKEN.to_vec(),
        prefix: vec![],
        vk_hash: [0u8; 32],
    };

    let ins = vec![Utxo {
        id: Some(UtxoId::empty()),
        amount: 1,
        state: vec![(token_state_key.clone(), Data::new(Box::new(1u64.to_le_bytes())))],
    }];
    let outs = vec![Utxo {
        id: None,
        amount: 1,
        state: vec![(token_state_key.clone(), Data::new(Box::new(1u64.to_le_bytes())))],
    }];

    let tx = Transaction {
        ins,
        refs: vec![],
        outs,
    };

    let (output, proof) = prove(token_state_key, tx, Data::empty(), Data::empty());
    let is_valid = verify(proof);

    dbg!(output);
    dbg!(is_valid);
}
