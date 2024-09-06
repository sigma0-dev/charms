use anyhow::{bail, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Read};

#[derive(Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub ins: Vec<Utxo>,
    pub refs: Vec<Utxo>,
    pub outs: Vec<Utxo>,
}

pub type Witness = BTreeMap<AppKey, WitnessData>;

// App UTXO as presented to the validation predicate.
#[derive(Clone, Serialize, Deserialize)]
pub struct Utxo {
    pub id: Option<UtxoId>,
    pub amount: u64,
    pub state: BTreeMap<AppKey, AppData>,
}

impl Utxo {
    pub fn get(&self, key: &AppKey) -> Option<&AppData> {
        self.state.get(key)
    }
}

#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Default, Clone, Debug, Serialize, Deserialize)]
pub struct UtxoId {
    pub txid: TxId,
    pub vout: u32,
}

impl UtxoId {
    pub fn new(txid: [u8; 32], vout: u32) -> Self {
        Self { txid, vout }
    }

    pub fn empty() -> Self {
        Self {
            txid: [0u8; 32],
            vout: 0,
        }
    }
}

#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Default, Clone, Debug, Serialize, Deserialize)]
pub struct AppKey {
    pub tag: Vec<u8>,
    pub prefix: Vec<u8>,
    pub vk_hash: VkHash,
}

pub type AppData = Data;

type TxId = [u8; 32];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WitnessData {
    pub proof: Data,
    pub public_input: Data,
}

pub type VkHash = [u8; 32];

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data(pub Vec<u8>);

impl Data {
    pub fn empty() -> Self {
        Self(vec![])
    }
}

impl TryFrom<&Data> for u64 {
    type Error = anyhow::Error;

    fn try_from(value: &Data) -> Result<Self> {
        if value.0.len() > 8 {
            bail!("Data too long to convert to u64");
        }
        Ok(u64::from_le_bytes(value.clone().0.try_into().unwrap()))
    }
}

impl From<u64> for Data {
    fn from(value: u64) -> Self {
        Self(value.to_le_bytes().to_vec())
    }
}

pub const TOKEN: &[u8] = b"Token";
pub const NFT: &[u8] = b"NFT";

pub fn sum_token_amount(self_app_key: &AppKey, utxos: &[Utxo]) -> Result<u64> {
    let mut in_amount: u64 = 0;
    for utxo in utxos {
        // We only care about UTXOs that have our token.
        if let Some(state) = utxo.get(self_app_key) {
            let utxo_amount: u64 = state.try_into()?;
            in_amount += utxo_amount;
        }
    }
    Ok(in_amount)
}

mod tests {
    use anyhow::{bail, ensure};
    use itertools::Itertools;

    use super::*;

    pub fn zk_meme_token_policy(self_app_key: &AppKey, tx: &Transaction, x: &Data, w: &Data) -> Result<()> {
        assert_eq!(self_app_key.tag, TOKEN);

        let in_amount = sum_token_amount(self_app_key, &tx.ins)?;
        let out_amount = sum_token_amount(self_app_key, &tx.outs)?;

        // is_meme_token_creator is a function that checks that
        // the spender is the creator of this meme token.
        // In our policy, the token creator can mint and burn tokens at will.
        assert!(in_amount == out_amount || is_meme_token_creator(x, w)?);

        Ok(())
    }

    fn is_meme_token_creator(x: &Data, w: &Data) -> Result<bool> {
        // TODO should be a real public key instead of a bunch of zeros
        const CREATOR_PUBLIC_KEY: [u8; 64] = [0u8; 64];
        // todo!("check the signature in the witness against CREATOR_PUBLIC_KEY")
        Ok(false)
    }

    impl TryFrom<&Data> for String {
        type Error = anyhow::Error;

        fn try_from(data: &Data) -> std::result::Result<Self, Self::Error> {
            Ok(String::from_utf8(data.0.to_vec())?)
        }
    }

    pub fn spender_owns_email_contract(self_app_key: &AppKey, tx: &Transaction, x: &Data, w: &Data) -> Result<()> {
        // Make sure the spender owns the email addresses in the input UTXOs.
        for utxo in &tx.ins {
            // Retrieve the state for this zkapp.
            // OWN_VK_HASH (always zeroed out) refers to the current validator's
            // own VK hash in the UTXO (as presented to the validator).
            // In an actual UTXO, the hash of the validator's VK is used instead.
            // Also, we only care about UTXOs that have a state for the current
            // validator.
            if let Some(state) = utxo.get(self_app_key) {
                // If the state is not even a string, the UTXO is invalid.
                let email: String = state.try_into()?;
                // Check if the spender owns the email address.
                ensure!(owns_email(&email, x, w)?);
            }
        }

        // Make sure our own state in output UTXOs is an email address.
        for utxo in &tx.outs {
            // Again, we only care about UTXOs that have a state for the current
            // validator.
            if let Some(state) = utxo.get(self_app_key) {
                // There needs to be an `impl TryFrom<&Data> for String`
                // for this to work.
                let email: String = state.try_into()?;
                // Check if the email address is valid XD
                ensure!(email.contains('@'));
            }
        }

        Ok(())
    }

    fn owns_email(email: &str, x: &Data, w: &Data) -> Result<bool> {
        todo!("Implement!")
    }

    struct RollupState(String);

    impl TryFrom<&Data> for RollupState {
        type Error = anyhow::Error;

        fn try_from(data: &Data) -> std::result::Result<Self, Self::Error> {
            todo!("Implement!")
        }
    }

    pub fn rollup_validator(self_app_key: &AppKey, tx: &Transaction, x: &Data, w: &Data) -> Result<()> {
        todo!()
    }

    #[test]
    fn test_zk_meme_token_validator() {
        let token_app_key = AppKey {
            tag: TOKEN.to_vec(),
            prefix: vec![],
            vk_hash: [0u8; 32],
        };

        let ins = vec![Utxo {
            id: Some(UtxoId::empty()),
            amount: 1,
            state: BTreeMap::from([(token_app_key.clone(), 1u64.into())]),
        }];
        let outs = vec![Utxo {
            id: None,
            amount: 1,
            state: BTreeMap::from([(token_app_key.clone(), 1u64.into())]),
        }];

        let tx = Transaction {
            ins,
            refs: vec![],
            outs,
        };

        assert!(zk_meme_token_policy(&token_app_key, &tx, &Data::empty(), &Data::empty()).is_ok());
    }
}
