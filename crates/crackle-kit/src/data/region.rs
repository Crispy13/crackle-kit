use std::{borrow::Cow, str::FromStr};

use crate::data::chrom::Chrom;

#[derive(Debug, PartialEq, Eq)]
pub struct GenomeRegion<'a> {
    pub contig: Chrom<'a>,
    pub start: i64,
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
