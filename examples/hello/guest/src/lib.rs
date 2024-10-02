#![no_main]

use charms_data::{
    nft_state_preserved, AppId, Data, Transaction, Utxo, NFT, TOKEN,
};
use jolt::provable;

#[provable]
pub fn zk_meme_token_policy(
    self_app_id: AppId,
    tx: Transaction,
    x: Data,
    #[private] w: Data,
) {
    assert_eq!(self_app_id.tag, TOKEN);

    let in_amount = sum_token_amount(&self_app_id, &tx.ins);
    let out_amount = sum_token_amount(&self_app_id, &tx.outs);

    assert!(in_amount == out_amount);
}

#[provable]
pub fn example_token_policy(
    app_id: AppId,
    tx: Transaction,
    x: Data,
    #[private] w: Data,
) {
    assert_eq!(app_id.tag, TOKEN);

    let in_amount = sum_token_amount(&app_id, &tx.ins);
    let out_amount = sum_token_amount(&app_id, &tx.outs);

    if in_amount != out_amount {
        // enforce token mint/burn policy based on the transaction,
        // public and private witness data
        assert!(can_mint_or_burn(&app_id, &tx, &x, &w))
    }
}

#[provable]
pub fn example_nft(
    app_id: AppId,
    tx: Transaction,
    x: Data,
    #[private] w: Data,
) {
    assert_eq!(app_id.tag, NFT);

    // if the NFT state is unchanged (it was simply transferred),
    // no need to check if we can update the state
    if !nft_state_preserved(&app_id, &tx) {
        assert!(can_update_nft_state(&app_id, &tx, &x, &w))
    }
}

fn sum_token_amount(app_id: &AppId, utxos: &[Utxo]) -> u64 {
    let mut in_amount: u64 = 0;
    for utxo in utxos {
        // We only care about UTXOs that have our token.
        if let Some(state) = utxo.get(app_id) {
            // There needs to be an `impl TryFrom<&Data> for u64`
            // for this to work.
            let utxo_amount: u64 =
                state.try_into().expect("token state value should be a u64");
            in_amount += utxo_amount;
        }
    }
    in_amount
}

fn can_mint_or_burn(
    self_app_id: &AppId,
    tx: &Transaction,
    x: &Data,
    w: &Data,
) -> bool {
    // TODO should be a real public key instead of a bunch of zeros
    const CREATOR_PUBLIC_KEY: [u8; 64] = [0u8; 64];

    // TODO check the signature in the witness against CREATOR_PUBLIC_KEY
    false
}

fn can_update_nft_state(
    self_app_id: &AppId,
    tx: &Transaction,
    x: &Data,
    w: &Data,
) -> bool {
    // TODO should be a real public key instead of a bunch of zeros
    const CREATOR_PUBLIC_KEY: [u8; 64] = [0u8; 64];

    // TODO check the signature in the witness against CREATOR_PUBLIC_KEY
    false
}
