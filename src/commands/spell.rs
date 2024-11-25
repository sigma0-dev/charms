use anyhow::Result;
use charms::spell::CompactSpell;

pub fn spell_parse() -> Result<()> {
    let spell: CompactSpell = serde_yaml::from_reader(std::io::stdin())?;
    ciborium::into_writer(&spell, std::io::stdout())?;

    Ok(())
}

pub fn spell_print() -> Result<()> {
    let spell: CompactSpell = ciborium::de::from_reader(std::io::stdin())?;
    serde_yaml::to_writer(std::io::stdout(), &spell)?;

    Ok(())
}
