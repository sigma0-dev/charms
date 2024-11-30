use crate::{AppContractProof, SpellData, SpellProof};
use charms_data::{
    nft_state_preserved, token_amounts_balanced, AppId, Data, Transaction, NFT, TOKEN,
};
use serde::{Deserialize, Serialize};
use sp1_primitives::io::SP1PublicValues;
use sp1_verifier::Groth16Verifier;

#[derive(Serialize, Deserialize)]
pub struct V0SpellProof {
    pub vk_bytes32: String,
    pub proof: Option<Box<[u8]>>,
}

fn to_public_values<T: Serialize>(t: &T) -> SP1PublicValues {
    let mut pv = SP1PublicValues::new();
    pv.write(t);
    pv
}

impl<'a> SpellProof for V0SpellProof {
    fn verify(&self, spell: &SpellData) -> bool {
        match &self.proof {
            Some(proof) => Groth16Verifier::verify(
                proof,
                to_public_values(&(&self.vk_bytes32, spell)).as_slice(),
                &self.vk_bytes32,
                *sp1_verifier::GROTH16_VK_BYTES,
            )
            .is_ok(),
            None => spell.tx.outs.iter().all(|charm| charm.is_empty()),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct V0AppContractProof {
    pub vk_bytes32: Option<String>,
    pub proof: Option<Box<[u8]>>,
}

impl AppContractProof for V0AppContractProof {
    fn verify(&self, app_id: &AppId, tx: &Transaction, x: &Data) -> bool {
        match &self.proof {
            Some(proof) => {
                let Some(vk_bytes32) = &self.vk_bytes32 else {
                    unreachable!()
                };
                Groth16Verifier::verify(
                    proof,
                    to_public_values(&(app_id, tx, x)).as_slice(),
                    vk_bytes32,
                    *sp1_verifier::GROTH16_VK_BYTES,
                )
                .is_ok()
            }
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
    #[test]
    fn dummy() {}
}
