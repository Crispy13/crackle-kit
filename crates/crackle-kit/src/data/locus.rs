use std::str::FromStr;
use std::ops::Deref;

use crate::data::chrom::Chrom;
use crate::errors::{LocusError, PosError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pos(i64);

impl Pos {
    pub fn from_0based(value: i64) -> Result<Self, PosError> {
        if value < 0 {
            return Err(PosError::NegativeZeroBased(value));
        }

        if value == i64::MAX {
            return Err(PosError::OverflowOneBased(value));
        }

        Ok(Self(value))
    }

    pub fn from_1based(value: i64) -> Result<Self, PosError> {
        if value <= 0 {
            return Err(PosError::NonPositiveOneBased(value));
        }

        Ok(Self(value - 1))
    }

    pub fn zero_based(&self) -> i64 {
        self.0
    }

    pub fn one_based(&self) -> i64 {
        self.0 + 1
    }

    pub fn zero_based_u32(&self) -> Result<u32, PosError> {
        u32::try_from(self.0).map_err(|_| PosError::OutOfRangeU32(self.0))
    }

    pub fn one_based_u32(&self) -> Result<u32, PosError> {
        let one_based = self.one_based();
        u32::try_from(one_based).map_err(|_| PosError::OutOfRangeU32(one_based))
    }

    pub fn checked_add(&self, rhs: i64) -> Result<Self, PosError> {
        let value = self
            .0
            .checked_add(rhs)
            .ok_or(PosError::OverflowOneBased(self.0))?;
        Self::from_0based(value)
    }

    pub fn checked_sub(&self, rhs: i64) -> Result<Self, PosError> {
        let value = self
            .0
            .checked_sub(rhs)
            .ok_or(PosError::NegativeZeroBased(self.0))?;
        Self::from_0based(value)
    }
}

impl From<u32> for Pos {
    fn from(value: u32) -> Self {
        Self(i64::from(value))
    }
}

impl From<i32> for Pos {
    fn from(value: i32) -> Self {
        Self::from_0based(i64::from(value))
            .expect("cannot convert i32 to Pos: value violates Pos invariants")
    }
}

impl From<i64> for Pos {
    fn from(value: i64) -> Self {
        Self::from_0based(value)
            .expect("cannot convert i64 to Pos: value violates Pos invariants")
    }
}

impl From<u64> for Pos {
    fn from(value: u64) -> Self {
        let value = i64::try_from(value)
            .expect("cannot convert u64 to Pos: value does not fit into i64 range");
        Self::from_0based(value)
            .expect("cannot convert u64 to Pos: value violates Pos invariants")
    }
}

impl From<usize> for Pos {
    fn from(value: usize) -> Self {
        let value = i64::try_from(value)
            .expect("cannot convert usize to Pos: value does not fit into i64 range");
        Self::from_0based(value)
            .expect("cannot convert usize to Pos: value violates Pos invariants")
    }
}

impl TryFrom<isize> for Pos {
    type Error = PosError;

    fn try_from(value: isize) -> Result<Self, Self::Error> {
        Self::from_0based(value as i64)
    }
}

impl AsRef<i64> for Pos {
    fn as_ref(&self) -> &i64 {
        &self.0
    }
}

impl Deref for Pos {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Pos> for i64 {
    fn from(value: Pos) -> Self {
        value.zero_based()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GenomeCoordinate<'a> {
    pub contig: Chrom<'a>,

    /// 1-based position.
    pub pos: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct GenomeRegion<'a> {
    pub contig: Chrom<'a>,

    /// 1-based position.
    pub start: i64,
    /// 1-based position.
    pub end: i64,
}

impl<'a> From<(&str, i64, i64)> for GenomeRegion<'a> {
    fn from(value: (&str, i64, i64)) -> Self {
        Self {
            contig: Chrom::from_str(value.0).unwrap(),
            start: value.1,
            end: value.2,
        }
    }
}

impl<'a> GenomeRegion<'a> {
    pub(crate) fn as_fetch_tuple(&self) -> (&str, i64, i64) {
        (self.contig.as_str(), self.start as i64, self.end as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pos_from_0based_and_1based() {
        let from_zero = Pos::from_0based(110_234).unwrap();
        assert_eq!(from_zero.zero_based(), 110_234);
        assert_eq!(from_zero.one_based(), 110_235);

        let from_one = Pos::from_1based(110_235).unwrap();
        assert_eq!(from_one.zero_based(), 110_234);
        assert_eq!(from_one.one_based(), 110_235);
    }

    #[test]
    fn pos_into_style() {
        let pos: Pos = 110_234.into();
        assert_eq!(pos.zero_based(), 110_234);
        assert_eq!(pos.one_based(), 110_235);
    }

    #[test]
    fn pos_try_from_signed() {
        let p = Pos::try_from(42_isize).unwrap();
        assert_eq!(p.zero_based(), 42);

        assert_eq!(
            Pos::try_from(-1_isize),
            Err(PosError::NegativeZeroBased(-1))
        );
    }

    #[test]
    fn pos_deref_like_number() {
        let pos: Pos = 11_u32.into();
        let v: i64 = *pos;
        assert_eq!(v, 11);

        let via_as_ref: i64 = *pos.as_ref();
        assert_eq!(via_as_ref, 11);
    }

    #[test]
    fn pos_validation() {
        assert_eq!(Pos::from_0based(-1), Err(PosError::NegativeZeroBased(-1)));
        assert_eq!(Pos::from_1based(0), Err(PosError::NonPositiveOneBased(0)));
        assert_eq!(
            Pos::from_0based(i64::MAX),
            Err(PosError::OverflowOneBased(i64::MAX))
        );
    }

    #[test]
    fn pos_u32_conversion() {
        let pos = Pos::from_0based(123).unwrap();
        assert_eq!(pos.zero_based_u32().unwrap(), 123);
        assert_eq!(pos.one_based_u32().unwrap(), 124);
    }

    #[test]
    fn pos_checked_arithmetic() {
        let pos = Pos::from_0based(10).unwrap();
        assert_eq!(pos.checked_add(5).unwrap().zero_based(), 15);
        assert_eq!(pos.checked_sub(4).unwrap().zero_based(), 6);
        assert_eq!(pos.checked_sub(20), Err(PosError::NegativeZeroBased(-10)));
    }

    #[test]
    fn locus_error_wraps_pos_error() {
        let err: LocusError = Pos::from_1based(0).unwrap_err().into();
        assert!(matches!(err, LocusError::Pos(PosError::NonPositiveOneBased(0))));
    }
}
