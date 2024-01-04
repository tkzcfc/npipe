use flexi_logger::Duplicate;

#[cfg(all(feature = "flexi_log", not(feature = "env_log")))]
static LOGGER_HANDLER: tokio::sync::OnceCell<flexi_logger::LoggerHandle> =
    tokio::sync::OnceCell::const_new();

#[cfg(all(feature = "flexi_log", not(feature = "env_log")))]
pub(crate) fn init_log() -> anyhow::Result<()> {
    use flexi_logger::{Age, Cleanup, Criterion, FileSpec, Logger, Naming, WriteMode};

    let logger = Logger::try_with_str("trace,sqlx=error")?
        .log_to_file(
            FileSpec::default()
                .directory("logs")
                .suppress_timestamp()
                .suffix("log"),
        )
        .duplicate_to_stdout(Duplicate::Debug)
        .format(flexi_logger::opt_format)
        .format_for_stdout(flexi_logger::colored_opt_format)
        .rotate(
            Criterion::AgeOrSize(Age::Day, 1024 * 1024 * 5),
            Naming::Numbers,
            Cleanup::KeepLogFiles(30),
        )
        .print_message()
        .write_mode(WriteMode::Async)
        .start()?;

    LOGGER_HANDLER
        .set(logger)
        .map_err(|_| anyhow::anyhow!("logger set error"))?;

    Ok(())
}

#[cfg(all(feature = "flexi_log", feature = "env_log"))]
pub(crate) fn install_log(syslog: bool) -> anyhow::Result<()> {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Trace)
        .filter_module("sqlx", log::LevelFilter::Error)
        .init();
    Ok(())
}
