use crate::{app, spell::Spell};
use anyhow::{anyhow, ensure, Result};
use charms_data::{Data, B32};
use std::{
    collections::BTreeMap,
    env, fs, io,
    path::PathBuf,
    process::{Command, Stdio},
};

pub fn new(name: &str) -> Result<()> {
    if !Command::new("which")
        .args(&["cargo-generate"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success()
    {
        Command::new("cargo")
            .args(&["install", "cargo-generate"])
            .stdout(Stdio::null())
            .status()?;
    }
    let status = Command::new("cargo")
        .args(&[
            "generate",
            "--git=https://github.com/sigma0-dev/charms-app",
            "--name",
            name,
        ])
        .status()?;
    ensure!(status.success());
    Ok(())
}

fn do_build() -> Result<String> {
    if !Command::new("which")
        .args(&["cargo-prove"])
        .env(
            "PATH",
            format!("{}:{}/.sp1/bin", env::var("PATH")?, env::var("HOME")?),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success()
    {
        Command::new("bash")
            .args(&["-c", "curl -L https://sp1.succinct.xyz | bash"])
            .stdout(Stdio::null())
            .status()?;
        Command::new(format!("{}/.sp1/bin/sp1up", env::var("HOME")?))
            .args(&["-v", "4.0.1"])
            .stdout(Stdio::null())
            .status()?;
    }
    let mut child = Command::new("cargo")
        .env(
            "PATH",
            format!("{}:{}/.sp1/bin", env::var("PATH")?, env::var("HOME")?),
        )
        .args(&[
            "prove",
            "build",
            "--locked",
            "--output-directory=./target",
            "--elf-name=charms-app",
        ])
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().expect("Failed to open stdout");
    io::copy(&mut io::BufReader::new(stdout), &mut io::stderr())?;
    let status = child.wait()?;
    ensure!(status.success());
    Ok("./target/charms-app".to_string())
}

pub fn build() -> Result<()> {
    let bin_path = do_build()?;
    println!("{}", bin_path);
    Ok(())
}

pub fn vk(path: Option<PathBuf>) -> Result<()> {
    let binary = match path {
        Some(path) => fs::read(path)?,
        None => {
            let bin_path = do_build()?;
            fs::read(bin_path)?
        }
    };
    let prover = app::Prover::new();
    let vk: [u8; 32] = prover.vk(&binary);

    println!("{}", hex::encode(&vk));
    Ok(())
}

pub fn run(spell: PathBuf, path: Option<PathBuf>) -> Result<()> {
    let binary = match path {
        Some(path) => fs::read(path)?,
        None => {
            let bin_path = do_build()?;
            fs::read(bin_path)?
        }
    };
    let prover = app::Prover::new();
    let vk = B32(prover.vk(&binary));

    let spell: Spell = serde_yaml::from_slice(
        &fs::read(&spell).map_err(|e| anyhow!("error reading {:?}: {}", &spell, e))?,
    )?;
    let tx = spell.to_tx()?;

    let public_inputs = spell.public_inputs.unwrap_or_default();
    let private_inputs = spell.private_inputs.unwrap_or_default();

    let mut app_present = false;
    for (k, app) in spell.apps.iter().filter(|(_, app)| app.vk == vk) {
        app_present = true;
        let x = data_for_key(&public_inputs, k);
        let w = data_for_key(&private_inputs, k);
        prover.run(&binary, app, &tx, &x, &w)?;
        eprintln!("✅  satisfied app contract for: {}", app);
    }
    if !app_present {
        eprintln!("⚠️  app not present for VK: {}", vk);
    }

    Ok(())
}

fn data_for_key(inputs: &BTreeMap<String, Data>, k: &String) -> Data {
    match inputs.get(k) {
        Some(v) => v.clone(),
        None => Data::empty(),
    }
}
