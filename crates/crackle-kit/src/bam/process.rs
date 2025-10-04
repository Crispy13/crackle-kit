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

use std::{collections::HashSet, i32, path::PathBuf};

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

use crate::{data::region::GenomeRegion, utils::batch_region::batch_region};

pub trait BamLocusWorker: Send + Sync {
    type Output: Send + Sync;

    fn work_for_locus(&self, plp: Pileup) -> Self::Output;
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
struct ParallelLocusProcessor<W: BamLocusWorker> {
    bam_locus_worker: W,
    n_threads: usize,
    bam_path: PathBuf,
}

impl<W: BamLocusWorker> ParallelLocusProcessor<W> {
    fn process_with_batch<'a>(
        &self,
        regions: Vec<GenomeRegion<'a>>,
        batch_window_size: usize,
    ) -> Result<Vec<W::Output>, Error> {
        // make batch
        let batched_regions = batch_region(regions.into_iter(), batch_window_size);

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
                .par_iter()
                .map(|batch| {
                    if batch.is_empty() {
                        return Ok(vec![]);
                    }

                    let mut ir = IndexedReader::from_path(&self.bam_path)?;

                    let batch_contig = batch.first().unwrap().contig.as_str();
                    let batch_pileup_start = batch.first().unwrap().start - 1; // batch is not empty, by the if condition of function start point.
                    let batch_pileup_end = batch.last().unwrap().end - 1;

                    ir.fetch((batch_contig, batch_pileup_start, batch_pileup_end))?;
                    let pileups = ir.pileup_with_option(PileupOption {
                        max_depth: i32::MAX,
                        ignore_overlaps: true,
                    });

                    let mut target_pos = batch.iter().map(|g| g.start - 1).collect::<HashSet<_>>();

                    let mut res = Vec::with_capacity(target_pos.len());

                    for plp_r in pileups {
                        let plp = plp_r?;

                        if !target_pos.remove(&(plp.pos() as i64)) {
                            continue;
                        }

                        let r = self.bam_locus_worker.work_for_locus(plp);

                        res.push(r);

                        if target_pos.is_empty() {
                            break;
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

    impl BamLocusWorker for MeanBPWorker {
        type Output = f64;

        fn work_for_locus(&self, plp: Pileup) -> Self::Output {
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

            bq_sum as f64 / len as f64
        }
    }

    #[test]
    fn parallel_locus_processor1() -> Result<(), Box<dyn std::error::Error>> {
        setup_logging_stderr_only_debug(LevelFilter::DEBUG)?;

        let bam_path = "/home/eck/workspace/common_data/NA12878.chrom11.ILLUMINA.bwa.CEU.low_coverage.20121211.bam";
        let plp = ParallelLocusProcessor {
            bam_locus_worker: MeanBPWorker,
            n_threads: 4,
            bam_path: bam_path.into(),
        };

        let regions = (60000..(60000 + 1_000_000))
            .step_by(1000)
            .map(|p| GenomeRegion {
                contig: Chrom::Other("11".into()),
                start: p,
                end: p + 1,
            })
            .collect::<Vec<_>>();

        let r = plp.process_with_batch(regions, 100_000)?;

        eprintln!("{} {:?}", r.len(), &r[..10]);

        Ok(())
    }
}
