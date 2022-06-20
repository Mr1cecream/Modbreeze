use thiserror::Error;

pub type Result<T> = std::result::Result<T, BreezeError>;

#[derive(Error, Debug)]
pub enum BreezeError {
    #[error("invalid mod loader specified")]
    InvalidLoader,
}
