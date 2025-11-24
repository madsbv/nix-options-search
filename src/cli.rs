use crate::{config::Config, project_paths};
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
    /// Delete existing cache files
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
            Commands::DefaultConfig { write } => default_config(write),
        }
    }
}

fn default_config(write: bool) -> Result<()> {
    let toml = Config::default().to_toml()?;
    println!("{toml}");
    if write {
        // NOTE: The two user confirmations are deliberate to prevent unintended loss of existing config.
        let write_path = project_paths::config_file();
        let warning = format!("Writing default config to {}", write_path.display());
        if user_confirm(&warning)? {
            if let Some(dir) = write_path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            if std::fs::exists(write_path)?
                && !user_confirm(
                    "Configuration file already exists. Replace with default configuration?",
                )?
            {
                // Return without overwriting existing config unless user confirms replacement.
                return Ok(());
            }
            std::fs::write(write_path, toml)?;
        }
    }
    Ok(())
}

fn clear_cache() -> Result<()> {
    let dir = cache_dir();
    let warning = format!("Deleting the following directory: {}", dir.display());
    if user_confirm(&warning)? {
        return delete_cache_dir();
    }
    Ok(())
}

fn user_confirm(warning: &str) -> Result<bool> {
    let warning_message = format!(
        r"{warning}
Press (Y) to confirm or (n) to cancel: "
    );
    print!("{warning_message}");
    std::io::stdout().flush()?;
    let mut answer = String::new();
    loop {
        answer.clear();
        std::io::stdin().read_line(&mut answer)?;
        match answer.as_str() {
            "Y\n" => {
                return Ok(true);
            }

            "n\n" => return Ok(false),
            _ => {
                println!("Unrecognized answer.");
                print!("{warning_message}");
                std::io::stdout().flush()?;
            }
        }
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
