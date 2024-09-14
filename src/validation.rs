use anyhow::{anyhow, Result};
use app_utxo_data::{
    nft_state_preserved, token_amounts_balanced, AppId, Data, Transaction, Witness, NFT, TOKEN,
};
use itertools::Itertools;
use std::collections::BTreeSet;

pub fn validate(tx: Transaction, witness: Witness) -> Result<()> {
    let app_ids = tx
        .ins
        .iter()
        .chain(tx.outs.iter())
        .map(|utxo| utxo.state.iter().map(|(k, _)| k))
        .flatten()
        .collect::<BTreeSet<_>>();

    for app_id in app_ids {
        match &app_id.tag {
            TOKEN if token_amounts_balanced(app_id, &tx) => {
                return Ok(());
            }
            NFT if nft_state_preserved(app_id, &tx) => {
                return Ok(());
            }
            _ => {}
        }

        let witness_data = witness
            .get(app_id)
            .ok_or_else(|| anyhow!("WitnessData missing for key {:?}", app_id))?;

        let proof = WrappedProof::from(&witness_data.proof);
        proof.verify(app_id, &tx, &witness_data.public_input)?;
    }

    Ok(())
}

trait Proof {
    fn verify(&self, self_app_id: &AppId, tx: &Transaction, public_input: &Data) -> Result<()>;
}

struct WrappedProof {
    proof: Data,
}

impl From<&Data> for WrappedProof {
    fn from(data: &Data) -> Self {
        Self {
            proof: data.clone(),
        }
    }
}

impl Proof for WrappedProof {
    fn verify(&self, self_app_id: &AppId, tx: &Transaction, public_input: &Data) -> Result<()> {
        todo!()
    }
}
