use std::{
    env,
    fs::File,
    io,
    path::{Path, PathBuf},
};

use anyhow::Error;
use tracing::{event, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    filter, fmt::time::ChronoLocal, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};

pub fn setup_logging_to_stderr_and_file(
    file_path: impl AsRef<Path>,
    // stderr_log_level: filter::LevelFilter,
) -> Result<(), Error> {
    let stderr_log_level = filter::LevelFilter::INFO;
    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stderr);

    let file_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(File::create(file_path.as_ref())?);

    tracing_subscriber::registry()
        .with(
            stderr_layer
                .with_timer(ChronoLocal::rfc_3339())
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
        .with(
            stderr_layer
                .with_timer(ChronoLocal::rfc_3339())
                .with_filter(stderr_log_level),
        )
        .with(
            file_layer
                .with_timer(ChronoLocal::rfc_3339())
                .with_ansi(false)
                .with_filter(filter::LevelFilter::DEBUG),
        )
        .try_init()?;

    let log_dir_abs_path = match Path::new(&tmp_dir).canonicalize() {
        Ok(v) => v,
        Err(_) => PathBuf::from(tmp_dir),
    };

    event!(Level::INFO, "log dir = {}", log_dir_abs_path.display());

    Ok(())
}

#[cfg(test)]
mod test {
    use tracing::{event, Level};

    use super::*;

    #[test]
    fn test_init() {
        // setup_logging_to_stderr_and_file("test.log").unwrap();
        setup_logging_to_stderr_and_rolling_file("crackle-kit").unwrap();

        event!(Level::DEBUG, "debug!");
        event!(Level::INFO, "info!");
        event!(Level::TRACE, "trace!");
        event!(Level::ERROR, "error!");
    }
}
