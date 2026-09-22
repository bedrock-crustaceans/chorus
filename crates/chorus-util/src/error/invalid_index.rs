use std::fmt::Display;

#[derive(Debug)]
pub struct InvalidIndexError(pub usize);

impl Display for InvalidIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InvalidIndexError {{ index: {:?} }}", self.0)
    }
}

impl std::error::Error for InvalidIndexError {}
