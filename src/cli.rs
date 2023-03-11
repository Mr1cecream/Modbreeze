use crate::{
    config::{Config, PathOrUrl},
    download, ModSide,
};
use anyhow::Result;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use promptly::prompt;
use reqwest::header::CONTENT_TYPE;
use std::{fs, path::PathBuf, sync::Arc, time::Duration};
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
        /// Minecraft root directory
        #[clap(short, long, value_parser, value_name = "DIR")]
        dir: Option<PathBuf>,
        /// Which types of mods to download
        #[clap(
            short,
            long,
            value_parser,
            value_enum,
            ignore_case = true,
            value_name = "SIDE"
        )]
        side: Option<ModSide>,
    },
    /// Upgrade mods
    Upgrade {
        /// Which types of mods to download
        #[clap(
            short,
            long,
            value_parser,
            value_enum,
            ignore_case = true,
            value_name = "SIDE"
        )]
        side: Option<ModSide>,
        /// TOML file with modpack definition
        #[clap(short, long, value_parser, value_name = "FILE")]
        file: Option<PathBuf>,
        /// URL to TOML with modpack definition
        #[clap(short, long, value_parser, value_name = "URL")]
        url: Option<Url>,
        /// Minecraft root directory
        #[clap(short, long, value_parser, value_name = "DIR")]
        dir: Option<PathBuf>,
        /// Whether to download resourcepacks
        #[clap(long)]
        resourcepacks: bool,
        /// Whether to download shaderpacks
        #[clap(long)]
        shaderpacks: bool,
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
        Commands::Config { dir, side } => {
            if let Some(dir) = dir {
                tokio::fs::create_dir_all(&dir).await?;
                config.mc_dir = Some(fs::canonicalize(dir)?);
            }
            if let Some(side) = side {
                config.side = Some(side);
            }
        }
        Commands::Upgrade {
            side,
            file,
            url,
            dir,
            resourcepacks,
            shaderpacks,
        } => {
            // TODO: remove when Customization support is to the CurseForge API
            if shaderpacks {
                warn!("Shaderpacks are currently unsupported by Modbreeze.");
            }
            let shaderpacks = false;

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
                tokio::fs::create_dir_all(&dir).await?;
                config.mc_dir = Some(fs::canonicalize(&dir)?);
                dir
            } else {
                if let Some(mc_dir) = config.mc_dir.clone() {
                    info!("Found Minecraft Directory in Config: {:?}", mc_dir);
                    mc_dir
                } else {
                    let dir: PathBuf = prompt("Minecraft Root Directory")?;
                    info!("Setting Minecraft Directory: {:?}", dir);

                    tokio::fs::create_dir_all(&dir).await?;
                    config.mc_dir = Some(fs::canonicalize(&dir)?);
                    dir
                }
            };
            let side = if let Some(side) = side {
                config.side = Some(side);
                side
            } else {
                config.side.unwrap_or(ModSide::Client)
            };

            let progress_bar = create_spinner("Parsing pack", "Finished parsing pack.");
            // Get TOML contents
            let toml = match source {
                PathOrUrl::Path(path) => std::fs::read_to_string(path)?,
                PathOrUrl::Url(url) => {
                    let resp = reqwest::get(url.as_str()).await?;
                    let content_type = resp.headers().get(CONTENT_TYPE);
                    if let Some(ct) = content_type {
                        if let Ok(ct) = ct.to_str() {
                            if !ct.contains("text/plain") {
                                return Err(CliError::NonPlainTextResponse(ct.to_string()).into());
                            }
                        }
                    }
                    resp.text().await?
                }
            };
            let pack = crate::toml::parse(toml)?;
            progress_bar.finish();

            let progress_bar = create_spinner("Fetching mods", "Finished fetching mods.");
            let mut to_download =
                download::get_downloadables(side, resourcepacks, shaderpacks, pack).await?;
            progress_bar.finish();

            let progress_bar = create_spinner("Cleaning old mods", "Finished cleaning old mods.");
            download::clean(&mc_dir.join("mods"), &mut to_download, true).await?;
            download::clean(&mc_dir.join("resourcepacks"), &mut to_download, false).await?;
            download::clean(&mc_dir.join("shaderpacks"), &mut to_download, false).await?;
            progress_bar.finish();

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

fn create_spinner(msg: &str, finish: &str) -> ProgressBar {
    let progress_bar = ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template("{spinner:.green}")
            .unwrap()
            .tick_strings(&[
                format!("{}{}", msg, ".").as_str(),
                format!("{}{}", msg, "..").as_str(),
                format!("{}{}", msg, "...").as_str(),
                finish,
            ]),
    );
    progress_bar.enable_steady_tick(Duration::from_millis(300));
    progress_bar
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
