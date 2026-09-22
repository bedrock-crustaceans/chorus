#[derive(Clone)]
pub struct SemVersion {
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
    pub revision: i32,
    pub build: i32,
}

impl SemVersion {
    pub const fn new(major: i32, minor: i32, patch: i32, revision: i32, build: i32) -> Self {
        Self { major, minor, patch, revision, build }
    }
}
