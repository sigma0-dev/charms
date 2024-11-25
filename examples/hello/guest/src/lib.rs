#![cfg_attr(feature = "guest", no_std)]
#![no_main]
extern crate alloc;

use alloc::vec::Vec;
use charms_data::{
    app_state_multiset, nft_state_preserved, sum_token_amount,
    token_amounts_balanced, AppId, Data, Transaction, Utxo, UtxoId, NFT, TOKEN,
};
use jolt::provable;

#[provable(stack_size = 16384)]
pub fn zk_meme_token_policy(
    app_id: AppId,
    tx: Transaction,
    _x: (),
    #[private] _w: (),
) -> bool {
    assert_eq!(app_id.tag, TOKEN);
    assert!(token_amounts_balanced(&app_id, &tx));
    true
}

// #[provable]
// pub fn example_token_policy(
//     app_id: AppId,
//     tx: Transaction,
//     x: Data,
//     #[private] w: Data,
// ) {
//     assert_eq!(app_id.tag, TOKEN);
//
//     if !token_amounts_balanced(&app_id, &tx).unwrap() {
//         // enforce token mint/burn policy based on the transaction,
//         // public and private witness data
//         assert!(can_mint_or_burn(&app_id, &tx, &x, &w))
//     }
// }

// #[provable]
// pub fn example_nft(
//     app_id: AppId,
//     tx: Transaction,
//     x: Data,
//     #[private] w: Data,
// ) {
//     assert_eq!(app_id.tag, NFT);
//
//     // if the NFT state is unchanged (it was simply transferred),
//     // no need to check if we can update the state
//     if !nft_state_preserved(&app_id, &tx) {
//         assert!(can_update_nft_state(&app_id, &tx, &x, &w))
//     }
// }

// fn can_mint_or_burn(
//     self_app_id: &AppId,
//     tx: &Transaction,
//     x: &Data,
//     w: &Data,
// ) -> bool {
//     // TODO should be a real public key instead of a bunch of zeros
//     const CREATOR_PUBLIC_KEY: [u8; 64] = [0u8; 64];
//
//     // TODO check the signature in the witness against CREATOR_PUBLIC_KEY
//     false
// }
//
// fn can_update_nft_state(
//     app_id: &AppId,
//     tx: &Transaction,
//     x: &Data,
//     w: &Data,
// ) -> bool {
//     match app_state_multiset(app_id, &tx.ins).len() {
//         0 => {
//             // minting a new NFT
//             if contains_utxo_id(&app_id.id, &tx.ins) {
//                 // can only mint an NFT with app_id.prefix ==
//                 // spent UTXO_ID
//                 return false;
//             }
//
//             // TODO: enforce NFT mint policy based on the transaction,
//             //       public and private witness data
//
//             true
//         }
//         _ => false, // can't update existing NFT state
//     }
// }
//
// fn contains_utxo_id(expected_id: &UtxoId, utxos: &Vec<Utxo>) -> bool {
//     let spent_utxo_id = utxos
//         .iter()
//         .filter_map(|utxo| match &utxo.id {
//             Some(id) if id == expected_id => Some(id),
//             _ => None,
//         })
//         .next();
//     let result = spent_utxo_id.is_none();
//     result
// }

// impl From<&Data> for String {
//     fn from(data: &Data) -> Self {
//         String::from_utf8(data.0.to_vec()).unwrap()
//     }
// }
//
// pub fn spender_owns_email_contract(
//     self_app_id: &AppId,
//     tx: &Transaction,
//     x: &Data,
//     w: &Data,
// ) -> Result<()> {
//     // Make sure the spender owns the email addresses in the input UTXOs.
//     for utxo in &tx.ins {
//         // Retrieve the state for this zkapp.
//         // OWN_VK_HASH (always zeroed out) refers to the current validator's
//         // own VK hash in the UTXO (as presented to the validator).
//         // In an actual UTXO, the hash of the validator's VK is used instead.
//         // Also, we only care about UTXOs that have a state for the current
//         // validator.
//         if let Some(state) = utxo.get(self_app_id) {
//             // If the state is not even a string, the UTXO is invalid.
//             let email: String = state.into();
//             // Check if the spender owns the email address.
//             ensure!(owns_email(&email, x, w)?);
//         }
//     }
//
//     // Make sure our own state in output UTXOs is an email address.
//     for utxo in &tx.outs {
//         // Again, we only care about UTXOs that have a state for the current
//         // validator.
//         if let Some(state) = utxo.get(self_app_id) {
//             // There needs to be an `impl TryFrom<&Data> for String`
//             // for this to work.
//             let email: String = state.into();
//             // Check if the email address is valid XD
//             ensure!(email.contains('@'));
//         }
//     }
//
//     Ok(())
// }
//
// fn owns_email(email: &str, x: &Data, w: &Data) -> Result<bool> {
//     todo!("Implement!")
// }
