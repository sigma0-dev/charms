use charms_sdk::data::{
    check, nft_state_preserved, sum_token_amount, token_amounts_balanced, App, Data, Transaction,
    B32, NFT, TOKEN,
};
use sha2::{Digest, Sha256};

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
        identity: app.identity.clone(),
        vk: app.vk.clone(),
    };
    check!(nft_state_preserved(app, tx) || can_mint_nft(app, tx) || can_mint_token(&token_app, tx));
    true
}

fn can_mint_nft(nft_app: &App, tx: &Transaction) -> bool {
    // can only mint an NFT with this contract if spending a UTXO with the same ID.
    check!(tx
        .ins
        .iter()
        .any(|(utxo_id, _)| &hash(&Data::from(utxo_id)) == &nft_app.identity));
    // can mint no more than one NFT.
    check!(
        tx.outs
            .iter()
            .filter(|&charms| charms.iter().any(|(app, _)| app == nft_app))
            .count()
            == 1
    );
    true
}

fn hash(data: &Data) -> B32 {
    let hash = Sha256::digest(data.byte_repr());
    B32(hash.into())
}

fn token_contract_satisfied(token_app: &App, tx: &Transaction) -> bool {
    check!(token_amounts_balanced(token_app, tx) || can_mint_token(token_app, tx));
    true
}

fn can_mint_token(token_app: &App, tx: &Transaction) -> bool {
    let nft_app = App {
        tag: NFT,
        identity: token_app.identity.clone(),
        vk: token_app.vk.clone(),
    };

    let Some(incoming_supply): Option<u64> = tx
        .ins
        .iter()
        .find_map(|(_, charms)| charms.get(&nft_app).cloned())
        .and_then(|data| data.value().ok())
    else {
        eprintln!("could not determine incoming supply");
        return false;
    };

    let Some(outgoing_supply): Option<u64> = tx
        .outs
        .iter()
        .find_map(|charms| charms.get(&nft_app).cloned())
        .and_then(|data| data.value().ok())
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
