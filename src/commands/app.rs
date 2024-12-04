use anyhow::Result;
use sp1_sdk::{HashableKey, ProverClient};
use std::{fs, mem};

pub(crate) fn vk(path: String) -> Result<()> {
    let client = ProverClient::new();
    let binary = fs::read(path)?;
    let (_pk, vk) = client.setup(&binary);
    let vk: [u8; 32] = unsafe {
        let vk: [u32; 8] = vk.hash_u32();
        mem::transmute(vk)
    };
    println!("{}", hex::encode(&vk));
    Ok(())
}
