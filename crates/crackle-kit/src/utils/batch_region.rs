use crate::data::region::GenomeRegion;

/// Batches an iterator of `GenomeRegion`s into `Vec<GenomeRegion>`s based on a `window_size`.
///
/// Regions are grouped together as long as they are on the same contig AND
/// the span from the start of the first region in the current batch to the end
/// of the current region does not exceed `window_size`.
///
/// # Arguments
/// * `input` - An iterator yielding items convertible into `GenomeRegion`.
/// * `window_size` - The maximum allowed span (in base pairs) for a single batch.
///
/// # Returns
/// A `Vec` of `Vec<GenomeRegion>`, where each inner `Vec` represents a batch of regions.
pub fn batch_region<'a, G: Into<GenomeRegion<'a>>>(
    input: impl Iterator<Item = G>,
    window_size: usize,
) -> Vec<Vec<GenomeRegion<'a>>> {
    let mut gr_iter = input.map(Into::<GenomeRegion>::into);

    let mut c_vec: Vec<GenomeRegion<'a>> = vec![];
    let (mut c_contig, mut c_start) = match gr_iter.next() {
        Some(gr) => {
            let contig_clone = gr.contig.clone();
            let start_val = gr.start;
            c_vec.push(gr);
            (contig_clone, start_val)
        }
        None => return vec![],
    };

    let mut res = vec![];
    for gr in gr_iter {
        // Condition to start a new batch:
        // 1. The contig changes.
        // 2. The span from the batch's start to the current region's end exceeds window_size.
        if c_contig == gr.contig && gr.end - c_start < window_size as i64 {
            c_vec.push(gr);
        } else {
            res.push(c_vec); // Push the completed batch
            c_contig = gr.contig.clone(); // Start a new batch: reset contig and start
            c_start = gr.start;
            c_vec = vec![gr]; // Start new batch with current region
        }
    }

    // Push any remaining regions in the last batch
    if !c_vec.is_empty() {
        res.push(c_vec);
    }

    res
}


// --- Test Functions for batch_region ---
#[cfg(test)] // This attribute tells Cargo to compile this module only when running tests
mod tests {
    use super::*; // Import all functions and structs from the parent scope

    #[test]
    fn test_batch_region_empty_input() {
        let regions: Vec<GenomeRegion> = vec![];
        let batches = batch_region(regions.into_iter(), 100);
        assert_eq!(batches, Vec::<Vec<GenomeRegion>>::new());
    }

    #[test]
    fn test_batch_region_single_region_fits_window() {
        let regions = vec![
            GenomeRegion::from(("chr1", 10, 50)),
        ];
        let batches = batch_region(regions.into_iter(), 100); // Window is 100
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 1);
        assert_eq!(batches[0][0].start, 10);
        assert_eq!(batches[0][0].end, 50);
        assert_eq!(batches[0][0].contig, "chr1"); // Direct string comparison works for Cow
    }

    #[test]
    fn test_batch_region_single_region_larger_than_window() {
        let regions = vec![
            GenomeRegion::from(("chr1", 0, 150)),
        ];
        // The first region itself spans more than window_size.
        // It will still be put in a batch by itself, because c_start is its start.
        // The condition `gr.end - c_start < window_size` will be true if it's the only one.
        // The logic handles the first element special.
        let batches = batch_region(regions.into_iter(), 100); // Window is 100
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 1);
        assert_eq!(batches[0][0].start, 0);
        assert_eq!(batches[0][0].end, 150);
    }

    #[test]
    fn test_batch_region_multiple_regions_fit_in_one_batch() {
        let regions = vec![
            GenomeRegion::from(("chr1", 10, 20)),
            GenomeRegion::from(("chr1", 25, 35)),
            GenomeRegion::from(("chr1", 40, 50)),
        ];
        let batches = batch_region(regions.into_iter(), 100); // Window is 100
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 3);
        assert_eq!(batches[0][0].start, 10);
        assert_eq!(batches[0][2].end, 50); // Total span is 50-10 = 40, which is < 100
    }

    #[test]
    fn test_batch_region_multiple_batches() {
        let regions = vec![
            GenomeRegion::from(("chr1", 0, 20)),   // Span 0-20
            GenomeRegion::from(("chr1", 20, 40)),  // Span 0-40 (batch 1)
            GenomeRegion::from(("chr1", 100, 120)), // Span 100-120 (batch 2)
            GenomeRegion::from(("chr1", 120, 140)),// Span 100-140 (batch 2)
            GenomeRegion::from(("chr1", 200, 220)),// Span 200-220 (batch 3)
        ];
        let batches = batch_region(regions.into_iter(), 50); // Window is 50

        assert_eq!(batches.len(), 3);

        // Batch 1: (0,20), (20,40)
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[0][0].start, 0);
        assert_eq!(batches[0][1].end, 40);

        // Batch 2: (100,120), (120,140)
        assert_eq!(batches[1].len(), 2);
        assert_eq!(batches[1][0].start, 100);
        assert_eq!(batches[1][1].end, 140);

        // Batch 3: (200,220)
        assert_eq!(batches[2].len(), 1);
        assert_eq!(batches[2][0].start, 200);
        assert_eq!(batches[2][0].end, 220);
    }

    #[test]
    fn test_batch_region_boundary_spanning() {
        let regions = vec![
            GenomeRegion::from(("chr1", 0, 40)),
            GenomeRegion::from(("chr1", 45, 60)),
            GenomeRegion::from(("chr1", 70, 110)),
        ];
        let batches = batch_region(regions.into_iter(), 100); // Window is 100

        assert_eq!(batches.len(), 2);

        // Batch 1: (0,40), (45,60)
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[0][0].start, 0);
        assert_eq!(batches[0][1].end, 60);

        // Batch 2: (70,110)
        assert_eq!(batches[1].len(), 1);
        assert_eq!(batches[1][0].start, 70);
        assert_eq!(batches[1][0].end, 110);
    }

    #[test]
    fn test_batch_region_many_small_regions() {
        let regions: Vec<GenomeRegion> = (0..100)
            .map(|i| GenomeRegion::from(("chrX", i * 5, i * 5 + 3)))
            .collect();
        let batches = batch_region(regions.into_iter(), 20); // Window is 20

        // Expected batches of 4 regions each. 100 regions / 4 regions/batch = 25 batches.
        assert_eq!(batches.len(), 25);
        assert_eq!(batches[0].len(), 4);
        assert_eq!(batches[0][0].start, 0);
        assert_eq!(batches[0][3].end, 18); // 0-18 (exclusive end for last region)

        // Check a middle batch (e.g., the one starting at region index 40, which is `40 * 5 = 200`)
        assert_eq!(batches[8].len(), 4); // Batch 8 (0-indexed) starts at region 8*4=32, so 32*5 = 160.
        assert_eq!(batches[8][0].start, 160);
        assert_eq!(batches[8][3].end, 178); // 160-178

        // Check the last batch
        let last_batch = batches.last().unwrap();
        assert_eq!(last_batch.len(), 4);
        assert_eq!(last_batch[0].start, 96 * 5); // Should start at 480
        assert_eq!(last_batch[3].end, 498);
    }

    #[test]
    fn test_batch_region_different_contigs() {
        let regions = vec![
            GenomeRegion::from(("chr1", 10, 20)),
            GenomeRegion::from(("chr1", 20, 30)),
            GenomeRegion::from(("chr2", 0, 10)),   // New contig, new batch should start here
            GenomeRegion::from(("chr2", 10, 20)),
            GenomeRegion::from(("chr3", 50, 60)), // New contig, new batch should start here
        ];
        let batches = batch_region(regions.into_iter(), 100); // Window is 100

        // Expected behavior: Batches should split when contig changes.
        assert_eq!(batches.len(), 3);

        // Batch 1: chr1 regions
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[0][0].contig, "chr1");
        assert_eq!(batches[0][0].start, 10);
        assert_eq!(batches[0][1].end, 30);

        // Batch 2: chr2 regions
        assert_eq!(batches[1].len(), 2);
        assert_eq!(batches[1][0].contig, "chr2");
        assert_eq!(batches[1][0].start, 0);
        assert_eq!(batches[1][1].end, 20);

        // Batch 3: chr3 regions
        assert_eq!(batches[2].len(), 1);
        assert_eq!(batches[2][0].contig, "chr3");
        assert_eq!(batches[2][0].start, 50);
        assert_eq!(batches[2][0].end, 60);
    }

    #[test]
    fn test_batch_region_contig_change_forces_new_batch_even_if_span_small() {
        let regions = vec![
            GenomeRegion::from(("chr1", 10, 20)),
            GenomeRegion::from(("chr2", 15, 25)), // New contig, but start is numerically close
        ];
        let batches = batch_region(regions.into_iter(), 100); // Window is 100

        // Expected: Two batches because of contig change, even though numerical span (25-10 = 15) is small.
        assert_eq!(batches.len(), 2);

        // Batch 1: chr1 region
        assert_eq!(batches[0].len(), 1);
        assert_eq!(batches[0][0].contig, "chr1");
        assert_eq!(batches[0][0].start, 10);
        assert_eq!(batches[0][0].end, 20);

        // Batch 2: chr2 region
        assert_eq!(batches[1].len(), 1);
        assert_eq!(batches[1][0].contig, "chr2");
        assert_eq!(batches[1][0].start, 15);
        assert_eq!(batches[1][0].end, 25);
    }
}
