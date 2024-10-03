use anyhow::{anyhow, ensure, Result};
use ark_serialize::CanonicalDeserialize;
use bitcoin::hashes::Hash;
use charms_data::{
    nft_state_preserved, token_amounts_balanced, AppId, Data, Transaction, VKs, VkHash, Witness,
    NFT, TOKEN, VK,
};
use itertools::Itertools;
use jolt::{host::ELFInstruction, Jolt, JoltHyperKZGProof, JoltPreprocessing, RV32IJoltVM, F, PCS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub fn validate(tx: &Transaction, witness: &Witness, vks: &VKs) -> Result<()> {
    let app_ids = tx
        .ins
        .iter()
        .chain(tx.outs.iter())
        .map(|utxo| utxo.charm.iter().map(|(k, _)| k))
        .flatten()
        .collect::<BTreeSet<_>>();

    for app_id in app_ids {
        match &app_id.tag {
            TOKEN if token_amounts_balanced(app_id, tx) => {
                return Ok(());
            }
            NFT if nft_state_preserved(app_id, tx) => {
                return Ok(());
            }
            _ => {}
        }

        let witness_data = witness
            .get(app_id)
            .ok_or_else(|| anyhow!("WitnessData missing for key {:?}", app_id))?;

        let proof = WrappedProof::try_from(&witness_data.proof)?;

        let vk = vks
            .get(&app_id.vk_hash)
            .ok_or_else(|| anyhow!("VK missing for key {:?}", app_id))?;

        proof.verify(app_id, vk.try_into()?, tx, &witness_data.public_input)?;
    }

    Ok(())
}

pub trait Proof {
    type VK;

    fn verify(
        self,
        app_id: &AppId,
        vk: &Self::VK,
        tx: &Transaction,
        public_input: &Data,
    ) -> Result<()>;
}

pub struct WrappedProof {
    proof: JoltHyperKZGProof,
}

impl TryFrom<&Data> for WrappedProof {
    type Error = anyhow::Error;

    fn try_from(value: &Data) -> std::result::Result<Self, Self::Error> {
        // deserialize proof from data
        let proof = JoltHyperKZGProof::deserialize_compressed(&value)?;
        Ok(Self { proof })
    }
}

impl Proof for WrappedProof {
    type VK = WrappedVK;

    fn verify(
        self,
        app_id: &AppId,
        vk: &Self::VK,
        tx: &Transaction,
        public_input: &Data,
    ) -> Result<()> {
        assert_eq!(vk.hash(), &app_id.vk_hash);

        let WrappedVK {
            bytecode,
            memory_init,
        } = vk.clone();

        dbg!("preprocessing");
        let preproc: JoltPreprocessing<4, F, PCS> =
            RV32IJoltVM::preprocess(bytecode, memory_init, 1 << 20, 1 << 20, 1 << 20);

        let JoltHyperKZGProof { proof, commitments } = self.proof;

        let (proof_app_id, proof_tx, proof_x, _): (AppId, Transaction, Data, Data) =
            postcard::from_bytes(&proof.program_io.inputs)?;

        ensure!(&proof_app_id == app_id, "app_id mismatch");
        ensure!(&proof_tx == tx, "tx mismatch");
        ensure!(&proof_x == public_input, "public_input mismatch");

        RV32IJoltVM::verify(preproc, proof, commitments, None).map_err(Into::into)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WrappedVK {
    bytecode: Vec<ELFInstruction>,
    memory_init: Vec<(u64, u8)>,
}

impl TryFrom<&VK> for WrappedVK {
    type Error = anyhow::Error;

    fn try_from(value: &VK) -> std::result::Result<Self, Self::Error> {
        postcard::from_bytes(&value.0).map_err(Into::into)
    }
}

impl WrappedVK {
    pub fn hash(&self) -> &VkHash {
        Hash::hash(&postcard::to_stdvec(self).unwrap()).as_ref()
    }
}
