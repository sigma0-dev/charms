use crate::data::{Data, StateKey, Transaction, Witness};
use anyhow::{anyhow, Result};
use std::collections::BTreeSet;

pub fn validate(tx: Transaction, witness: Witness) -> Result<()> {
    let state_keys = tx
        .ins
        .iter()
        .chain(tx.outs.iter())
        .map(|utxo| utxo.state.keys())
        .flatten()
        .collect::<BTreeSet<_>>();

    for key in state_keys {
        let witness_data = witness
            .get(key)
            .ok_or_else(|| anyhow!("WitnessData missing for key {:?}", key))?;

        let proof = WrappedProof::from(&witness_data.proof);
        proof.verify(key, &tx, &witness_data.public_input)?;
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
