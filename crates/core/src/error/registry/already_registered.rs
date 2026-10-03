use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct AlreadyRegisteredError {
    pub identifier: String,
}

impl Display for AlreadyRegisteredError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "AlreadyRegisteredError: {{ identifier: {} }}", self.identifier)
    }
}

impl std::error::Error for AlreadyRegisteredError {}
