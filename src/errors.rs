use thiserror::Error;

#[derive(Error, Debug)]
pub enum BreezeError {
    #[error("invalid mod loader specified")]
    InvalidLoader,
    #[error("couldn't find compatible file")]
    NoCompatFile,
    #[error("distribution denied for mod {0}, id: {1}")]
    DistributionDenied(String, u32),
}
