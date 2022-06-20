use crate::errors::{BreezeError, BreezeErrorKind};
use crate::{Loader, Mod, ModSide};
use serde::Deserialize;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Pack {
    name: String,
    version: String,
    loader: Loader,
    mc_version: String,
    mods: Vec<Mod>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Data {
    name: String,
    version: String,
    loader: String,
    mc_version: String,
    mods: Mods,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Mods {
    client: HashMap<String, u32>,
    server: HashMap<String, u32>,
    all: HashMap<String, u32>,
}

pub fn parse(toml: String) -> Result<Pack, Box<dyn std::error::Error>> {
    let data: Data = toml::from_str(toml.as_str())?;
    let loader: Loader = match data.loader.to_lowercase().as_str() {
        "forge" => Loader::Forge,
        "fabric" => Loader::Fabric,
        _ => return Err(Box::new(BreezeError::new(BreezeErrorKind::InvalidLoader))),
    };

    let mut mods: Vec<Mod> = Vec::new();

    for id in data.mods.client.values() {
        mods.push(Mod {
            id: *id,
            side: ModSide::Client,
        })
    }
    for id in data.mods.server.values() {
        mods.push(Mod {
            id: *id,
            side: ModSide::Server,
        })
    }
    for id in data.mods.all.values() {
        mods.push(Mod {
            id: *id,
            side: ModSide::All,
        })
    }

    Ok(Pack {
        name: data.name,
        version: data.version,
        loader,
        mc_version: data.mc_version,
        mods,
    })
}
