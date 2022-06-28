use crate::{
    config::{Config, PathOrUrl},
    download, ModSide,
};
use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;
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
            if let Some(file) = file {
                config.source = Some(PathOrUrl::Path(fs::canonicalize(file)?));
            } else if let Some(url) = url {
                config.source = Some(PathOrUrl::Url(url));
            } else {
                return Err(CliError::NoSourceSpecified.into());
            }
            info!("Setting source to {:?}", config.source);
        }
        Commands::Config { dir } => {
            if !dir.exists() {
                tokio::fs::create_dir_all(&dir).await?;
            }
            config.out_dir = Some(fs::canonicalize(dir)?);
        }
        Commands::Upgrade {
            side,
            file,
            url,
            dir,
        } => {
            // Get TOML source
            let source: PathOrUrl = if let Some(file) = file {
                config.source = Some(PathOrUrl::Path(fs::canonicalize(&file)?));
                PathOrUrl::Path(file)
            } else if let Some(url) = url {
                config.source = Some(PathOrUrl::Url(url.clone()));
                PathOrUrl::Url(url)
            } else {
                if let Some(source) = config.source.clone() {
                    source
                } else {
                    return Err(CliError::NoSourceSpecified.into());
                }
            };
            // Get Minecraft directory
            let mod_dir = if let Some(dir) = dir {
                if !dir.exists() {
                    tokio::fs::create_dir_all(&dir).await?;
                }
                config.out_dir = Some(fs::canonicalize(&dir)?);
                dir
            } else {
                if let Some(mod_dir) = config.out_dir.clone() {
                    mod_dir
                } else {
                    return Err(CliError::NoModDirSpecified.into());
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
            download::clean(&mod_dir, &mut to_download).await?;
            download::download(Arc::new(mod_dir), to_download).await?;
        }
    };
    Ok(())
}

#[derive(Error, Debug)]
enum CliError {
    #[error("no file or path was specified")]
    NoSourceSpecified,
    #[error("no Minecraft directory was specified")]
    NoModDirSpecified,
    #[error("expected plain text from URL response, got {0}. check the specified URL")]
    NonPlainTextResponse(String),
}
