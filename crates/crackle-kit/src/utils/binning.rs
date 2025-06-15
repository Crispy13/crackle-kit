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
