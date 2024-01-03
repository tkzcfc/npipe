#[cfg(all(feature = "flexi_log", not(feature = "env_log")))]
static LOGGER_HANDLER: tokio::sync::OnceCell<flexi_logger::LoggerHandle> =
    tokio::sync::OnceCell::const_new();

#[cfg(all(feature = "flexi_log", not(feature = "env_log")))]
pub(crate) fn install_log(syslog: bool) -> anyhow::Result<()> {
    use anyhow::Context;
    use flexi_logger::writers::LogWriter;
    use flexi_logger::{style, DeferredNow, TS_DASHES_BLANK_COLONS_DOT_BLANK};
    use flexi_logger::{Age, Cleanup, Criterion, FileSpec, Logger, Naming, WriteMode};
    use log::{LevelFilter, Record};
    use std::io::Write;
    use std::path::Path;

    struct StdErrLog;

    fn get_file_name(path: Option<&str>) -> anyhow::Result<&str> {
        match path {
            Some(v) => Ok(Path::new(v)
                .file_name()
                .context("<unnamed>")?
                .to_str()
                .context("<unnamed>")?),
            None => Ok("<unnamed>"),
        }
    }

    impl LogWriter for StdErrLog {
        #[inline]
        fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
            let level = record.level();
            write!(
                std::io::stderr(),
                "[{} {} {}:{}] {}\r\n",
                now.format(TS_DASHES_BLANK_COLONS_DOT_BLANK),
                style(level).paint(level.to_string()),
                get_file_name(record.file()).unwrap_or("<unnamed>"),
                record.line().unwrap_or(0),
                record.args()
            )
        }

        fn flush(&self) -> std::io::Result<()> {
            std::io::stderr().flush()
        }

        fn max_log_level(&self) -> LevelFilter {
            log::LevelFilter::Trace
        }
    }

    if syslog {
        let logger = Logger::try_with_str("trace,sqlx=error")?
            .log_to_file_and_writer(
                FileSpec::default()
                    .directory("logs")
                    .suppress_timestamp()
                    .suffix("log"),
                Box::new(StdErrLog),
            )
            .format(flexi_logger::opt_format)
            .rotate(
                Criterion::AgeOrSize(Age::Day, 1024 * 1024 * 5),
                Naming::Numbers,
                Cleanup::KeepLogFiles(30),
            )
            .print_message()
            .set_palette("196;190;2;4;8".into())
            .write_mode(WriteMode::Async)
            .start()?;

        LOGGER_HANDLER
            .set(logger)
            .map_err(|_| anyhow::anyhow!("logger set error"))?;
    } else {
        let logger = Logger::try_with_str("trace,sqlx=error")?
            .log_to_file(
                FileSpec::default()
                    .directory("logs")
                    .suppress_timestamp()
                    .suffix("log"),
            )
            .format(flexi_logger::opt_format)
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
    }

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
