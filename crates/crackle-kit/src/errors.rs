use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PosError {
    #[error("0-based position must be >= 0, got {0}")]
    NegativeZeroBased(i64),
    #[error("1-based position must be > 0, got {0}")]
    NonPositiveOneBased(i64),
    #[error("cannot represent 1-based position for 0-based value {0}")]
    OverflowOneBased(i64),
    #[error("position {0} cannot be represented as u32")]
    OutOfRangeU32(i64),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LocusError {
    #[error(transparent)]
    Pos(#[from] PosError),
}
