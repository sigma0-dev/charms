#![no_main]

use app_utxo_data::{Data, StateKey, Transaction, Utxo, TOKEN};

#[jolt::provable]
fn fib(n: u32) -> u128 {
    let mut a: u128 = 0;
    let mut b: u128 = 1;
    let mut sum: u128;
    for _ in 1..n {
        sum = a + b;
        a = b;
        b = sum;
    }

    b
}

#[jolt::provable]
pub fn zk_meme_token_policy(self_state_key: StateKey, tx: Transaction, x: Data, w: Data) {
    assert_eq!(self_state_key.tag, TOKEN);

    let in_amount = sum_token_amount(&self_state_key, &tx.ins);
    let out_amount = sum_token_amount(&self_state_key, &tx.outs);

    // is_meme_token_creator is a function that checks that
    // the spender is the creator of this meme token.
    // In our policy, the token creator can mint and burn tokens at will.
    assert!(in_amount == out_amount || is_meme_token_creator(&x, &w));
}

fn sum_token_amount(self_state_key: &StateKey, utxos: &[Utxo]) -> u64 {
    let mut in_amount: u64 = 0;
    for utxo in utxos {
        // We only care about UTXOs that have our token.
        if let Some(state) = utxo.get(self_state_key) {
            // There needs to be an `impl TryFrom<&Data> for u64`
            // for this to work.
            let utxo_amount: u64 = state.try_into().expect("token state value should be a u64");
            in_amount += utxo_amount;
        }
    }
    in_amount
}

fn is_meme_token_creator(x: &Data, w: &Data) -> bool {
    // TODO should be a real public key instead of a bunch of zeros
    const CREATOR_PUBLIC_KEY: [u8; 64] = [0u8; 64];

    // TODO check the signature in the witness against CREATOR_PUBLIC_KEY
    false
}
