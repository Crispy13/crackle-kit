
use std::borrow::Cow;

#[derive(Debug, PartialEq, Eq)]
pub struct GenomeRegion<'a> {
    pub contig: Cow<'a, str>,
    pub start: i64,
    pub end: i64,
}

impl<'a, C: Into<Cow<'a, str>>> From<(C, i64, i64)> for GenomeRegion<'a> {
    fn from(value: (C, i64, i64)) -> Self {
        Self {
            contig: value.0.into(),
            start: value.1,
            end: value.2,
        }
    }
}

impl<'a> GenomeRegion<'a> {
    pub(crate) fn as_fetch_tuple(&self) -> (&str, i64, i64) {
        (&*self.contig, self.start as i64, self.end as i64)
    }
}
