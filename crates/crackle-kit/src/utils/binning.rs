#[cfg(feature = "bio")]
use self::fasta::*;
use std::path::Path;

pub fn make_bins(start: usize, end: usize, bin_size: usize) -> Vec<(usize, usize)> {
    let mut bin_ranges = (start..end)
        .step_by(bin_size)
        .map(|i| (i, i + bin_size))
        .collect::<Vec<_>>();
    match bin_ranges.last_mut() {
        Some(v) => {
            if v.1 < end {
                let elem = (v.1, end);
                bin_ranges.push(elem);
            } else if v.1 > end {
                v.1 = end;
            }
        }
        None => {}
    }

    bin_ranges
}

#[cfg(feature = "bio")]
mod fasta {
    use std::{collections::HashMap, str::FromStr};

    use crate::data::chrom::Chrom;

    use super::*;
    use anyhow::Error;
    use bio::io::fasta::IndexedReader;

    pub fn make_bins_from_fasta(
        fasta_file: impl AsRef<Path>,
        bin_size: usize,
    ) -> Result<HashMap<Chrom, Vec<(usize, usize)>>, Error> {
        let ir = IndexedReader::from_file(&fasta_file.as_ref())?;

        let mut res = HashMap::with_capacity(ir.index.sequences().len());
        for seq in ir.index.sequences() {
            let bins = make_bins(0, seq.len as usize, bin_size);
            res.insert(Chrom::from_str(&seq.name).unwrap(), bins);
        }

        Ok(res)
    }
}

// --- Test Functions ---
#[cfg(test)] // This attribute tells Cargo to compile this module only when running tests
mod tests {
    use super::*; // Import the `make_bins` function from the parent scope

    #[test]
    fn test_exact_fit() {
        // Test case where the end is an exact multiple of bin_size from the start
        let bins = make_bins(0, 100, 10);
        assert_eq!(
            bins,
            vec![
                (0, 10),
                (10, 20),
                (20, 30),
                (30, 40),
                (40, 50),
                (50, 60),
                (60, 70),
                (70, 80),
                (80, 90),
                (90, 100)
            ]
        );
    }

    #[test]
    fn test_partial_last_bin() {
        // Test case where the last bin is partial
        let bins = make_bins(0, 105, 10);
        assert_eq!(
            bins,
            vec![
                (0, 10),
                (10, 20),
                (20, 30),
                (30, 40),
                (40, 50),
                (50, 60),
                (60, 70),
                (70, 80),
                (80, 90),
                (90, 100),
                (100, 105) // The last bin should go up to `end`
            ]
        );
    }

    #[test]
    fn test_single_bin_exact() {
        // Test case for a single bin that fits exactly
        let bins = make_bins(10, 20, 10);
        assert_eq!(bins, vec![(10, 20)]);
    }

    #[test]
    fn test_single_bin_partial() {
        // Test case for a single bin that is partial
        let bins = make_bins(10, 15, 10);
        assert_eq!(bins, vec![(10, 15)]);
    }

    #[test]
    fn test_start_equals_end() {
        // Test case where start and end are the same
        let bins = make_bins(50, 50, 10);
        assert_eq!(bins, vec![]); // Or vec![(50,50)] depending on desired behavior for a zero-length range.
        // Current implementation produces an empty vector.
    }

    #[test]
    fn test_bin_size_larger_than_range() {
        // Test case where bin_size is larger than the total range
        let bins = make_bins(0, 5, 10);
        assert_eq!(bins, vec![(0, 5)]);
    }

    #[test]
    fn test_large_range_small_bin() {
        // Test with a larger range and small bin size
        let bins = make_bins(100, 1000, 25);
        // Just check the first few and last few to ensure logic holds
        assert_eq!(bins[0], (100, 125));
        assert_eq!(bins[1], (125, 150));
        assert_eq!(bins.last().unwrap(), &(975, 1000));
        assert_eq!(bins.len(), (1000 - 100) / 25);
    }

    #[test]
    fn test_start_non_zero_partial_end() {
        let bins = make_bins(5, 27, 10);
        assert_eq!(bins, vec![(5, 15), (15, 25), (25, 27)]);
    }

    #[test]
    fn test_empty_range() {
        // Test with start > end, should result in an empty vector
        let bins = make_bins(100, 50, 10);
        assert_eq!(bins, vec![]);
    }
}

#[cfg(test)]
#[cfg(feature = "bio")]
mod fasta_tests {
    use crate::data::chrom::Chrom;

    use super::*;

    #[test]
    fn test_make_bins_from_fasta() {
        use std::collections::HashMap;
        use std::fs::File;
        use std::io::Write;

        use crate::data::chrom::Chrom;

        // 1. Create a dummy FASTA file
        let fasta_path = "test_bins.fa";
        let mut fasta_file = File::create(fasta_path).unwrap();
        writeln!(fasta_file, ">chr1").unwrap();
        writeln!(fasta_file, "ACGTACGTACGT").unwrap(); // 12 bases
        writeln!(fasta_file, ">chrM").unwrap();
        writeln!(fasta_file, "NNNNN").unwrap(); // 5 bases

        // 2. Create a corresponding FASTA index file (.fai)
        let fai_path = "test_bins.fa.fai";
        let mut fai_file = File::create(fai_path).unwrap();
        // Format: name, length, offset, line_bases, line_width
        writeln!(fai_file, "chr1\t12\t6\t12\t13").unwrap();
        writeln!(fai_file, "chrM\t5\t25\t5\t6").unwrap();

        // 3. Call the function to be tested
        let bin_size = 10;
        let result = fasta::make_bins_from_fasta(fasta_path, bin_size).unwrap();

        // 4. Define the expected output
        let mut expected = HashMap::new();
        expected.insert(Chrom::Chr1, vec![(0, 10), (10, 12)]);
        expected.insert(Chrom::ChrM, vec![(0, 5)]);

        // 5. Assert correctness
        assert_eq!(result.len(), 2);
        assert_eq!(result.get(&Chrom::Chr1), Some(&vec![(0, 10), (10, 12)]));
        assert_eq!(result.get(&Chrom::ChrM), Some(&vec![(0, 5)]));
        assert_eq!(result, expected);

        // 6. Clean up the dummy files
        std::fs::remove_file(fasta_path).unwrap();
        std::fs::remove_file(fai_path).unwrap();
    }

    #[test]
    fn make_bins_using_grch38_fasta() -> Result<(), Box<dyn std::error::Error>> {
        let fasta_file =
            "/home/eck/workspace/common_resources/GCF_000001405.40_GRCh38.p14_genomic.fna.gz";
        let mut bin_map = make_bins_from_fasta(fasta_file, 100_000_000)?;
        bin_map.retain(|k, v| ["NC_000002.12", "NC_000001.11"].contains(&k.as_str()));

        println!("{:?}", bin_map);

        Ok(())
    }
}
