use std::str::FromStr;

use anyhow::{Context, Error, anyhow};

use crate::data::{chrom::Chrom, locus::GenomeRegion};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Variant<'a> {
    chrom: Chrom<'a>,
    pos: i64,
    ref_b: String,
    alt_b: String,
}

impl<'a> Variant<'a> {
    fn from_str_key(s: &'a str) -> Result<Self, Error> {
        fn parse_internal(s: &str) -> Result<Variant, Error> {
            let mut elem_iter = s.split("_");

            macro_rules! parse_next {
                () => {
                    elem_iter
                        .next()
                        .ok_or_else(|| anyhow!("Failed to get next element of variant key."))
                };
            }

            // first elem: chrom
            let chrom = Chrom::from_str(parse_next!()?)?;
            let pos = parse_next!()?.parse::<i64>()?;
            let ref_b = parse_next!()?.to_string();
            let alt_b = parse_next!()?.to_string();

            if parse_next!().is_ok() {
                Err(anyhow!("Invalid variant key, it has 5-th element: {}", s))?
            }

            Ok(Variant {
                chrom,
                pos,
                ref_b,
                alt_b,
            })
        }

        match parse_internal(s) {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!("{err} Input variant key: {s}")),
        }
    }

    fn get_1bp_region(&self) -> GenomeRegion<'_> {
        GenomeRegion {
            contig: self.chrom.clone(),
            start: self.pos,
            end: self.pos + 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::data::{chrom::Chrom, variant::Variant};

    #[test]
    fn test_parse_string() -> Result<(), Box<dyn std::error::Error>> {
        let a = "chrX_12341_AA_GG";

        assert_eq!(
            Variant::from_str_key(a)?,
            Variant {
                chrom: Chrom::ChrX,
                pos: 12341,
                ref_b: "AA".to_string(),
                alt_b: "GG".to_string()
            }
        );

        let a = "chr1_1234111_ACA_TGG";

        assert_eq!(
            Variant::from_str_key(a)?,
            Variant {
                chrom: Chrom::Chr1,
                pos: 1234111,
                ref_b: "ACA".to_string(),
                alt_b: "TGG".to_string()
            }
        );

        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_parse_string_invalid1() {
        let a = "chrX_12341_AA_GG_";

        Variant::from_str_key(a).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_parse_string_invalid2() {
        let a = "_chrX_12341_AA_GG";

        Variant::from_str_key(a).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_parse_string_invalid3() {
        let a = "chrX_12341_AA";

        Variant::from_str_key(a).unwrap();
    }
}
