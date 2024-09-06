use anyhow::{anyhow, Result};
use app_utxo_data::{sum_token_amount, AppKey, Data, Transaction, Witness, NFT, TOKEN};
use itertools::Itertools;
use std::collections::BTreeSet;

pub fn validate(tx: Transaction, witness: Witness) -> Result<()> {
    let app_keys = tx
        .ins
        .iter()
        .chain(tx.outs.iter())
        .map(|utxo| utxo.state.iter().map(|(k, _)| k))
        .flatten()
        .collect::<BTreeSet<_>>();

    for app_key in app_keys {
        match &app_key.tag {
            TOKEN if token_amounts_balanced(app_key, &tx)? => {
                return Ok(());
            }
            NFT if nft_state_unchanged(app_key, &tx)? => {
                return Ok(());
            }
            _ => {}
        }

        let witness_data = witness
            .get(app_key)
            .ok_or_else(|| anyhow!("WitnessData missing for key {:?}", app_key))?;

        let proof = WrappedProof::from(&witness_data.proof);
        proof.verify(app_key, &tx, &witness_data.public_input)?;
    }

    Ok(())
}

fn token_amounts_balanced(app_key: &AppKey, tx: &Transaction) -> Result<bool> {
    let token_amount_in = sum_token_amount(app_key, &tx.ins)?;
    let token_amount_out = sum_token_amount(app_key, &tx.outs)?;
    Ok(token_amount_in == token_amount_out)
}

fn nft_state_unchanged(app_key: &AppKey, tx: &Transaction) -> Result<bool> {
    let nft_states_multiset_in = tx
        .ins
        .iter()
        .filter_map(|utxo| utxo.state.get(app_key))
        .map(|&s| (s, ()))
        .into_group_map();
    let nft_states_multiset_out = tx
        .outs
        .iter()
        .filter_map(|utxo| utxo.state.get(app_key))
        .map(|&s| (s, ()))
        .into_group_map();

    Ok(nft_states_multiset_in == nft_states_multiset_out)
}

trait Proof {
    fn verify(&self, self_app_key: &AppKey, tx: &Transaction, public_input: &Data) -> Result<()>;
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
    fn verify(&self, self_app_key: &AppKey, tx: &Transaction, public_input: &Data) -> Result<()> {
        todo!()
    }
}
