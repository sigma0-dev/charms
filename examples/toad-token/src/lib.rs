use charms_data::{
    nft_state_preserved, token_amounts_balanced, AppId, Charm, Data, Transaction, UtxoId, NFT,
    TOKEN,
};

pub fn main() {
    let (app_id, tx, x, w): (AppId, Transaction, Data, Data) = sp1_zkvm::io::read();
    assert_eq!(x, Data::empty());
    assert_eq!(w, Data::empty());
    assert!(app_contract(&app_id, &tx, (), ()));
    sp1_zkvm::io::commit(&(&app_id, &tx, &x));
}

pub fn app_contract(app_id: &AppId, tx: &Transaction, _x: (), _w: ()) -> bool {
    match app_id.tag {
        NFT => {
            assert!(nft_contract_satisfied(app_id, tx))
        }
        TOKEN => {
            assert!(token_contract_satisfied(app_id, tx))
        }
        _ => unreachable!(),
    }
    true
}

fn nft_contract_satisfied(app_id: &AppId, tx: &Transaction) -> bool {
    assert!(nft_state_preserved(app_id, tx) || can_mint_nft(app_id, tx));
    true
}

fn can_mint_nft(self_app_id: &AppId, tx: &Transaction) -> bool {
    // can only mint an NFT with this contract if spending a UTXO with the same ID.
    assert!(tx
        .ins
        .iter()
        .any(|utxo| utxo.id == Some(self_app_id.id.clone())));
    // can mint no more than one NFT.
    assert_eq!(
        tx.outs
            .iter()
            .filter(|&utxo| utxo
                .charm
                .iter()
                .any(|(app_id, _)| app_id.id == self_app_id.id))
            .count(),
        1
    );
    true
}

fn token_contract_satisfied(app_id: &AppId, tx: &Transaction) -> bool {
    assert!(token_amounts_balanced(app_id, tx) || can_mint_token(app_id, tx));
    true
}

fn can_mint_token(app_id: &AppId, tx: &Transaction) -> bool {
    // see if there's any input with an NFT with app_id.id == the token's app_id.id
    if tx
        .ins
        .iter()
        .any(|utxo| charm_has_managing_nft(&utxo.charm, &app_id.id))
    {
        return true;
    }

    false
}

fn charm_has_managing_nft(charm: &Charm, nft_id: &UtxoId) -> bool {
    charm
        .iter()
        .any(|(app_id, _)| app_id.tag == NFT && &app_id.id == nft_id)
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
