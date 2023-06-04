use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

use crate::structs::ModSide;

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mc_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<PathOrUrl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<ModSide>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum PathOrUrl {
    Path(PathBuf),
    Url(Url),
}
