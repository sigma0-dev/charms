use anyhow::Result;
use charms::app;
use std::fs;

pub fn vk(path: String) -> Result<()> {
    let prover = app::Prover::new();

    let binary = fs::read(path)?;
    let vk: [u8; 32] = prover.vk(&binary);

    println!("{}", hex::encode(&vk));
    Ok(())
}
