use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

use crate::ModSide;

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Config {
    pub mc_dir: Option<PathBuf>,
    pub source: Option<PathOrUrl>,
    pub side: Option<ModSide>,
    pub resourcepacks: Option<bool>,
    pub shaderpacks: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum PathOrUrl {
    Path(PathBuf),
    Url(Url),
}
