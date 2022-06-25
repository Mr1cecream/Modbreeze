use crate::toml::Pack;
use std::path::PathBuf;

pub struct Config {
    pub mod_dir: PathBuf,
    pub last_pack: Option<Pack>,
}
