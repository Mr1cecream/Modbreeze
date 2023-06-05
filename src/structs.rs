use clap::{builder::PossibleValue, ValueEnum};
use libium::config::structs::ModLoader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Mod {
    pub name: String,
    pub id: ModId,
    pub side: ModSide,
    pub ignore_loader: bool,
    pub ignore_version: bool,
}

impl PartialEq for Mod {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
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

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
#[serde(untagged)]
pub enum ModId {
    /// CurseForge ProjectID
    CurseForgeId(u32),
    /// Modrinth ProjectID
    ModrinthId(String),
}

impl std::fmt::Display for ModId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::CurseForgeId(id) => format!("[CurseForge]{id}"),
            Self::ModrinthId(id) => format!("[Modrinth]{id}"),
        };
        write!(f, "{}", str)
    }
}

#[derive(Debug)]
pub struct Pack {
    pub name: String,
    pub version: String,
    pub loader: ModLoader,
    pub mc_version: String,
    pub mods: Vec<Mod>,
    pub resourcepacks: Vec<Mod>,
    pub shaderpacks: Vec<Mod>,
}
