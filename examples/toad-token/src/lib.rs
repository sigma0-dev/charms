#![cfg_attr(feature = "guest", no_std)]
extern crate alloc;

pub(crate) use charms_data::{
    token_amounts_balanced, AppId, Charm, Transaction, UtxoId, NFT, TOKEN,
};
use jolt::provable;

#[provable(stack_size = 16384)]
pub fn toad_token_policy(self_app_id: AppId, tx: Transaction, _x: (), _w: ()) -> bool {
    assert_eq!(self_app_id.tag, TOKEN);
    assert!(
        token_amounts_balanced(&self_app_id, &tx).unwrap() || can_mint_or_burn(&self_app_id, &tx)
    );
    true
}

fn can_mint_or_burn(self_app_id: &AppId, tx: &Transaction) -> bool {
    let minting_utxo_id = Some(self_app_id.clone().id);

    // see if the transaction has an input with utxo_id == the token's
    // app_id.prefix or if it involves an NFT with app_id.prefix == the
    // token's app_id.prefix
    if tx.ins.iter().any(|utxo| {
        utxo.id == minting_utxo_id || charm_has_nft_with_app_id_prefix(&utxo.charm, &self_app_id.id)
    }) {
        return true;
    }

    false
}

fn charm_has_nft_with_app_id_prefix(charm: &Charm, nft_app_id_id: &UtxoId) -> bool {
    charm
        .iter()
        .any(|(app_id, _)| app_id.tag == NFT && nft_app_id_id == &app_id.id)
}
