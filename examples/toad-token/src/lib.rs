use charms_sdk::data::{
    check, nft_state_preserved, sum_token_amount, token_amounts_balanced, App, Data, Transaction,
    NFT, TOKEN,
};

pub fn app_contract(app: &App, tx: &Transaction, x: &Data, w: &Data) -> bool {
    let empty = Data::empty();
    assert_eq!(x, &empty);
    assert_eq!(w, &empty);
    match app.tag {
        NFT => {
            check!(nft_contract_satisfied(app, tx))
        }
        TOKEN => {
            check!(token_contract_satisfied(app, tx))
        }
        _ => unreachable!(),
    }
    true
}

fn nft_contract_satisfied(app: &App, tx: &Transaction) -> bool {
    let token_app = &App {
        tag: TOKEN,
        id: app.id.clone(),
        vk_hash: app.vk_hash.clone(),
    };
    check!(nft_state_preserved(app, tx) || can_mint_nft(app, tx) || can_mint_token(&token_app, tx));
    true
}

fn can_mint_nft(nft_app: &App, tx: &Transaction) -> bool {
    // can only mint an NFT with this contract if spending a UTXO with the same ID.
    check!(tx.ins.iter().any(|(utxo_id, _)| utxo_id == &nft_app.id));
    // can mint no more than one NFT.
    check!(
        tx.outs
            .iter()
            .filter(|&charm| charm.iter().any(|(app, _)| app == nft_app))
            .count()
            == 1
    );
    true
}

fn token_contract_satisfied(token_app: &App, tx: &Transaction) -> bool {
    check!(token_amounts_balanced(token_app, tx) || can_mint_token(token_app, tx));
    true
}

fn can_mint_token(token_app: &App, tx: &Transaction) -> bool {
    let nft_app = App {
        tag: NFT,
        id: token_app.id.clone(),
        vk_hash: token_app.vk_hash.clone(),
    };

    let Some(incoming_supply): Option<u64> = tx
        .ins
        .iter()
        .find_map(|(_, charm)| charm.get(&nft_app).cloned())
        .and_then(|data| u64::try_from(&data).ok())
    else {
        eprintln!("could not determine incoming supply");
        return false;
    };

    let Some(outgoing_supply): Option<u64> = tx
        .outs
        .iter()
        .find_map(|charm| charm.get(&nft_app).cloned())
        .and_then(|data| u64::try_from(&data).ok())
    else {
        eprintln!("could not determine outgoing supply");
        return false;
    };

    if !(incoming_supply >= outgoing_supply) {
        eprintln!("incoming supply must be greater than or equal to outgoing supply");
        return false;
    }

    let Some(input_token_amount) = sum_token_amount(&token_app, tx.ins.values()).ok() else {
        eprintln!("could not determine input token amount");
        return false;
    };
    let Some(output_token_amount) = sum_token_amount(&token_app, tx.outs.iter()).ok() else {
        eprintln!("could not determine output token amount");
        return false;
    };

    // can mint no more than what's allowed by the managing NFT state change.
    output_token_amount - input_token_amount == incoming_supply - outgoing_supply
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
