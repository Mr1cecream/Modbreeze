use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

mod cli;
mod config;
mod download;
mod errors;
mod toml;

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub struct Mod {
    name: String,
    id: u32,
    side: ModSide,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub enum ModSide {
    Client,
    Server,
    All,
}

#[allow(dead_code)]
#[derive(Debug)]
enum Loader {
    Fabric,
    Forge,
}

#[tokio::main]
async fn main() {
    let res = actual_main().await;
    if let Err(e) = res {
        println!("{}", e);
    }
}

async fn actual_main() -> Result<()> {
    setup_logging(false)?;

    let example_pack = std::fs::read_to_string("example_pack.toml")?;
    let pack = toml::parse(example_pack)?;
    let config = config::Config {
        mod_dir: PathBuf::from("/home/guy/dev/testing/modbreeze"),
        last_pack: None,
    };
    let to_download = download::get_downloadables(ModSide::All, pack).await?;
    download::download(Arc::new(config.mod_dir), to_download).await?;

    Ok(())
}

fn setup_logging(verbose: bool) -> Result<()> {
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
                .open("latest.log")?,
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
