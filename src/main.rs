use anyhow::Result;
use clap::{ValueEnum, builder::PossibleValue};
use config::Config;
use serde::{Deserialize, Serialize};
use std::{
    io::prelude::Write,
    path::{Path, PathBuf},
};

mod cli;
mod config;
mod download;
mod errors;
mod toml;

#[derive(Debug, Clone)]
pub struct Mod {
    name: String,
    id: u32,
    side: ModSide,
    ignore_loader: bool,
    ignore_version: bool,
}

impl PartialEq for Mod {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum ModSide {
    Client,
    Server,
    All,
    Resourcepack,
    Shaderpack,
}

impl ValueEnum for ModSide {
    fn value_variants<'a>() -> &'a [Self] {
        &[ModSide::Client, ModSide::Server, ModSide::All]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match &self {
            &ModSide::All => Some(PossibleValue::new("All").aliases(["a", "common"])),
            &ModSide::Client => Some(PossibleValue::new("Client").alias("c")),
            &ModSide::Server => Some(PossibleValue::new("Server").alias("s")),
            _ => None,
        }
    }
}

#[tokio::main]
async fn main() {
    let res = actual_main().await;
    if let Err(e) = res {
        eprintln!("{}", e);
    }
}

async fn actual_main() -> Result<()> {
    let config_path = get_config_file_path()?;
    setup_logging(&config_path, false)?;
    let mut config = if Path::new(&config_path).exists() {
        load_config(&config_path)?
    } else {
        Default::default()
    };

    cli::cli(&mut config).await?;

    save_config(config, &config_path)?;
    Ok(())
}

fn setup_logging(path: &Path, verbose: bool) -> Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(if let Some(parent) = path.parent() {
                    parent.join("latest.log")
                } else {
                    Path::new("latest.log").to_path_buf()
                })?,
        )
        .chain(
            fern::Dispatch::new()
                .level(if verbose {
                    log::LevelFilter::Info
                } else {
                    log::LevelFilter::Warn
                })
                .chain(std::io::stdout()),
        )
        .apply()?;
    Ok(())
}

fn load_config(config_path: &Path) -> Result<Config> {
    let json = std::fs::read_to_string(config_path)?;
    let config = serde_json::from_str(&json)?;
    Ok(config)
}

fn save_config(config: Config, config_path: &Path) -> Result<()> {
    let ser = serde_json::to_string(&config)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(config_path)?;
    write!(file, "{}", ser)?;

    Ok(())
}

fn get_config_file_path() -> Result<PathBuf> {
    if let Ok(ok) = std::env::var("MODBREEZE_CONFIG_PATH") {
        Ok(Path::new(&ok).to_path_buf())
    } else {
        Ok(Path::new(&std::env::current_exe()?)
            .parent()
            .unwrap_or(&dirs::config_dir().unwrap().join("modbreeze"))
            .join("config.json"))
    }
}
