use std::fmt::Write;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle};

/// Make `indicatif::ProgressBar` instance.
/// 
/// You can give `0` to `len` argument, then a question mark will be on progress bar.
pub fn prepare_pbar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);

    pb.set_draw_target(ProgressDrawTarget::stderr_with_hz(8));

    let template = match len {
        1.. => {
            "{spinner:.green} [{elapsed_precise}] {msg} [{bar:.cyan/blue}] {pos}/{len} ({eta}, {per_sec})"
            // "{spinner:.green} [{elapsed_precise}] {msg} [{bar}] {pos}/{len} ({eta}, {per_sec})"
        }
        0 => {
            // "{spinner:.green} [{elapsed_precise}] {msg} [ ? ] {pos} ({per_sec})"
            "{spinner:.green} [{elapsed_precise}] [ ? ] {msg} ({per_sec})"
        }
    };

    pb.set_style(
        ProgressStyle::with_template(template).unwrap().with_key(
            "eta",
            |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            },
        ), // .with_key("bases", |state: &ProgressState, w: &mut dyn Write| {
           //     write!(
           //         w,
           //         "{:.1}Gb ({} bases)",
           //         state.pos() as f64 / 10_f64.powi(9),
           //         state.pos()
           //     )
           //     .unwrap()
           // }), // .progress_chars("#>-"),
    );

    pb
}
