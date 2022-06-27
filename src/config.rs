use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Config {
    pub mod_dir: Option<PathBuf>,
    pub source: Option<PathOrUrl>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum PathOrUrl {
    Path(PathBuf),
    Url(Url),
}
