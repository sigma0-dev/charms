use crate::app;
use anyhow::{ensure, Result};
use std::{
    env, fs, io,
    process::{Command, Stdio},
};

pub fn new(name: &str) -> Result<()> {
    if !Command::new("which")
        .args(&["cargo-generate"])
        .stdout(Stdio::null())
        .status()?
        .success()
    {
        Command::new("cargo")
            .args(&["install", "cargo-generate"])
            .status()?;
    }
    let status = Command::new("cargo")
        .args(&["generate", "sigma0-dev/charms-app", "--name", name])
        .status()?;
    ensure!(status.success());
    Ok(())
}

pub fn build() -> Result<()> {
    if !Command::new("which")
        .args(&["cargo-prove"])
        .stdout(Stdio::null())
        .status()?
        .success()
    {
        Command::new("bash")
            .args(&["-c", "curl -L https://sp1.succinct.xyz | bash"])
            .stdout(Stdio::null())
            .status()?;
        Command::new(format!("{}/.sp1/bin/sp1up", env::var("HOME")?))
            .stdout(Stdio::null())
            .status()?;
    }
    let mut child = Command::new("cargo")
        .args(&["prove", "build"])
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().expect("Failed to open stdout");
    io::copy(&mut io::BufReader::new(stdout), &mut io::stderr())?;
    let status = child.wait()?;
    ensure!(status.success());
    Ok(())
}

pub fn vk(path: Option<String>) -> Result<()> {
    let prover = app::Prover::new();

    let binary = match path {
        Some(path) => fs::read(path)?,
        None => {
            build()?;
            fs::read("./elf/riscv32im-succinct-zkvm-elf")?
        }
    };
    let vk: [u8; 32] = prover.vk(&binary);

    println!("{}", hex::encode(&vk));
    Ok(())
}
