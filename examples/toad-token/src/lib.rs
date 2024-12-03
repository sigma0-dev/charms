use charms_data::{
    nft_state_preserved, token_amounts_balanced, App, Charm, Data, Transaction, UtxoId, NFT, TOKEN,
};

pub fn main() {
    let (app, tx, x, w): (App, Transaction, Data, Data) = sp1_zkvm::io::read();
    assert_eq!(x, Data::empty());
    assert_eq!(w, Data::empty());
    assert!(app_contract(&app, &tx, (), ()));
    sp1_zkvm::io::commit(&(&app, &tx, &x));
}

pub fn app_contract(app: &App, tx: &Transaction, _x: (), _w: ()) -> bool {
    match app.tag {
        NFT => {
            assert!(nft_contract_satisfied(app, tx))
        }
        TOKEN => {
            assert!(token_contract_satisfied(app, tx))
        }
        _ => unreachable!(),
    }
    true
}

fn nft_contract_satisfied(app: &App, tx: &Transaction) -> bool {
    assert!(nft_state_preserved(app, tx) || can_mint_nft(app, tx));
    true
}

fn can_mint_nft(self_app: &App, tx: &Transaction) -> bool {
    // can only mint an NFT with this contract if spending a UTXO with the same ID.
    assert!(tx
        .ins
        .iter()
        .any(|utxo| utxo.id == Some(self_app.id.clone())));
    // can mint no more than one NFT.
    assert_eq!(
        tx.outs
            .iter()
            .filter(|&utxo| utxo.charm.iter().any(|(app, _)| app.id == self_app.id))
            .count(),
        1
    );
    true
}

fn token_contract_satisfied(app: &App, tx: &Transaction) -> bool {
    assert!(token_amounts_balanced(app, tx) || can_mint_token(app, tx));
    true
}

fn can_mint_token(app: &App, tx: &Transaction) -> bool {
    // see if there's any input with an NFT with app.id == the token's app.id
    if tx
        .ins
        .iter()
        .any(|utxo| charm_has_managing_nft(&utxo.charm, &app.id))
    {
        return true;
    }

    false
}

fn charm_has_managing_nft(charm: &Charm, nft_id: &UtxoId) -> bool {
    charm
        .iter()
        .any(|(app, _)| app.tag == NFT && &app.id == nft_id)
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
