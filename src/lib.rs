extern crate core;

pub mod app;
mod script;
pub mod spell;
pub mod tx;

pub const SPELL_CHECKER_BINARY: &[u8] = include_bytes!("./bin/charms-spell-checker");

pub const SPELL_VK: &str = "0x00715b1076f4d23a8a37cdb298df9018a1cf72e740fdacea324af87faf7dd162";

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
