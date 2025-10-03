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

use crate::data::region::GenomeRegion;


pub trait BamLocusWorker {
    fn work_for_locus<'a, T>(&self, region:GenomeRegion<'a>) -> T;
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
struct ParallelLocusProcessor<B: BamLocusWorker> {
    bam_locus_worker: B,
    n_jobs: usize,
}

