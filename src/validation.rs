use anyhow::{anyhow, Result};
use app_utxo_data::{Data, StateKey, Transaction, Witness};
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
        let witness_data = witness
            .get(state_key)
            .ok_or_else(|| anyhow!("WitnessData missing for key {:?}", state_key))?;

        let proof = WrappedProof::from(&witness_data.proof);
        proof.verify(state_key, &tx, &witness_data.public_input)?;
    }

    Ok(())
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
