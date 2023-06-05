use thiserror::Error;

use crate::structs::ModId;

#[derive(Error, Debug)]
pub enum BreezeError {
    #[error("invalid mod loader specified. valid options are: forge, fabric, quilt")]
    InvalidLoader,
    #[error("no mods in the pack")]
    EmptyPack,
    #[error("couldn't find compatible file for mod {0}, id: {1}")]
    NoCompatFile(String, ModId),
    #[error("distribution denied for mod {0}, id: {1}")]
    DistributionDenied(String, ModId),
}
