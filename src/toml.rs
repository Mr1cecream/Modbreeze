use crate::errors::BreezeError;
use crate::{Mod, ModSide};
use anyhow::Result;
use libium::config::structs::ModLoader;
use log::{info, warn};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Pack {
    pub name: String,
    pub version: String,
    pub loader: ModLoader,
    pub mc_version: String,
    pub mods: Vec<Mod>,
}

#[derive(Deserialize)]
struct Data {
    name: String,
    version: String,
    loader: String,
    mc_version: String,
    mods: Mods,
}

#[derive(Deserialize)]
struct Mods {
    client: HashMap<String, u32>,
    server: HashMap<String, u32>,
    all: HashMap<String, u32>,
}

impl TryFrom<Data> for Pack {
    type Error = anyhow::Error;

    fn try_from(data: Data) -> Result<Self, Self::Error> {
        let loader: ModLoader = match data.loader.to_lowercase().as_str() {
            "forge" => ModLoader::Forge,
            "fabric" => ModLoader::Fabric,
            "quilt" => ModLoader::Quilt,
            _ => return Err(BreezeError::InvalidLoader.into()),
        };
        info!("Found loader: {}", data.loader.to_lowercase());

        let mut mods: Vec<Mod> = Vec::new();
        convert_mods(&mut mods, data.mods.client, ModSide::Client);
        convert_mods(&mut mods, data.mods.server, ModSide::Server);
        convert_mods(&mut mods, data.mods.all, ModSide::All);

        Ok(Pack {
            name: data.name,
            version: data.version,
            loader,
            mc_version: data.mc_version,
            mods,
        })
    }
}

fn convert_mods(mods: &mut Vec<Mod>, raw: HashMap<String, u32>, side: ModSide) {
    let msg = match side {
        ModSide::All => "general",
        ModSide::Client => "client",
        ModSide::Server => "server",
    };
    let new: Vec<Mod> = raw
        .par_iter()
        .map(|(name, id)| Mod {
            name: name.to_string(),
            id: *id,
            side: side.clone(),
        })
        .filter(|mod_| {
            if mods.contains(mod_) {
                warn!("Found duplicate mod: {}, id: {}", mod_.name, mod_.id);
                false
            } else {
                true
            }
        })
        .collect();
    for mod_ in new {
        info!("Adding {} mod: {}, id: {}", msg, mod_.name, mod_.id);
        mods.push(mod_);
    }
}

/// Parse the given TOML string to a `Pack` struct format
pub fn parse(toml: String) -> Result<Pack> {
    let data: Data = toml::from_str(toml.as_str())?;
    data.try_into()
}
