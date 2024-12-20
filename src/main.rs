extern crate core;

mod commands;

use anyhow::Result;
use clap::Parser;
use commands::*;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Spell { command } => match command {
            SpellCommands::Parse => spell::spell_parse(),
            SpellCommands::Print => spell::spell_print(),
            SpellCommands::Prove { .. } => spell::spell_prove(command),
        },
        Commands::Tx { command } => match command {
            TxCommands::AddSpell { .. } => tx::tx_add_spell(command),
            TxCommands::ShowSpell { tx } => tx::tx_show_spell(tx),
        },
        Commands::App { command } => match command {
            AppCommands::Vk { path } => app::vk(path),
        },
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn dummy() {}
}
