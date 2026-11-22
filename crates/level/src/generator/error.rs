use glam::IVec2;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum PhaseError {
    UndeclaredRead { dependency: &'static str, cell: IVec2 },
    UnboundedSelfDependency,
    MaxHopsOnOtherPhase { dependency: &'static str },
    RequestCycle { dependency: &'static str, cell: IVec2 },
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
            Self::UndeclaredRead { dependency, cell } => write!(f, "read {dependency} at ({}, {}) without depending on it there", cell.x, cell.y),
            Self::UnboundedSelfDependency => f.write_str("depends on itself without a max_hops bound"),
            Self::MaxHopsOnOtherPhase { dependency } => write!(f, "set max_hops on {dependency}, but max_hops only applies to a phase depending on itself"),
            Self::RequestCycle { dependency, cell } => write!(f, "requested {dependency} at ({}, {}), which would make the graph depend on itself", cell.x, cell.y),
            Self::DependencyFailed(failure) => write!(f, "dependency failed: {failure}"),
            Self::Custom(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for PhaseError {}

#[derive(Debug)]
pub struct PhaseFailure {
    pub phase: &'static str,
    pub cell: IVec2,
    pub error: PhaseError,
}

impl Display for PhaseFailure {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{} at ({}, {}): {}", self.phase, self.cell.x, self.cell.y, self.error)
    }
}

impl std::error::Error for PhaseFailure {}
