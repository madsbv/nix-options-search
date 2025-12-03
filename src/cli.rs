use crate::config::{default_config_file, default_config_toml, AppConfig, UserConfig};
use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::eyre::Result;
use std::{io::Write, path::PathBuf};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    /// Path to a custom config file
    #[arg(short, long, value_name = "FILE")]
    pub(crate) config: Option<PathBuf>,
    #[arg(short, long, value_name = "FILE")]
    pub(crate) log_file: Option<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Delete existing cache files
    ClearCache,
    /// Print the default configuration for nox
    PrintConfig {
        /// Write the default configuration to the default config location, or the path given to `--config` if set
        #[arg(short, long)]
        write: bool,
        printable_config: Option<PrintableConfig>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default)]
pub(crate) enum PrintableConfig {
    #[default]
    Current,
    Default,
}

impl Commands {
    #[allow(unused_variables)]
    pub(crate) fn run(&self, cli: &Cli, config: &AppConfig) -> Result<()> {
        match self {
            Commands::ClearCache => clear_cache(config),
            Commands::PrintConfig {
                write,
                printable_config,
            } => print_config(*write, *printable_config, config, cli.config.as_ref()),
        }
    }
}

fn print_config(
    write: bool,

    printable_config: Option<PrintableConfig>,
    config: &AppConfig,
    write_path: Option<&PathBuf>,
) -> Result<()> {
    let toml = match printable_config.unwrap_or_default() {
        PrintableConfig::Default => default_config_toml(),
        PrintableConfig::Current => UserConfig::from(config.clone()).to_toml()?,
    };

    println!("{toml}");
    if write {
        // NOTE: The two user confirmations are deliberate to prevent unintended loss of existing config.
        let default_config_file = default_config_file();
        let write_path = write_path.unwrap_or(&default_config_file);
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

fn clear_cache(config: &AppConfig) -> Result<()> {
    let Some(ref dir) = config.cache_dir else {
        println!("Cache directory is unset in your configuration, nothing to clear.");
        return Ok(());
    };
    let warning = format!("Deleting the following directory: {}", dir.display());
    if user_confirm(&warning)? {
        return Ok(std::fs::remove_dir_all(dir)?);
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
