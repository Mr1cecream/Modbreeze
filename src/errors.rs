use std::error;
use std::fmt;

pub type Result<T> = std::result::Result<T, BreezeError>;

#[derive(Debug, Clone)]
pub struct BreezeError {
    kind: BreezeErrorKind,
}

#[derive(Debug, Clone)]
pub enum BreezeErrorKind {
    InvalidLoader,
}

impl BreezeError {
    pub fn new(kind: BreezeErrorKind) -> Self {
        BreezeError { kind }
    }
}
impl fmt::Display for BreezeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let errortext: &str = match self.kind {
            BreezeErrorKind::InvalidLoader => "Invalid mod loader specified in TOML file.",
            _ => "Unknown error.",
        };
        write!(f, "{}", errortext)
    }
}

impl error::Error for BreezeError {}
