use crate::{AppContractProof, SpellData, SpellProof, TransactionData};
use charms_data::{
    nft_state_preserved, token_amounts_balanced, AppId, Data, Transaction, NFT, TOKEN,
};
use serde::{Deserialize, Serialize};
use sp1_verifier::Groth16Verifier;

#[derive(Serialize, Deserialize)]
pub struct V0SpellProof<'a> {
    pub vk_bytes32: &'a str,
    pub proof: Option<Box<[u8]>>,
}

impl<'a> SpellProof for V0SpellProof<'a> {
    fn verify(&self, spell: &SpellData) -> bool {
        match &self.proof {
            Some(proof) => Groth16Verifier::verify(
                proof,
                postcard::to_stdvec(spell).unwrap().as_slice(),
                self.vk_bytes32,
                *sp1_verifier::GROTH16_VK_BYTES,
            )
            .is_ok(),
            None => spell.tx.outs.iter().all(|charm| charm.is_empty()),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct V0AppContractProof<'a> {
    pub vk_bytes32: &'a str,
    pub proof: Option<Box<[u8]>>,
}

impl<'a> AppContractProof for V0AppContractProof<'a> {
    fn verify(&self, app_id: &AppId, tx: &Transaction, x: &Data) -> bool {
        match &self.proof {
            Some(proof) => Groth16Verifier::verify(
                proof,
                postcard::to_stdvec(&(app_id, tx, x)).unwrap().as_slice(),
                self.vk_bytes32,
                *sp1_verifier::GROTH16_VK_BYTES,
            )
            .is_ok(),
            None => match app_id.tag {
                TOKEN => token_amounts_balanced(app_id, &tx),
                NFT => nft_state_preserved(app_id, &tx),
                _ => false,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn dummy() {}
}
