use anyhow::Result;
use std::collections::{BTreeMap, HashMap};

pub struct Transaction {
    pub ins: Vec<Utxo>,
    pub refs: Vec<Utxo>,
    pub outs: Vec<Utxo>,
}

pub type Witness = BTreeMap<StateKey, WitnessData>;

// App UTXO as presented to the validation predicate.
pub struct Utxo {
    id: Option<UtxoId>,
    amount: u64,
    state: BTreeMap<StateKey, StateValue>,
}

#[derive(Eq, PartialEq, Hash, Default, Clone, Debug)]
struct UtxoId {
    txid: TxId,
    vout: u32,
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

#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Default, Clone, Debug)]
pub struct StateKey {
    pub tag: &'static [u8],
    pub prefix: Vec<u8>,
    pub vk_hash: VkHash,
}

pub type StateValue = Data;

type TxId = [u8; 32];

pub struct WitnessData {
    pub proof: Data, // TODO real proof type
    pub public_input: Data,
}

pub type VkHash = [u8; 32];

pub struct Data {
    data: Box<[u8]>,
}

impl Data {
    pub fn new(data: Box<[u8]>) -> Self {
        Self { data }
    }

    pub fn empty() -> Self {
        Self { data: Box::new([]) }
    }
}

pub const TOKEN_TAG: &[u8] = b"Token";

pub const NFT_TAG: &[u8] = b"NFT";

mod tests {
    use byteorder::{ByteOrder, LittleEndian};
    use itertools::Itertools;

    use super::*;

    impl From<&Data> for u64 {
        fn from(data: &Data) -> Self {
            let buffer = data.data.iter().take(8).copied().pad_using(8, |_| 0).collect_vec();
            LittleEndian::read_u64(&buffer)
        }
    }

    pub fn zk_meme_token_policy(
        self_state_key: &StateKey,
        ins: &[Utxo],
        outs: &[Utxo],
        x: &Data,
        w: &Data,
    ) -> Result<bool> {
        assert_eq!(self_state_key.tag, TOKEN_TAG);

        let in_amount = sum_token_amount(self_state_key, ins)?;
        let out_amount = sum_token_amount(self_state_key, outs)?;

        // is_meme_token_creator is a function that checks that
        // the spender is the creator of this meme token.
        // In our policy, the token creator can mint and burn tokens at will.
        Ok(in_amount == out_amount || is_meme_token_creator(x, w)?)
    }

    fn sum_token_amount(self_state_key: &StateKey, utxos: &[Utxo]) -> Result<u64> {
        let mut in_amount: u64 = 0;
        for utxo in utxos {
            // We only care about UTXOs that have our token.
            if let Some(state) = utxo.state.get(self_state_key) {
                // There needs to be an `impl TryFrom<&Data> for u64`
                // for this to work.
                let utxo_amount: u64 = state.try_into()?;
                in_amount += utxo_amount;
            }
        }
        Ok(in_amount)
    }

    fn is_meme_token_creator(x: &Data, w: &Data) -> Result<bool> {
        // TODO should be a real public key instead of a bunch of zeros
        const CREATOR_PUBLIC_KEY: [u8; 64] = [0u8; 64];
        todo!("check the signature in the witness against CREATOR_PUBLIC_KEY")
    }

    impl TryFrom<&Data> for String {
        type Error = anyhow::Error;

        fn try_from(data: &Data) -> std::result::Result<Self, Self::Error> {
            Ok(String::from_utf8(data.data.to_vec())?)
        }
    }

    pub fn spender_owns_email_contract(
        self_state_key: &StateKey,
        ins: &[Utxo],
        outs: &[Utxo],
        x: &Data,
        w: &Data,
    ) -> Result<bool> {
        // Make sure the spender owns the email addresses in the input UTXOs.
        for utxo in ins {
            // Retrieve the state for this zkapp.
            // OWN_VK_HASH (always zeroed out) refers to the current validator's
            // own VK hash in the UTXO (as presented to the validator).
            // In an actual UTXO, the hash of the validator's VK is used instead.
            // Also, we only care about UTXOs that have a state for the current
            // validator.
            if let Some(state) = utxo.state.get(self_state_key) {
                // If the state is not even a string, the UTXO is invalid.
                let email: String = state.try_into()?;
                // Check if the spender owns the email address.
                if !owns_email(&email, x, w)? {
                    return Ok(false);
                }
            }
        }

        // Make sure our own state in output UTXOs is an email address.
        for utxo in outs {
            // Again, we only care about UTXOs that have a state for the current
            // validator.
            if let Some(state) = utxo.state.get(self_state_key) {
                // There needs to be an `impl TryFrom<&Data> for String`
                // for this to work.
                let email: String = state.try_into()?;
                // Check if the email address is valid XD
                if !email.contains('@') {
                    return Ok(false);
                }
            }
        }

        Ok(true)
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

    pub fn rollup_validator(ins: &[Utxo], outs: &[Utxo], x: &Data, w: &Data) -> Result<bool> {
        Ok(true)
    }

    #[test]
    fn test_zk_meme_token_validator() {
        let ins = vec![Utxo {
            id: Some(UtxoId::empty()),
            amount: 1,
            state: BTreeMap::new(),
        }];
        let outs = vec![Utxo {
            id: None,
            amount: 1,
            state: BTreeMap::new(),
        }];
        assert_eq!(
            zk_meme_token_policy(&StateKey::default(), &ins, &outs, &Data::empty(), &Data::empty()).unwrap(),
            true
        );
    }
}
