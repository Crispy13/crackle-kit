use std::{
    any::Any,
    collections::HashMap,
    env,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use anyhow::{Error, anyhow};
use tracing::{Level, event, level_filters::LevelFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    EnvFilter, Layer, filter, fmt::time::ChronoLocal, layer::SubscriberExt, reload,
    util::SubscriberInitExt,
};

// use crate::err_opt_ext::{HashMapExt, impl_option_handle_trait};
// TODO: replace levelfilter with envfilter(level filter included)

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
    EnvFilter,
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
        .with_filter(
            EnvFilter::builder()
                .with_default_directive(stderr_log_level.into())
                .from_env_lossy(),
        )
    // .with_filter(stderr_log_level)
}

fn set_default_options_to_stderr_debug<W2>(
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
    EnvFilter,
    tracing_subscriber::Registry,
>
where
    W2: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + 'static,
{
    stderr_layer
        .with_timer(ChronoLocal::rfc_3339())
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_ansi(false)
        .with_filter(
            EnvFilter::builder()
                .with_default_directive(stderr_log_level.into())
                .from_env_lossy(),
        )
    // .with_filter(stderr_log_level)
}

pub fn setup_logging_stderr_only(
    stderr_log_level: filter::LevelFilter,
) -> Result<
    reload::Handle<
        filter::Filtered<
            tracing_subscriber::fmt::Layer<
                tracing_subscriber::Registry,
                tracing_subscriber::fmt::format::Pretty,
                tracing_subscriber::fmt::format::Format<
                    tracing_subscriber::fmt::format::Pretty,
                    ChronoLocal,
                >,
                impl Fn() -> io::Stderr,
            >,
            EnvFilter,
            tracing_subscriber::Registry,
        >,
        tracing_subscriber::Registry,
    >,
    Error,
>
// where
//     W2: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + 'static,
{
    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stderr);

    let filtered_layer = set_default_options_to_stderr(stderr_layer, stderr_log_level);

    let (layer, reload_handle) = reload::Layer::new(filtered_layer);

    tracing_subscriber::registry().with(layer).try_init()?;

    // reload_handle.modify(|filter| {
    //     *filter.filter_mut() = LevelFilter::DEBUG;
    // })?;

    Ok(reload_handle)
}

#[deprecated = "Renamed. Use `setup_logging_stderr_only_verbose` instead."]
pub fn setup_logging_stderr_only_debug(
    stderr_log_level: filter::LevelFilter,
) -> Result<
    reload::Handle<
        filter::Filtered<
            tracing_subscriber::fmt::Layer<
                tracing_subscriber::Registry,
                tracing_subscriber::fmt::format::Pretty,
                tracing_subscriber::fmt::format::Format<
                    tracing_subscriber::fmt::format::Pretty,
                    ChronoLocal,
                >,
                impl Fn() -> io::Stderr,
            >,
            EnvFilter,
            tracing_subscriber::Registry,
        >,
        tracing_subscriber::Registry,
    >,
    Error,
> {
    setup_logging_stderr_only_verbose(stderr_log_level)
}

pub fn setup_logging_stderr_only_verbose(
    stderr_log_level: filter::LevelFilter,
) -> Result<
    reload::Handle<
        filter::Filtered<
            tracing_subscriber::fmt::Layer<
                tracing_subscriber::Registry,
                tracing_subscriber::fmt::format::Pretty,
                tracing_subscriber::fmt::format::Format<
                    tracing_subscriber::fmt::format::Pretty,
                    ChronoLocal,
                >,
                impl Fn() -> io::Stderr,
            >,
            EnvFilter,
            tracing_subscriber::Registry,
        >,
        tracing_subscriber::Registry,
    >,
    Error,
>
// where
//     W2: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + 'static,
{
    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(io::stderr);

    let filtered_layer = set_default_options_to_stderr_debug(stderr_layer, stderr_log_level);

    let (layer, reload_handle) = reload::Layer::new(filtered_layer);

    tracing_subscriber::registry().with(layer).try_init()?;

    // reload_handle.modify(|filter| {
    //     *filter.filter_mut() = LevelFilter::DEBUG;
    // })?;

    Ok(reload_handle)
}

pub fn setup_logging_to_stderr_and_rolling_file(
    filename_prefix: &str,
    // stderr_log_level: filter::LevelFilter,
) -> Result<
    (
        reload::Handle<
            filter::Filtered<
                tracing_subscriber::fmt::Layer<
                    tracing_subscriber::Registry,
                    tracing_subscriber::fmt::format::Pretty,
                    tracing_subscriber::fmt::format::Format<
                        tracing_subscriber::fmt::format::Pretty,
                        ChronoLocal,
                    >,
                    impl Fn() -> io::Stderr,
                >,
                EnvFilter,
                tracing_subscriber::Registry,
            >,
            tracing_subscriber::Registry,
        >,
        reload::Handle<
            filter::Filtered<
                tracing_subscriber::fmt::Layer<
                    tracing_subscriber::layer::Layered<
                        reload::Layer<
                            filter::Filtered<
                                tracing_subscriber::fmt::Layer<
                                    tracing_subscriber::Registry,
                                    tracing_subscriber::fmt::format::Pretty,
                                    tracing_subscriber::fmt::format::Format<
                                        tracing_subscriber::fmt::format::Pretty,
                                        ChronoLocal,
                                    >,
                                    impl Fn() -> io::Stderr,
                                >,
                                EnvFilter,
                                tracing_subscriber::Registry,
                            >,
                            tracing_subscriber::Registry,
                        >,
                        tracing_subscriber::Registry,
                    >,
                    tracing_subscriber::fmt::format::Pretty,
                    tracing_subscriber::fmt::format::Format<
                        tracing_subscriber::fmt::format::Pretty,
                        ChronoLocal,
                    >,
                    RollingFileAppender,
                >,
                LevelFilter,
                tracing_subscriber::layer::Layered<
                    reload::Layer<
                        filter::Filtered<
                            tracing_subscriber::fmt::Layer<
                                tracing_subscriber::Registry,
                                tracing_subscriber::fmt::format::Pretty,
                                tracing_subscriber::fmt::format::Format<
                                    tracing_subscriber::fmt::format::Pretty,
                                    ChronoLocal,
                                >,
                                impl Fn() -> io::Stderr,
                            >,
                            EnvFilter,
                            tracing_subscriber::Registry,
                        >,
                        tracing_subscriber::Registry,
                    >,
                    tracing_subscriber::Registry,
                >,
            >,
            tracing_subscriber::layer::Layered<
                reload::Layer<
                    filter::Filtered<
                        tracing_subscriber::fmt::Layer<
                            tracing_subscriber::Registry,
                            tracing_subscriber::fmt::format::Pretty,
                            tracing_subscriber::fmt::format::Format<
                                tracing_subscriber::fmt::format::Pretty,
                                ChronoLocal,
                            >,
                            impl Fn() -> io::Stderr,
                        >,
                        EnvFilter,
                        tracing_subscriber::Registry,
                    >,
                    tracing_subscriber::Registry,
                >,
                tracing_subscriber::Registry,
            >,
        >,
    ),
    Error,
> {
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

    let (stderr_layer2, stderr_layer_handler) = reload::Layer::new(set_default_options_to_stderr(
        stderr_layer,
        stderr_log_level,
    ));

    let (file_layer2, filelayer_handler) = reload::Layer::new(
        file_layer
            .with_timer(ChronoLocal::rfc_3339())
            .with_ansi(false)
            .with_filter(filter::LevelFilter::DEBUG),
    );

    tracing_subscriber::registry()
        .with(stderr_layer2)
        .with(
            file_layer2, // .with_filter(get_env_filter(filter::LevelFilter::DEBUG)?),
        )
        .try_init()?;

    let log_dir_abs_path = match Path::new(&tmp_dir).canonicalize() {
        Ok(v) => v,
        Err(_) => PathBuf::from(tmp_dir),
    };

    // event!(Level::INFO, "log dir = {}", log_dir_abs_path.display());

    Ok((stderr_layer_handler, filelayer_handler))
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

pub enum TracingFilterMut<'a> {
    LevelFilter(&'a mut LevelFilter),
    EnvFilter(&'a mut EnvFilter),
}

pub trait FilteredModifier {
    fn filter_mut(&mut self) -> TracingFilterMut<'_>;
}

impl<L, S> FilteredModifier for filter::Filtered<L, LevelFilter, S> {
    fn filter_mut(&mut self) -> TracingFilterMut<'_> {
        TracingFilterMut::LevelFilter(self.filter_mut())
    }
}

impl<L, S> FilteredModifier for filter::Filtered<L, EnvFilter, S> {
    fn filter_mut(&mut self) -> TracingFilterMut<'_> {
        TracingFilterMut::EnvFilter(self.filter_mut())
    }
}

pub trait ReloadHandler {
    fn modify_any(
        &self,
        f: Box<dyn FnOnce(&mut dyn FilteredModifier)>,
    ) -> Result<(), reload::Error>;
}

impl<L: FilteredModifier + 'static, S> ReloadHandler for reload::Handle<L, S> {
    fn modify_any(
        &self,
        f: Box<dyn FnOnce(&mut dyn FilteredModifier)>,
    ) -> Result<(), reload::Error> {
        // Downcast the mutable reference from `Any` back to `L`
        self.modify(|l: &mut L| {
            f(l);
        })
    }
}

#[derive(Default)]
pub struct TracingControlTower {
    handler_map: Mutex<HashMap<String, Box<dyn ReloadHandler + Send>>>,
}

impl TracingControlTower {
    pub fn add_handler(
        &self,
        name: String,
        handler: Box<dyn ReloadHandler + Send>,
    ) -> Result<(), Error> {
        self.handler_map
            .lock()
            .map_err(|err| anyhow!("{err:?}"))?
            .insert(name, handler);

        Ok(())
    }

    pub fn modify_handler(
        &self,
        name: &str,
        f: impl FnOnce(&mut dyn FilteredModifier) + 'static,
    ) -> Result<(), Error> {
        let box_f = Box::new(f);

        let mut map = self.handler_map.lock().map_err(|err| anyhow!("{err:?}"))?;

        map.get_mut(name)
            .ok_or_else(|| anyhow!("Key {} not found", name))?
            .modify_any(box_f)?;

        Ok(())
    }
}

pub fn global_tracing_control_tower() -> &'static TracingControlTower {
    static CC: OnceLock<TracingControlTower> = OnceLock::new();
    CC.get_or_init(|| TracingControlTower::default())
}

#[cfg(test)]
mod test {
    use tracing::{Level, event};

    use super::*;
    
    #[test]
    fn test_cc() -> Result<(), Box<dyn std::error::Error>> {
        let cc = TracingControlTower::default();

        let sh = setup_logging_stderr_only(LevelFilter::DEBUG)?;

        cc.add_handler("stderr".to_owned(), Box::new(sh))?;

        event!(Level::DEBUG, "this will be showed!");

        event!(Level::INFO, "INFO");

        cc.modify_handler("stderr", |v| match v.filter_mut() {
            TracingFilterMut::LevelFilter(level_filter) => {
                *level_filter = LevelFilter::INFO;
            }
            TracingFilterMut::EnvFilter(env_filter) => {
                *env_filter = EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy();
            }
        })?;

        event!(Level::DEBUG, "this will be not showed!");
        event!(Level::INFO, "after changing level to INFO, INFO!");

        cc.modify_handler("stderr", |v| match v.filter_mut() {
            TracingFilterMut::LevelFilter(level_filter) => {
                *level_filter = LevelFilter::DEBUG;
            }
            TracingFilterMut::EnvFilter(env_filter) => {
                *env_filter = EnvFilter::builder()
                    .with_default_directive(LevelFilter::DEBUG.into())
                    .from_env_lossy();
            }
        })?;

        event!(Level::DEBUG, "Debug will be showed again!");
        event!(Level::INFO, "after changing level to DEBUG, INFO!");

        Ok(())
    }

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
