use thiserror::Error;

#[derive(Error, Debug)]
pub enum BreezeError {
    #[error("invalid mod loader specified")]
    InvalidLoader,
    #[error("couldn't find compatible file for mod {0}, id: {1}")]
    NoCompatFile(String, u32),
    #[error("distribution denied for mod {0}, id: {1}")]
    DistributionDenied(String, u32),
}
