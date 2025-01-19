use charms_data::{
    nft_state_preserved, token_amounts_balanced, util, App, Data, Transaction, NFT, TOKEN,
};
use serde::{Deserialize, Serialize};
use sp1_primitives::io::SP1PublicValues;
use sp1_zkvm::lib::verify::verify_sp1_proof;

fn to_public_values<T: Serialize>(t: &T) -> SP1PublicValues {
    SP1PublicValues::from(
        util::write(t)
            .expect("(app, tx, x) should serialize successfully")
            .as_slice(),
    )
}

#[derive(Serialize, Deserialize)]
pub(crate) struct AppContractVK {
    pub vk: Option<[u32; 8]>,
}

impl AppContractVK {
    pub(crate) fn verify(&self, app: &App, tx: &Transaction, x: &Data) -> bool {
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
