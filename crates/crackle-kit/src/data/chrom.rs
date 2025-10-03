use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use self::constants::*;

/// Represents a chromosome, optimizing for common cases to avoid string allocations.
///
/// This enum is used instead of `String` to represent chromosome names from formats
/// like VCF or BED. For the 25 standard human chromosomes (1-22, X, Y, M),
/// no new memory is allocated. Any other chromosome name is stored in the `Other`
/// variant as a `String`.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Chrom<'a> {
    Chr1,
    Chr2,
    Chr3,
    Chr4,
    Chr5,
    Chr6,
    Chr7,
    Chr8,
    Chr9,
    Chr10,
    Chr11,
    Chr12,
    Chr13,
    Chr14,
    Chr15,
    Chr16,
    Chr17,
    Chr18,
    Chr19,
    Chr20,
    Chr21,
    Chr22,
    ChrX,
    ChrY,
    ChrM,
    /// For chromosome names that are not standard (e.g., "chrEBV", custom contigs).
    Other(Cow<'a, str>),
}

impl<'a> From<&str> for Chrom<'a> {
    fn from(value: &str) -> Self {
        Self::from_str(value).unwrap()
    }
}

/// Allows `Chrom` to be formatted into a string using `format!`, `println!`, or `.to_string()`.
///
/// # Examples
///
/// ```
/// let chrom = Chrom::Chr1;
/// assert_eq!(chrom.to_string(), "chr1");
///
/// let other_chrom = Chrom::Other("chrEBV".to_string());
/// assert_eq!(other_chrom.to_string(), "chrEBV");
/// ```
impl<'a> fmt::Display for Chrom<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Allows a string slice to be parsed into a `Chrom` using `.parse()`.
///
/// This implementation never fails. If the string does not match a standard
/// chromosome, it will be stored in the `Chrom::Other` variant.
///
/// # Examples
///
/// ```
/// let chrom: Chrom = "chrX".parse().unwrap();
/// assert_eq!(chrom, Chrom::ChrX);
///
/// let other: Chrom = "random_contig".parse().unwrap();
/// assert_eq!(other, Chrom::Other("random_contig".to_string()));
/// ```
impl<'a> FromStr for Chrom<'a> {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let r = match s {
            CHR1 => Chrom::Chr1,
            CHR2 => Chrom::Chr2,
            CHR3 => Chrom::Chr3,
            CHR4 => Chrom::Chr4,
            CHR5 => Chrom::Chr5,
            CHR6 => Chrom::Chr6,
            CHR7 => Chrom::Chr7,
            CHR8 => Chrom::Chr8,
            CHR9 => Chrom::Chr9,
            CHR10 => Chrom::Chr10,
            CHR11 => Chrom::Chr11,
            CHR12 => Chrom::Chr12,
            CHR13 => Chrom::Chr13,
            CHR14 => Chrom::Chr14,
            CHR15 => Chrom::Chr15,
            CHR16 => Chrom::Chr16,
            CHR17 => Chrom::Chr17,
            CHR18 => Chrom::Chr18,
            CHR19 => Chrom::Chr19,
            CHR20 => Chrom::Chr20,
            CHR21 => Chrom::Chr21,
            CHR22 => Chrom::Chr22,
            CHRX => Chrom::ChrX,
            CHRY => Chrom::ChrY,
            CHRM => Chrom::ChrM,
            oth => Chrom::Other(oth.to_string().into()),
        };

        Ok(r)
    }
}

impl<'a> Chrom<'a> {
    pub fn typical_chroms() -> [Chrom<'static>; 24] {
        const TYPICAL_CHROMS: [Chrom; 24] = [
            Chrom::Chr1,
            Chrom::Chr2,
            Chrom::Chr3,
            Chrom::Chr4,
            Chrom::Chr5,
            Chrom::Chr6,
            Chrom::Chr7,
            Chrom::Chr8,
            Chrom::Chr9,
            Chrom::Chr10,
            Chrom::Chr11,
            Chrom::Chr12,
            Chrom::Chr13,
            Chrom::Chr14,
            Chrom::Chr15,
            Chrom::Chr16,
            Chrom::Chr17,
            Chrom::Chr18,
            Chrom::Chr19,
            Chrom::Chr20,
            Chrom::Chr21,
            Chrom::Chr22,
            Chrom::ChrX,
            Chrom::ChrY,
        ];

        TYPICAL_CHROMS
    }

    /// Returns a string slice (`&str`) representation of the chromosome.
    ///
    /// This is a zero-cost operation for standard chromosomes. For the `Other`
    /// variant, it returns a slice of the contained `String`.
    pub fn as_str(&self) -> &str {
        match self {
            Chrom::Chr1 => CHR1,
            Chrom::Chr2 => CHR2,
            Chrom::Chr3 => CHR3,
            Chrom::Chr4 => CHR4,
            Chrom::Chr5 => CHR5,
            Chrom::Chr6 => CHR6,
            Chrom::Chr7 => CHR7,
            Chrom::Chr8 => CHR8,
            Chrom::Chr9 => CHR9,
            Chrom::Chr10 => CHR10,
            Chrom::Chr11 => CHR11,
            Chrom::Chr12 => CHR12,
            Chrom::Chr13 => CHR13,
            Chrom::Chr14 => CHR14,
            Chrom::Chr15 => CHR15,
            Chrom::Chr16 => CHR16,
            Chrom::Chr17 => CHR17,
            Chrom::Chr18 => CHR18,
            Chrom::Chr19 => CHR19,
            Chrom::Chr20 => CHR20,
            Chrom::Chr21 => CHR21,
            Chrom::Chr22 => CHR22,
            Chrom::ChrX => CHRX,
            Chrom::ChrY => CHRY,
            Chrom::ChrM => CHRM,
            Chrom::Other(s) => &*s,
        }
    }
}

/// A module to hold the string constants for standard chromosome names.
mod constants {
    pub(crate) const CHR1: &'static str = "chr1";
    pub(crate) const CHR2: &'static str = "chr2";
    pub(crate) const CHR3: &'static str = "chr3";
    pub(crate) const CHR4: &'static str = "chr4";
    pub(crate) const CHR5: &'static str = "chr5";
    pub(crate) const CHR6: &'static str = "chr6";
    pub(crate) const CHR7: &'static str = "chr7";
    pub(crate) const CHR8: &'static str = "chr8";
    pub(crate) const CHR9: &'static str = "chr9";
    pub(crate) const CHR10: &'static str = "chr10";
    pub(crate) const CHR11: &'static str = "chr11";
    pub(crate) const CHR12: &'static str = "chr12";
    pub(crate) const CHR13: &'static str = "chr13";
    pub(crate) const CHR14: &'static str = "chr14";
    pub(crate) const CHR15: &'static str = "chr15";
    pub(crate) const CHR16: &'static str = "chr16";
    pub(crate) const CHR17: &'static str = "chr17";
    pub(crate) const CHR18: &'static str = "chr18";
    pub(crate) const CHR19: &'static str = "chr19";
    pub(crate) const CHR20: &'static str = "chr20";
    pub(crate) const CHR21: &'static str = "chr21";
    pub(crate) const CHR22: &'static str = "chr22";
    pub(crate) const CHRX: &'static str = "chrX";
    pub(crate) const CHRY: &'static str = "chrY";
    pub(crate) const CHRM: &'static str = "chrM";
}

// This block is only compiled when running `cargo test`.
#[cfg(test)]
mod tests {
    // Import everything from the parent module (the file itself).
    use super::*;

    #[test]
    fn test_from_str_parsing() {
        // Test a standard autosome
        let chrom1: Chrom = "chr1".parse().unwrap();
        assert_eq!(chrom1, Chrom::Chr1);

        // Test a sex chromosome
        let chromx: Chrom = "chrX".parse().unwrap();
        assert_eq!(chromx, Chrom::ChrX);

        // Test the mitochondrial chromosome
        let chromm: Chrom = "chrM".parse().unwrap();
        assert_eq!(chromm, Chrom::ChrM);

        // Test a non-standard chromosome
        let other: Chrom = "chrEBV".parse().unwrap();
        assert_eq!(other, Chrom::Other("chrEBV".to_string().into()));
    }

    #[test]
    fn test_as_str_and_display() {
        // Test as_str
        assert_eq!(Chrom::Chr22.as_str(), "chr22");
        let other_chrom = Chrom::Other("my_contig".to_string().into());
        assert_eq!(other_chrom.as_str(), "my_contig");

        // Test Display trait (.to_string())
        assert_eq!(Chrom::Chr22.to_string(), "chr22");
        assert_eq!(other_chrom.to_string(), "my_contig");
    }

    #[test]
    fn test_round_trip_conversion() {
        // Test that parsing and then converting back to a string yields the original.
        let original_str = "chr5";
        let parsed: Chrom = original_str.parse().unwrap();
        let final_str = parsed.to_string();
        assert_eq!(original_str, final_str);

        // Test round-trip for a non-standard name
        let original_other_str = "GL000218.1";
        let parsed_other: Chrom = original_other_str.parse().unwrap();
        let final_other_str = parsed_other.to_string();
        assert_eq!(original_other_str, final_other_str);
    }
}
