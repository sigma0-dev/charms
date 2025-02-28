use charms_data::{is_simple_transfer, util, App, Data, Transaction};
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
            None => is_simple_transfer(app, tx),
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
