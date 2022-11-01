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
    client: Option<HashMap<String, TomlMod>>,
    server: Option<HashMap<String, TomlMod>>,
    common: Option<HashMap<String, TomlMod>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TomlMod {
    Id(u32),
    Tabled {
        id: u32,
        ignore_loader: Option<bool>,
        ignore_version: Option<bool>,
    },
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
        convert_mods(&mut mods, data.mods.common, ModSide::All);

        if mods.is_empty() {
            return Err(BreezeError::EmptyPack.into());
        }

        Ok(Pack {
            name: data.name,
            version: data.version,
            loader,
            mc_version: data.mc_version,
            mods,
        })
    }
}

fn convert_mods(mods: &mut Vec<Mod>, raw: Option<HashMap<String, TomlMod>>, side: ModSide) {
    if raw.is_none() {
        return;
    }
    let raw = raw.unwrap();
    let msg = match side {
        ModSide::All => "common",
        ModSide::Client => "client",
        ModSide::Server => "server",
    };
    let new: Vec<Mod> = raw
        .par_iter()
        .map(|(name, id)| {
            let name = name.to_string();
            let side = side.clone();
            let (id, ignore_loader, ignore_version) = match id {
                TomlMod::Id(id) => (id, false, false),
                TomlMod::Tabled {
                    id,
                    ignore_loader,
                    ignore_version,
                } => (
                    id,
                    ignore_loader.unwrap_or(false),
                    ignore_version.unwrap_or(false),
                ),
            };
            Mod {
                name,
                id: *id,
                side,
                ignore_loader,
                ignore_version,
            }
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
