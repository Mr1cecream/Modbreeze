use crate::errors::BreezeError;
use crate::{Mod, ModSide};
use anyhow::Result;
use libium::config::structs::ModLoader;
use log::{info, warn};
use serde::Deserialize;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Pack {
    pub name: String,
    pub version: String,
    pub loader: ModLoader,
    pub mc_version: String,
    pub mods: Vec<Mod>,
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

/// Parse the given TOML string to a `Pack` struct format
pub fn parse(toml: String) -> Result<Pack> {
    let data: Data = toml::from_str(toml.as_str())?;

    let loader: ModLoader = match data.loader.to_lowercase().as_str() {
        "forge" => ModLoader::Forge,
        "fabric" => ModLoader::Fabric,
        _ => return Err(BreezeError::InvalidLoader.into()),
    };
    info!("Found loader: {}", data.loader.to_lowercase());

    let mut mods: Vec<Mod> = Vec::new();
    // has to be a better way to do this
    for (name, id) in data.mods.client.iter() {
        let mod_ = Mod {
            name: name.to_string(),
            id: *id,
            side: ModSide::Client,
        };
        if mods.contains(&mod_) {
            warn!("Found duplicate mod: {}, id: {}", name, id);
            continue;
        }
        mods.push(mod_);
        info!("Adding client mod: {}, id: {}", name, id);
    }
    for (name, id) in data.mods.server.iter() {
        let mod_ = Mod {
            name: name.to_string(),
            id: *id,
            side: ModSide::Server,
        };
        if mods.contains(&mod_) {
            warn!("Found duplicate mod: {}, id: {}", name, id);
            continue;
        }
        mods.push(mod_);
        info!("Adding server mod: {}, id: {}", name, id);
    }
    for (name, id) in data.mods.all.iter() {
        let mod_ = Mod {
            name: name.to_string(),
            id: *id,
            side: ModSide::All,
        };
        if mods.contains(&mod_) {
            warn!("Found duplicate mod: {}, id: {}", name, id);
            continue;
        }
        mods.push(mod_);
        info!("Adding general mod: {}, id: {}", name, id);
    }

    Ok(Pack {
        name: data.name,
        version: data.version,
        loader,
        mc_version: data.mc_version,
        mods,
    })
}
