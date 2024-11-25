use crate::{AppContractProof, Spell, SpellProof};
use charms_data::{
    nft_state_preserved, token_amounts_balanced, AppId, Data, Transaction, NFT, TOKEN,
};
use sp1_verifier::Groth16Verifier;

pub struct SP1SpellProof<'a> {
    pub vk_bytes32: &'a str,
    pub proof: Option<Box<[u8]>>,
}

impl<'a> SpellProof for SP1SpellProof<'a> {
    fn verify(&self, spell: &Spell) -> bool {
        match &self.proof {
            Some(proof) => Groth16Verifier::verify(
                proof,
                postcard::to_stdvec(spell).unwrap().as_slice(),
                self.vk_bytes32,
                *sp1_verifier::GROTH16_VK_BYTES,
            )
            .is_ok(),
            None => spell.tx.outs.iter().all(|utxo| utxo.charm.is_empty()),
        }
    }
}

pub struct SP1AppContractProof<'a> {
    pub vk_bytes32: &'a str,
    pub proof: Option<Box<[u8]>>,
}

impl<'a> AppContractProof for SP1AppContractProof<'a> {
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
                TOKEN => token_amounts_balanced(app_id, tx),
                NFT => nft_state_preserved(app_id, tx),
                _ => false,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {}
}
