use crate::utils::sem_version::SemVersion;

pub const SEM_VERSION: SemVersion = SemVersion::new(1, 26, 1, 0, 0);
pub const BLOCK_STATE_VERSION: i32 = (SEM_VERSION.major << 24) | (SEM_VERSION.minor << 16) | (SEM_VERSION.patch << 8);
