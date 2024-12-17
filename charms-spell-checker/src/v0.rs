use crate::AppContractProof;
use charms_data::{
    nft_state_preserved, token_amounts_balanced, App, Data, Transaction, NFT, TOKEN,
};
use serde::{Deserialize, Serialize};
use sp1_primitives::io::SP1PublicValues;
use sp1_zkvm::lib::verify::verify_sp1_proof;

fn to_public_values<T: Serialize>(t: &T) -> SP1PublicValues {
    let mut pv = SP1PublicValues::new();
    pv.write(t);
    pv
}

#[derive(Serialize, Deserialize)]
pub struct V0AppContractProof {
    pub vk: Option<[u32; 8]>,
}

impl AppContractProof for V0AppContractProof {
    fn verify(&self, app: &App, tx: &Transaction, x: &Data) -> bool {
        match &self.vk {
            Some(vk) => {
                let Ok(pv) = to_public_values(&(app, tx, x)).hash().try_into() else {
                    unreachable!()
                };
                verify_sp1_proof(vk, &pv);
                true
            }
            None => match app.tag {
                TOKEN => token_amounts_balanced(app, &tx),
                NFT => nft_state_preserved(app, &tx),
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
