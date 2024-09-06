use anyhow::{anyhow, Result};
use app_utxo_data::{sum_token_amount, Data, StateKey, Transaction, Witness, NFT, TOKEN};
use itertools::Itertools;
use std::collections::BTreeSet;

pub fn validate(tx: Transaction, witness: Witness) -> Result<()> {
    let state_keys = tx
        .ins
        .iter()
        .chain(tx.outs.iter())
        .map(|utxo| utxo.state.iter().map(|(k, _)| k))
        .flatten()
        .collect::<BTreeSet<_>>();

    for state_key in state_keys {
        match &state_key.tag {
            TOKEN if token_amounts_balanced(state_key, &tx)? => {
                return Ok(());
            }
            NFT if nft_state_unchanged(state_key, &tx)? => {
                return Ok(());
            }
            _ => {}
        }

        let witness_data = witness
            .get(state_key)
            .ok_or_else(|| anyhow!("WitnessData missing for key {:?}", state_key))?;

        let proof = WrappedProof::from(&witness_data.proof);
        proof.verify(state_key, &tx, &witness_data.public_input)?;
    }

    Ok(())
}

fn token_amounts_balanced(state_key: &StateKey, tx: &Transaction) -> Result<bool> {
    let token_amount_in = sum_token_amount(state_key, &tx.ins)?;
    let token_amount_out = sum_token_amount(state_key, &tx.outs)?;
    Ok(token_amount_in == token_amount_out)
}

fn nft_state_unchanged(state_key: &StateKey, tx: &Transaction) -> Result<bool> {
    let nft_states_multiset_in = tx
        .ins
        .iter()
        .filter_map(|utxo| utxo.state.get(state_key))
        .map(|&s| (s, ()))
        .into_group_map();
    let nft_states_multiset_out = tx
        .outs
        .iter()
        .filter_map(|utxo| utxo.state.get(state_key))
        .map(|&s| (s, ()))
        .into_group_map();

    Ok(nft_states_multiset_in == nft_states_multiset_out)
}

trait Proof {
    fn verify(&self, self_state_key: &StateKey, tx: &Transaction, public_input: &Data) -> Result<()>;
}

struct WrappedProof {
    proof: Data,
}

impl From<&Data> for WrappedProof {
    fn from(data: &Data) -> Self {
        Self { proof: data.clone() }
    }
}

impl Proof for WrappedProof {
    fn verify(&self, self_state_key: &StateKey, tx: &Transaction, public_input: &Data) -> Result<()> {
        todo!()
    }
}
