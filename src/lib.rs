pub mod app;
pub mod cli;
pub mod script;
pub mod spell;
pub mod tx;
pub mod utils;

pub const SPELL_CHECKER_BINARY: &[u8] = include_bytes!("./bin/charms-spell-checker");

pub const SPELL_VK: &str = "0x0061d4c45041155bfb34ee39a01d47e21cc56baf69e4be25f1320d36eb19ff6c";

#[cfg(test)]
mod test {
    use super::*;
    use sp1_sdk::{HashableKey, ProverClient};

    #[test]
    fn test_spell_vk() {
        let client = ProverClient::from_env();
        let (_, vk) = client.setup(SPELL_CHECKER_BINARY);
        let s = vk.bytes32();

        assert_eq!(SPELL_VK, s.as_str(),);
    }
}
