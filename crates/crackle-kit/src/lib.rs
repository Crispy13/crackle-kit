
mod err_opt_ext;
mod macros;


mod nuc_base_map;
mod utils;

pub mod data;

#[cfg(feature="tracing")]
pub mod tracing_kit;
#[cfg(feature="tracing")]
pub use tracing;

#[cfg(feature="pbar")]
pub mod pbar;
#[cfg(feature="pbar")]
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