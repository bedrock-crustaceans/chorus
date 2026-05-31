#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum RakPriority {
    Immediate,
    High,
    Normal,
    Low,
}
