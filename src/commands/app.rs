use anyhow::Result;
use charms::app;
use std::fs;

pub fn vk(path: Option<String>) -> Result<()> {
    let prover = app::Prover::new();

    let binary = match path {
        Some(path) => fs::read(path)?,
        None => {
            let mut child = std::process::Command::new("cargo")
                .args(&["prove", "build"])
                .stdout(std::process::Stdio::piped())
                .spawn()?;
            let stdout = child.stdout.take().expect("Failed to open stdout");
            std::io::copy(&mut std::io::BufReader::new(stdout), &mut std::io::stderr())?;
            let status = child.wait()?;
            assert!(status.success());
            fs::read("./elf/riscv32im-succinct-zkvm-elf")?
        }
    };
    let vk: [u8; 32] = prover.vk(&binary);

    println!("{}", hex::encode(&vk));
    Ok(())
}
