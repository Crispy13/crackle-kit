pub mod tracing_kit;
mod err_opt_ext;
mod macros;

// re-export
pub use tracing;

#[cfg(feature="memfd")]
mod memfd_file;

#[cfg(feature="fastq")]
mod fastq;

mod pbar;
mod nuc_base_map;
mod utils;

pub mod data;

#[cfg(feature="bam")]
pub mod bam;

#[cfg(feature="bam")]
// re-export
pub use rust_htslib;