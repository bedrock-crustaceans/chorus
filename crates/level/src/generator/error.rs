use std::fmt::{Display, Formatter};
use std::sync::Arc;

use crate::generator::pos::ChunkPos;

#[derive(Debug, Clone)]
pub enum PhaseError {
    UndeclaredRead { dependency: &'static str, cell: ChunkPos },
    UnboundedSelfRequirement,
    MaxHopsOnOtherPhase { requirement: &'static str },
    DependencyFailed(Arc<PhaseFailure>),
    Custom(Arc<dyn std::error::Error + Send + Sync>),
}

impl PhaseError {
    pub fn custom(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Custom(Arc::new(error))
    }
}

impl Display for PhaseError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::UndeclaredRead { dependency, cell } => write!(f, "read {dependency} at ({}, {}) without requiring it there", cell.x, cell.z),
            Self::UnboundedSelfRequirement => f.write_str("requires itself without a max_hops bound"),
            Self::MaxHopsOnOtherPhase { requirement } => write!(f, "set max_hops on {requirement}, but max_hops only applies to a phase requiring itself"),
            Self::DependencyFailed(failure) => write!(f, "dependency failed: {failure}"),
            Self::Custom(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for PhaseError {}

#[derive(Debug)]
pub struct PhaseFailure {
    pub phase: &'static str,
    pub cell: ChunkPos,
    pub error: PhaseError,
}

impl Display for PhaseFailure {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{} at ({}, {}): {}", self.phase, self.cell.x, self.cell.z, self.error)
    }
}

impl std::error::Error for PhaseFailure {}
