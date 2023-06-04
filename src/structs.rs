use clap::{ValueEnum, builder::PossibleValue};
use libium::config::structs::ModLoader;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct Mod {
    pub name: String,
    pub id: u32,
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
