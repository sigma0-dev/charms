use sp1_sdk::{HashableKey, ProverClient};
use std::sync::LazyLock;

pub mod app;
mod script;
pub mod spell;
pub mod tx;

pub const SPELL_CHECKER_BINARY: &[u8] =
    include_bytes!("../spell-prover/elf/riscv32im-succinct-zkvm-elf");

pub static SPELL_VK: LazyLock<String> = LazyLock::new(|| {
    let client = ProverClient::new();
    let (_, vk) = client.setup(SPELL_CHECKER_BINARY);
    eprintln!("spell vk: {}", vk.bytes32());
    vk.bytes32()
});
