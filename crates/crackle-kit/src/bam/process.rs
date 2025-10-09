//! ## Bam file process helpers
//!
//! 2 Processes are supported:
//! 1. Make a modified bam (with Producer and Consumer)
//! 2. Do a given task per genomic positions (in parallel)
//!     Examples)
//!     a. Pileup each variant position and get reads.
//!
//!
//!

/*
251009 TODO: Refactor ParallelLocusProcessor for Correctness and Performance
The current implementation has a critical bug: the HashMap logic only checks the start coordinate of a region. This must be replaced to correctly handle all sites.

The plan is to enforce a 1-bp site constraint and replace the core algorithm with a "merge/zip" (sweep-line) approach.

Algorithm:
Inside the rayon::map closure, process the sorted pileup iterator and the sorted batch of inputs in a single pass. Use peekable() on both iterators and match on the cmp() result of their positions to handle the three cases:

pileup < site: Advance the pileup iterator.

pileup > site: Advance the site iterator.

pileup == site: Process the match and advance both iterators.
*/

use std::{
    cmp::Ordering, collections::{HashMap, HashSet}, i32, path::PathBuf
};

use anyhow::Error;
use rayon::{
    ThreadPoolBuilder,
    iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator},
};
use rust_htslib::bam::{
    IndexedReader,
    pileup::{Pileup, PileupOption},
};
use tracing::{Level, event};

use crate::{
    data::locus::{GenomeCoordinate, GenomeRegion},
    utils::batch_region::batch_region,
};

pub trait BamLocusWorker<'a>: Send + Sync {
    type Input: BamLocusWorkInput<'a>;
    type Output: Send + Sync;
    type Error: Into<Error>;

    fn work_for_locus(&self, plp: Pileup, input: Self::Input) -> Result<Self::Output, Self::Error>;
}

pub trait BamLocusWorkInput<'a>: Send + Sync {
    fn genome_coordinate(&self) -> &GenomeCoordinate<'a>;
}

impl<'a> BamLocusWorkInput<'a> for GenomeCoordinate<'a> {
    fn genome_coordinate(&self) -> &GenomeCoordinate<'a> {
        self
    }
}

fn batch_input_by_coordinate<'a, I: BamLocusWorkInput<'a>>(
    inputs: impl IntoIterator<Item = I>,
    window_size: usize,
) -> Vec<Vec<I>> {
    let mut input_iter = inputs.into_iter();
    let mut c_vec = vec![];
    let (mut c_contig, mut c_start) = match input_iter.next() {
        Some(inp) => {
            let gr = inp.genome_coordinate();

            let contig_clone = gr.contig.clone();
            let start_val = gr.pos;
            c_vec.push(inp);
            (contig_clone, start_val)
        }
        None => return vec![],
    };

    let mut res: Vec<Vec<I>> = vec![];
    for inp in input_iter {
        let gc = inp.genome_coordinate();
        // Condition to start a new batch:
        // 1. The contig changes.
        // 2. The span from the batch's start to the current region's end exceeds window_size.
        if c_contig == gc.contig && gc.pos - c_start < window_size as i64 {
            c_vec.push(inp);
        } else {
            res.push(c_vec); // Push the completed batch
            c_contig = gc.contig.clone(); // Start a new batch: reset contig and start
            c_start = gc.pos;
            c_vec = vec![inp]; // Start new batch with current region
        }
    }

    // Push any remaining regions in the last batch
    if !c_vec.is_empty() {
        res.push(c_vec);
    }

    res
}

///
/// # Example
/// ```
/// struct MeanBPWorker {
/// }
///
/// impl BamLocusWorker for MeanBPWorker {
///     fn work_for_locus() {
///         // your code here
///     }
/// }
///
///
///
///
/// ```
pub struct ParallelLocusProcessor<W: for<'a> BamLocusWorker<'a>> {
    bam_locus_worker: W,
    n_threads: usize,
    bam_path: PathBuf,
}

impl<W: for<'a> BamLocusWorker<'a>> ParallelLocusProcessor<W> {
    pub fn new(bam_locus_worker: W, n_threads: usize, bam_path: PathBuf) -> Self {
        Self {
            bam_locus_worker,
            n_threads,
            bam_path,
        }
    }

    pub fn process_with_batch<'a>(
        &self,
        inputs: Vec<<W as BamLocusWorker<'a>>::Input>,
        batch_window_size: usize,
    ) -> Result<Vec<<W as BamLocusWorker<'a>>::Output>, Error> {
        // make batch
        let batched_regions = batch_input_by_coordinate(inputs.into_iter(), batch_window_size);

        event!(
            Level::DEBUG,
            "batched_regions len={}",
            batched_regions.len()
        );

        // open threadpool and distribute the jobs.
        let tp = ThreadPoolBuilder::new()
            .num_threads(self.n_threads)
            .build()?;

        let batch_res = tp.scope(|_scope| {
            let r = batched_regions
                .into_par_iter()
                .map(|batch| {
                    if batch.is_empty() {
                        return Ok(vec![]);
                    }

                    let mut ir = IndexedReader::from_path(&self.bam_path)?;

                    let first_elem = batch.first().unwrap();
                    let last_elem = batch.last().unwrap();

                    let batch_contig = first_elem.genome_coordinate().contig.as_str();
                    let batch_pileup_start = first_elem.genome_coordinate().pos - 1; // batch is not empty, by the if condition of function start point.
                    let batch_pileup_end = last_elem.genome_coordinate().pos;

                    ir.fetch((batch_contig, batch_pileup_start, batch_pileup_end))?;
                    let mut pileups = ir
                        .pileup_with_option(PileupOption {
                            max_depth: i32::MAX,
                            ignore_overlaps: true,
                        })
                        .peekable();

                    // Create peekable iterators for both the pileups and the batch of inputs.
                    let mut res = Vec::with_capacity(batch.len());
                    
                    let mut batch_peekable = batch.into_iter().peekable();

                    // This is the efficient "merge/zip" sweep-line algorithm
                    while let (Some(Ok(pileup_col)), Some(input)) =
                        (pileups.peek(), batch_peekable.peek())
                    {
                        let pileup_pos = pileup_col.pos() as i64;
                        // Assuming you've updated the trait to use GenomeCoordinate
                        let target_pos = input.genome_coordinate().pos - 1;

                        match pileup_pos.cmp(&target_pos) {
                            Ordering::Less => {
                                // Case 1: Pileup is before our target site.
                                // Discard the pileup and advance the pileup iterator.
                                pileups.next();
                            }
                            Ordering::Greater => {
                                // Case 2: We've passed our target site, but there was no pileup (zero coverage).
                                // Discard the target and advance the site iterator.
                                batch_peekable.next();
                            }
                            Ordering::Equal => {
                                // Case 3: Match found! Process it.
                                // We must consume both items from the iterators to advance.
                                if let (Some(Ok(plp)), Some(inp)) =
                                    (pileups.next(), batch_peekable.next())
                                {
                                    let r = self
                                        .bam_locus_worker
                                        .work_for_locus(plp, inp)
                                        .map_err(|err| err.into())?;
                                    res.push(r);
                                }
                            }
                        }
                    }

                    Ok::<_, Error>(res)
                })
                .collect::<Result<Vec<_>, Error>>()?
                .into_par_iter()
                .flatten_iter()
                .collect::<Vec<_>>();

            Ok::<_, Error>(r)
        })?;

        Ok(batch_res)
    }
}

#[cfg(test)]
mod tests {
    use tracing::level_filters::LevelFilter;

    use super::*;
    use crate::{
        bam::process::BamLocusWorker, data::chrom::Chrom, tracing::setup_logging_stderr_only_debug,
    };

    struct MeanBPWorker;

    impl<'a> BamLocusWorker<'a> for MeanBPWorker {
        type Output = f64;
        type Input = GenomeCoordinate<'a>;
        type Error = Error;

        fn work_for_locus(
            &self,
            plp: Pileup,
            inp: Self::Input,
        ) -> Result<Self::Output, Self::Error> {
            let mut bq_sum: u64 = 0;
            let alignments = plp.alignments();
            let len = alignments.len();
            for alignment in alignments {
                let qpos = match alignment.qpos() {
                    Some(qpos) => qpos,
                    None => continue,
                };

                let record = alignment.record();

                let bq = record.qual().get(qpos).unwrap();
                bq_sum += *bq as u64;
            }

            Ok(bq_sum as f64 / len as f64)
        }
    }

    #[test]
    fn parallel_locus_processor1() -> Result<(), Box<dyn std::error::Error>> {
        setup_logging_stderr_only_debug(LevelFilter::DEBUG)?;

        let bam_path = "/home/eck/workspace/common_resources/NA12878.chrom20.ILLUMINA.bwa.CEU.low_coverage.20121211.bam";
        let plp = ParallelLocusProcessor {
            bam_locus_worker: MeanBPWorker,
            n_threads: 4,
            bam_path: bam_path.into(),
        };

        let regions = (60000..(60000 + 1_000_000))
            .step_by(1000)
            .map(|p| GenomeCoordinate {
                contig: Chrom::Other("20".into()),
                pos: p,
            })
            .collect::<Vec<_>>();

        let r = plp.process_with_batch(regions, 100_000)?;

        eprintln!("{} {:?}", r.len(), &r[..10]);

        Ok(())
    }
}
