use anyhow::Result;
use charms::spell::Spell;

pub fn spell_parse() -> Result<()> {
    let spell: Spell = serde_yaml::from_reader(std::io::stdin())?;
    ciborium::into_writer(&spell, std::io::stdout())?;

    Ok(())
}

pub fn spell_print() -> Result<()> {
    let spell: Spell = ciborium::de::from_reader(std::io::stdin())?;
    serde_yaml::to_writer(std::io::stdout(), &spell)?;

    Ok(())
}
