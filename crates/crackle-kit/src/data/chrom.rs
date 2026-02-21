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
/// let other_chrom = Chrom::Other("chrEBV".to_string().into());
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
/// For non-standard chromosome names that do not start with `"chr"`, this
/// parser canonicalizes them to `"chr{name}"` in `Chrom::Other`.
///
/// # Examples
///
/// ```
/// let chrom: Chrom = "chrX".parse().unwrap();
/// assert_eq!(chrom, Chrom::ChrX);
///
/// let other: Chrom = "random_contig".parse().unwrap();
/// assert_eq!(other, Chrom::Other("chrrandom_contig".to_string().into()));
/// ```
impl<'a> FromStr for Chrom<'a> {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(core) = s.strip_prefix("chr") {
            if let Some(chrom) = Self::from_standard_core(core) {
                return Ok(chrom);
            }

            return Ok(Chrom::Other(s.to_string().into()));
        }

        if let Some(chrom) = Self::from_standard_core(s) {
            return Ok(chrom);
        }

        Ok(Chrom::Other(format!("chr{s}").into()))
    }
}

impl AsRef<str> for Chrom<'_> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> From<Cow<'a, str>> for Chrom<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        if let Some(core) = value.strip_prefix("chr") {
            if let Some(chrom) = Chrom::from_standard_core(core) {
                return chrom;
            }

            return Chrom::Other(value);
        }

        if let Some(chrom) = Chrom::from_standard_core(&value) {
            return chrom;
        }

        Chrom::Other(format!("chr{value}").into())
    }
}

impl From<String> for Chrom<'static> {
    fn from(value: String) -> Self {
        Chrom::from(Cow::Owned(value))
    }
}

impl<'a> Chrom<'a> {
    fn from_standard_core(s: &str) -> Option<Self> {
        match s {
            "1" => Some(Chrom::Chr1),
            "2" => Some(Chrom::Chr2),
            "3" => Some(Chrom::Chr3),
            "4" => Some(Chrom::Chr4),
            "5" => Some(Chrom::Chr5),
            "6" => Some(Chrom::Chr6),
            "7" => Some(Chrom::Chr7),
            "8" => Some(Chrom::Chr8),
            "9" => Some(Chrom::Chr9),
            "10" => Some(Chrom::Chr10),
            "11" => Some(Chrom::Chr11),
            "12" => Some(Chrom::Chr12),
            "13" => Some(Chrom::Chr13),
            "14" => Some(Chrom::Chr14),
            "15" => Some(Chrom::Chr15),
            "16" => Some(Chrom::Chr16),
            "17" => Some(Chrom::Chr17),
            "18" => Some(Chrom::Chr18),
            "19" => Some(Chrom::Chr19),
            "20" => Some(Chrom::Chr20),
            "21" => Some(Chrom::Chr21),
            "22" => Some(Chrom::Chr22),
            "X" => Some(Chrom::ChrX),
            "Y" => Some(Chrom::ChrY),
            "M" => Some(Chrom::ChrM),
            _ => None,
        }
    }

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

    /// Returns the chromosome as a `chr`-prefixed contig name.
    ///
    /// This is useful when interoperating with formats that use contigs like
    /// `chr1`, `chrX`, `chrM`.
    pub fn to_prefixed(&self) -> Cow<'_, str> {
        match self {
            Chrom::Chr1 => Cow::Borrowed(CHR1),
            Chrom::Chr2 => Cow::Borrowed(CHR2),
            Chrom::Chr3 => Cow::Borrowed(CHR3),
            Chrom::Chr4 => Cow::Borrowed(CHR4),
            Chrom::Chr5 => Cow::Borrowed(CHR5),
            Chrom::Chr6 => Cow::Borrowed(CHR6),
            Chrom::Chr7 => Cow::Borrowed(CHR7),
            Chrom::Chr8 => Cow::Borrowed(CHR8),
            Chrom::Chr9 => Cow::Borrowed(CHR9),
            Chrom::Chr10 => Cow::Borrowed(CHR10),
            Chrom::Chr11 => Cow::Borrowed(CHR11),
            Chrom::Chr12 => Cow::Borrowed(CHR12),
            Chrom::Chr13 => Cow::Borrowed(CHR13),
            Chrom::Chr14 => Cow::Borrowed(CHR14),
            Chrom::Chr15 => Cow::Borrowed(CHR15),
            Chrom::Chr16 => Cow::Borrowed(CHR16),
            Chrom::Chr17 => Cow::Borrowed(CHR17),
            Chrom::Chr18 => Cow::Borrowed(CHR18),
            Chrom::Chr19 => Cow::Borrowed(CHR19),
            Chrom::Chr20 => Cow::Borrowed(CHR20),
            Chrom::Chr21 => Cow::Borrowed(CHR21),
            Chrom::Chr22 => Cow::Borrowed(CHR22),
            Chrom::ChrX => Cow::Borrowed(CHRX),
            Chrom::ChrY => Cow::Borrowed(CHRY),
            Chrom::ChrM => Cow::Borrowed(CHRM),
            Chrom::Other(s) => {
                if s.starts_with("chr") {
                    Cow::Borrowed(s)
                } else {
                    Cow::Owned(format!("chr{s}"))
                }
            }
        }
    }

    /// Returns the chromosome as an unprefixed contig name.
    ///
    /// This is useful when interoperating with formats that use contigs like
    /// `1`, `X`, `M`.
    pub fn to_unprefixed(&self) -> &str {
        match self {
            Chrom::Chr1 => "1",
            Chrom::Chr2 => "2",
            Chrom::Chr3 => "3",
            Chrom::Chr4 => "4",
            Chrom::Chr5 => "5",
            Chrom::Chr6 => "6",
            Chrom::Chr7 => "7",
            Chrom::Chr8 => "8",
            Chrom::Chr9 => "9",
            Chrom::Chr10 => "10",
            Chrom::Chr11 => "11",
            Chrom::Chr12 => "12",
            Chrom::Chr13 => "13",
            Chrom::Chr14 => "14",
            Chrom::Chr15 => "15",
            Chrom::Chr16 => "16",
            Chrom::Chr17 => "17",
            Chrom::Chr18 => "18",
            Chrom::Chr19 => "19",
            Chrom::Chr20 => "20",
            Chrom::Chr21 => "21",
            Chrom::Chr22 => "22",
            Chrom::ChrX => "X",
            Chrom::ChrY => "Y",
            Chrom::ChrM => "M",
            Chrom::Other(s) => {
                if let Some(core) = s.strip_prefix("chr") {
                    core
                } else {
                    s
                }
            }
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

        // Test a standard autosome without prefix
        let chrom1_no_prefix: Chrom = "1".parse().unwrap();
        assert_eq!(chrom1_no_prefix, Chrom::Chr1);

        // Test a sex chromosome
        let chromx: Chrom = "chrX".parse().unwrap();
        assert_eq!(chromx, Chrom::ChrX);

        // Test a sex chromosome without prefix
        let chromx_no_prefix: Chrom = "X".parse().unwrap();
        assert_eq!(chromx_no_prefix, Chrom::ChrX);

        // Test the mitochondrial chromosome
        let chromm: Chrom = "chrM".parse().unwrap();
        assert_eq!(chromm, Chrom::ChrM);

        // Test the mitochondrial chromosome without prefix
        let chromm_no_prefix: Chrom = "M".parse().unwrap();
        assert_eq!(chromm_no_prefix, Chrom::ChrM);

        // Test a non-standard chromosome
        let other: Chrom = "chrEBV".parse().unwrap();
        assert_eq!(other, Chrom::Other("chrEBV".to_string().into()));

        // Test non-standard chromosome canonicalization when no prefix is present
        let other_no_prefix: Chrom = "EBV".parse().unwrap();
        assert_eq!(other_no_prefix, Chrom::Other("chrEBV".to_string().into()));
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

        // Non-standard names without a prefix are canonicalized to include "chr".
        let original_other_str = "GL000218.1";
        let parsed_other: Chrom = original_other_str.parse().unwrap();
        let final_other_str = parsed_other.to_string();
        assert_eq!("chrGL000218.1", final_other_str);

        // Prefixed non-standard names are preserved.
        let prefixed_other = "chrGL000218.2";
        let parsed_prefixed_other: Chrom = prefixed_other.parse().unwrap();
        assert_eq!(parsed_prefixed_other.to_string(), prefixed_other);
    }

    #[test]
    fn test_edge_cases_and_traits() {
        let empty: Chrom = "".parse().unwrap();
        assert_eq!(empty, Chrom::Other("chr".to_string().into()));

        let bare_prefix: Chrom = "chr".parse().unwrap();
        assert_eq!(bare_prefix, Chrom::Other("chr".to_string().into()));

        let unknown_prefixed: Chrom = "chr0".parse().unwrap();
        assert_eq!(unknown_prefixed, Chrom::Other("chr0".to_string().into()));

        let via_cow = Chrom::from(Cow::Borrowed("chrY"));
        assert_eq!(via_cow, Chrom::ChrY);

        let via_string = Chrom::from("EBV".to_string());
        assert_eq!(via_string, Chrom::Other("chrEBV".to_string().into()));

        assert_eq!(Chrom::Chr22.as_ref(), "chr22");
    }

    #[test]
    fn test_to_prefixed_and_to_unprefixed() {
        let standard = Chrom::Chr1;
        assert!(matches!(standard.to_prefixed(), Cow::Borrowed("chr1")));
        assert_eq!(standard.to_unprefixed(), "1");

        let standard_m = Chrom::ChrM;
        assert!(matches!(standard_m.to_prefixed(), Cow::Borrowed("chrM")));
        assert_eq!(standard_m.to_unprefixed(), "M");

        let other_prefixed = Chrom::Other(Cow::Borrowed("chrEBV"));
        assert!(matches!(
            other_prefixed.to_prefixed(),
            Cow::Borrowed("chrEBV")
        ));
        assert_eq!(other_prefixed.to_unprefixed(), "EBV");

        let other_unprefixed = Chrom::Other(Cow::Borrowed("EBV"));
        assert!(matches!(
            other_unprefixed.to_prefixed(),
            Cow::Owned(ref s) if s == "chrEBV"
        ));
        assert_eq!(other_unprefixed.to_unprefixed(), "EBV");

        let other_mixed_case = Chrom::Other(Cow::Borrowed("ChrEBV"));
        assert!(matches!(
            other_mixed_case.to_prefixed(),
            Cow::Owned(ref s) if s == "chrChrEBV"
        ));
        assert_eq!(other_mixed_case.to_unprefixed(), "ChrEBV");
    }
}
