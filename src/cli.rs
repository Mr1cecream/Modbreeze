use crate::{
    config::{Config, PathOrUrl},
    download, ModSide,
};
use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;
use promptly::prompt;
use std::{fs, path::PathBuf, sync::Arc};
use thiserror::Error;
use url::Url;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Source a file or url as a modpack
    Source {
        /// TOML file with modpack definition
        #[clap(short, long, value_parser, conflicts_with = "url", value_name = "FILE")]
        file: Option<PathBuf>,
        /// URL to TOML with modpack definition
        #[clap(short, long, value_parser, conflicts_with = "file", value_name = "URL")]
        url: Option<Url>,
    },
    /// Configure the CLI
    Config {
        /// Minecraft directory
        #[clap(short, long, value_parser, value_name = "DIR")]
        dir: PathBuf,
    },
    /// Upgrade mods
    Upgrade {
        #[clap(short, long, value_parser, arg_enum, value_name = "SIDE")]
        side: ModSide,
        /// TOML file with modpack definition
        #[clap(short, long, value_parser, value_name = "FILE")]
        file: Option<PathBuf>,
        /// URL to TOML with modpack definition
        #[clap(short, long, value_parser, value_name = "URL")]
        url: Option<Url>,
        /// Minecraft directory
        #[clap(short, long, value_parser, value_name = "DIR")]
        dir: Option<PathBuf>,
    },
}

pub async fn cli(config: &mut Config) -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Source { file, url } => {
            if let Some(source) = get_source(file, url)? {
                config.source = Some(source);
            } else {
                return Err(CliError::NoSourceSpecified.into());
            }
            info!("Setting source to {:?}", config.source);
        }
        Commands::Config { dir } => {
            if !dir.exists() {
                tokio::fs::create_dir_all(&dir).await?;
            }
            config.mc_dir = Some(fs::canonicalize(dir)?);
        }
        Commands::Upgrade {
            side,
            file,
            url,
            dir,
        } => {
            // Get TOML source
            let source: PathOrUrl = if let Some(source) = get_source(file, url)? {
                config.source = Some(source.clone());
                source
            } else {
                if let Some(source) = config.source.clone() {
                    source
                } else {
                    return Err(CliError::NoSourceSpecified.into());
                }
            };
            // Get Minecraft directory
            let mc_dir = if let Some(dir) = dir {
                info!("Setting Minecraft Directory: {:?}", dir);
                if !dir.exists() {
                    tokio::fs::create_dir_all(&dir).await?;
                }
                config.mc_dir = Some(fs::canonicalize(&dir)?);
                dir
            } else {
                if let Some(mc_dir) = config.mc_dir.clone() {
                    info!("Found Minecraft Directory in Config: {:?}", mc_dir);
                    mc_dir
                } else {
                    let dir: PathBuf = prompt("Minecraft Directory")?;
                    info!("Setting Minecraft Directory: {:?}", dir);

                    if !dir.exists() {
                        tokio::fs::create_dir_all(&dir).await?;
                    }
                    config.mc_dir = Some(fs::canonicalize(&dir)?);
                    dir
                }
            };
            // Get TOML contents
            let toml = match source {
                PathOrUrl::Path(path) => std::fs::read_to_string(path)?,
                PathOrUrl::Url(url) => {
                    let resp = ureq::get(url.as_str()).call()?;
                    match resp.content_type() {
                        "text/plain" => resp.into_string()?,
                        ct => return Err(CliError::NonPlainTextResponse(ct.to_string()).into()),
                    }
                }
            };
            let pack = crate::toml::parse(toml)?;
            let mut to_download = download::get_downloadables(side, pack).await?;
            download::clean(&mc_dir.join("mods"), &mut to_download).await?;
            download::download(Arc::new(mc_dir), to_download).await?;
        }
    };
    Ok(())
}

#[derive(Error, Debug)]
enum CliError {
    #[error("no file or path was specified")]
    NoSourceSpecified,
    // #[error("no Minecraft directory was specified")]
    // NoModDirSpecified,
    #[error("expected plain text from URL response, got {0}. check the specified URL")]
    NonPlainTextResponse(String),
}

fn get_source(file: Option<PathBuf>, url: Option<Url>) -> Result<Option<PathOrUrl>> {
    if let Some(file) = file {
        Ok(Some(PathOrUrl::Path(fs::canonicalize(&file)?)))
    } else if let Some(url) = url {
        Ok(Some(PathOrUrl::Url(url)))
    } else {
        Ok(None)
    }
}
