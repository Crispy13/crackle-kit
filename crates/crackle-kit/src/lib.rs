pub mod tracing_kit;
mod err_opt_ext;
mod macros;

pub mod pbar;
mod nuc_base_map;
mod utils;

pub mod data;


// re-export
pub use tracing;
pub use indicatif;

#[cfg(feature="memfd")]
mod memfd_file;

#[cfg(feature="fastq")]
mod fastq;

#[cfg(feature="bam")]
pub mod bam;

#[cfg(feature="bam")]
// re-export
pub use rust_htslib;