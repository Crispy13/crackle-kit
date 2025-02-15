use std::{
    env,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use anyhow::Error;
use tracing::{Level, event};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    Layer, filter, fmt::time::ChronoLocal, layer::SubscriberExt, util::SubscriberInitExt,
};

pub fn setup_logging_to_stderr_and_file(
    file_path: impl AsRef<Path>,
    // stderr_log_level: filter::LevelFilter,
) -> Result<(), Error> {
    let stderr_log_level = filter::LevelFilter::INFO;
    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stderr);

    let file_layer = tracing_subscriber::fmt::layer().pretty().with_writer(
        fs::OpenOptions::new()
            .append(true)
            .open(file_path.as_ref())?,
    );

    tracing_subscriber::registry()
        .with(
            stderr_layer
                .with_timer(ChronoLocal::rfc_3339())
                .with_file(false)
                .with_line_number(false)
                .with_target(false)
                .with_filter(stderr_log_level),
        )
        .with(
            file_layer
                .with_timer(ChronoLocal::rfc_3339())
                .with_ansi(false)
                .with_filter(filter::LevelFilter::DEBUG),
        )
        .try_init()?;

    Ok(())
}

fn get_tmp_dir() -> String {
    match env::var("TMPDIR").or_else(|_| env::var("TEMP")) {
        Ok(v) => v,
        Err(_) => "log".into(),
    }
}

fn get_env_filter(level: filter::LevelFilter) -> Result<filter::EnvFilter, Error> {
    let env_filter = match std::env::var("RUST_LOG") {
        Ok(_) => filter::EnvFilter::builder()
            .with_default_directive(level.into())
            .from_env_lossy(),
        Err(_) => {
            let directives = format!(
                "{}={},{}",
                env!("CARGO_PKG_NAME").replace("-", "_"),
                level,
                filter::LevelFilter::OFF,
            );

            filter::EnvFilter::builder().parse(directives)?
            // .add_directive(env!("CARGO_PKG_NAME").replace("-", "_").parse()?)
            // .add_directive(directive.parse()?)
        }
    };

    // eprintln!("env_filter = {}", env_filter);

    Ok(env_filter)
}

/// # Example
/// ```rust
/// let stderr_layer = tracing_subscriber::fmt::layer()
///    .pretty()
/// .with_writer(io::stderr);
///
/// let stderr_log_level = filter::LevelFilter::INFO;
///
/// set_default_options_to_stderr!(stderr_layer, stderr_log_level)
/// ```
///
///
macro_rules! set_default_options_to_stderr {
    ($stderr_layer:ident, $stderr_log_level:ident) => {
        $stderr_layer
            .with_timer(ChronoLocal::rfc_3339())
            .with_file(false)
            .with_line_number(false)
            .with_target(false)
            .with_ansi(false)
            .with_filter($stderr_log_level)
        // .with_filter(get_env_filter(stderr_log_level)?),
    };
}

fn set_default_options_to_stderr<W2>(
    stderr_layer: tracing_subscriber::fmt::Layer<
        tracing_subscriber::Registry,
        tracing_subscriber::fmt::format::Pretty,
        tracing_subscriber::fmt::format::Format<tracing_subscriber::fmt::format::Pretty>,
        W2,
    >,
    stderr_log_level: filter::LevelFilter,
) -> filter::Filtered<
    tracing_subscriber::fmt::Layer<
        tracing_subscriber::Registry,
        tracing_subscriber::fmt::format::Pretty,
        tracing_subscriber::fmt::format::Format<
            tracing_subscriber::fmt::format::Pretty,
            ChronoLocal,
        >,
        W2,
    >,
    filter::LevelFilter,
    tracing_subscriber::Registry,
>
where
    W2: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + 'static,
{
    stderr_layer
        .with_timer(ChronoLocal::rfc_3339())
        .with_file(false)
        .with_line_number(false)
        .with_target(false)
        .with_ansi(false)
        .with_filter(stderr_log_level)
}

pub fn setup_logging_stderr_only(stderr_log_level: filter::LevelFilter) -> Result<(), Error> {
    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stderr);

    Ok(tracing_subscriber::registry()
        .with(set_default_options_to_stderr(
            stderr_layer,
            stderr_log_level,
        ))
        .try_init()?)
}

pub fn setup_logging_to_stderr_and_rolling_file(
    filename_prefix: &str,
    // stderr_log_level: filter::LevelFilter,
) -> Result<(), Error> {
    let stderr_log_level = filter::LevelFilter::INFO;

    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stderr);

    let tmp_dir = get_tmp_dir();

    let file_layer = tracing_subscriber::fmt::layer().pretty().with_writer(
        RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix(filename_prefix)
            .filename_suffix("log")
            .build(&tmp_dir)?,
    );

    tracing_subscriber::registry()
        .with(set_default_options_to_stderr!(
            stderr_layer,
            stderr_log_level
        ))
        .with(
            file_layer
                .with_timer(ChronoLocal::rfc_3339())
                .with_ansi(false)
                .with_filter(filter::LevelFilter::DEBUG), // .with_filter(get_env_filter(filter::LevelFilter::DEBUG)?),
        )
        .try_init()?;

    let log_dir_abs_path = match Path::new(&tmp_dir).canonicalize() {
        Ok(v) => v,
        Err(_) => PathBuf::from(tmp_dir),
    };

    // event!(Level::INFO, "log dir = {}", log_dir_abs_path.display());

    Ok(())
}

pub struct SliceDebugWithNewLine<'a, T: std::fmt::Debug>(&'a [T]);

impl<'a, T: std::fmt::Debug> std::fmt::Debug for SliceDebugWithNewLine<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for elem in self.0 {
            writeln!(f, "{:?}", elem)?
        }

        Ok(())
    }
}

pub trait SliceDebugWithNewLineTrait<T: std::fmt::Debug> {
    fn into_debug_with_newline(&self) -> SliceDebugWithNewLine<'_, T>;
}

impl<'a, T: std::fmt::Debug> SliceDebugWithNewLineTrait<T> for &'a [T] {
    fn into_debug_with_newline(&self) -> SliceDebugWithNewLine<'_, T> {
        SliceDebugWithNewLine(&self)
    }
}

#[cfg(test)]
mod test {
    use tracing::{Level, event};

    use super::*;

    #[test]
    fn test_rolling() {
        // setup_logging_to_stderr_and_file("test.log").unwrap();
        setup_logging_to_stderr_and_rolling_file(env!("CARGO_PKG_NAME")).unwrap();

        event!(Level::TRACE, "trace!");
        event!(Level::DEBUG, "debug!");
        event!(Level::INFO, "info!");
        event!(Level::WARN, "trace!");
        event!(Level::ERROR, "error!");
    }

    #[test]
    fn test_append() {
        setup_logging_to_stderr_and_file("test.log").unwrap();
        // setup_logging_to_stderr_and_rolling_file("crackle-kit").unwrap();

        event!(Level::TRACE, "trace!");
        event!(Level::DEBUG, "debug!");
        event!(Level::INFO, "info!");
        event!(Level::WARN, "trace!");
        event!(Level::ERROR, "error!");
    }
}
