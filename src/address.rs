use bitcoin::{
    bip32::{DerivationPath, Xpriv},
    key::{Keypair, Parity, UntweakedKeypair},
    opcodes::all::{OP_2DROP, OP_CHECKSIG, OP_DROP},
    secp256k1::Secp256k1,
    taproot::{ControlBlock, LeafVersion, TaprootBuilder, TaprootSpendInfo},
    Address, Network, PrivateKey, PublicKey, ScriptBuf, XOnlyPublicKey,
};

pub fn magic_address(public_key: XOnlyPublicKey, network: Network) -> Address {
    Address::p2tr_tweaked(taproot_spend_info(public_key).output_key(), network)
}

pub fn derive_private_key(
    xpriv: &Xpriv,
    derivation_path: &DerivationPath,
    network: Network,
) -> PrivateKey {
    let secp = Secp256k1::new();
    let child_key = xpriv.derive_priv(&secp, derivation_path).unwrap();
    let private_key = child_key.private_key;

    PrivateKey::new(private_key, network)
}

pub fn control_block(public_key: XOnlyPublicKey) -> ControlBlock {
    taproot_spend_info(public_key)
        .control_block(&(spend_script(public_key), LeafVersion::TapScript))
        .unwrap()
}

pub fn spend_script(public_key: XOnlyPublicKey) -> ScriptBuf {
    ScriptBuf::builder()
        .push_opcode(OP_2DROP) // drop "spell", <spell_data>
        .push_slice(public_key.serialize())
        .push_opcode(OP_CHECKSIG)
        .into_script()
}

fn taproot_spend_info(public_key: XOnlyPublicKey) -> TaprootSpendInfo {
    let secp256k1 = Secp256k1::new();
    TaprootBuilder::new()
        .add_leaf(0, spend_script(public_key))
        .unwrap()
        .finalize(&secp256k1, public_key)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use std::str::FromStr;

    const NETWORK: Network = Network::Testnet4;

    // original address: tb1p38tsza94t9pak7g52zku06c7r9jqcjx67l4r7d7ehw0ch8fm8cysknej4p

    #[test]
    fn test_get_private_key() {
        let tprv = "tprv8ZgxMBicQKsPdY8xCZptFjJ8HV5TFQr7K3k5Ue1cKfgjR3GuXps3JbFFXP2rLKxQ84u93ZvaXemwpmQcYcLwBS9gSrkRuNMZMdjvwUAdgwU";
        let path = "m/86h/1h/0h/0/17";

        let xkey = Xpriv::from_str(tprv).unwrap();
        let derivation_path = DerivationPath::from_str(path).unwrap();

        let new_priv_key = derive_private_key(&xkey, &derivation_path, NETWORK);
        dbg!(new_priv_key.to_wif());

        let secp256k1 = Secp256k1::new();
        let keypair = Keypair::from_secret_key(&secp256k1, &new_priv_key.inner);

        dbg!(magic_address(XOnlyPublicKey::from_keypair(&keypair).0, NETWORK).to_string());
    }
}
