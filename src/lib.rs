pub mod app;
pub mod cli;
pub mod script;
pub mod spell;
pub mod tx;

pub const SPELL_CHECKER_BINARY: &[u8] = include_bytes!("./bin/charms-spell-checker");

pub const SPELL_VK: &str = "0x0001e3ce998c2201c9d85ae6d0b0713385e85edf0bf3574ceadccd261a8cb9e5";

#[cfg(test)]
mod test {
    use super::*;
    use sp1_sdk::{HashableKey, ProverClient};

    #[test]
    fn test_sha1() {
        let client = ProverClient::new();
        let (_, vk) = client.setup(SPELL_CHECKER_BINARY);
        let s = vk.bytes32();

        assert_eq!(SPELL_VK, s.as_str(),);
    }
}
