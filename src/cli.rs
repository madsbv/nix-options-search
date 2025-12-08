use crate::config::Config;
use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use std::{io::Write, path::PathBuf};

use crate::{cache::delete_cache_dir, project_paths::cache_dir};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    /// Path to a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    ClearCache,
    /// Print the default configuration for nox
    DefaultConfig {
        /// Write the default configuration to the default config location, or the path given to `--config` if set
        #[arg(short, long)]
        write: bool,
    },
}

impl Commands {
    pub(crate) fn run(self) -> Result<()> {
        match self {
            Commands::ClearCache => clear_cache(),
            Commands::DefaultConfig { write } => {
                println!("{:?}", Config::get());
                println!("{write}");
                Ok(())
            }
        }
    }
}

fn clear_cache() -> Result<()> {
    let dir = cache_dir();
    let warning_message = format!(
        r"The following directory will be deleted: {}
Press (Y) to confirm or (n) to cancel: ",
        dir.display()
    );
    print!("{warning_message}");
    std::io::stdout().flush()?;
    let mut answer = String::new();
    loop {
        answer.clear();
        std::io::stdin().read_line(&mut answer)?;
        match answer.as_str() {
            "Y\n" => {
                delete_cache_dir()?;
                break;
            }

            "n\n" => break,
            _ => {
                println!("Unrecognized answer.");
                print!("{warning_message}");
                std::io::stdout().flush()?;
            }
        }
    }
    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
