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
        },
        Commands::Tx { command } => match command {
            command @ TxCommands::AddSpell { .. } => tx::tx_add_spell(command),
        },
    }
}

#[cfg(test)]
mod tests {}
