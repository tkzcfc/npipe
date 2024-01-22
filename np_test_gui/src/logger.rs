use flexi_logger::{Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming, WriteMode};
use std::env;

static LOGGER_HANDLER: tokio::sync::OnceCell<flexi_logger::LoggerHandle> =
    tokio::sync::OnceCell::const_new();

pub(crate) fn init_logger() -> anyhow::Result<()> {
    env::set_var("RUST_BACKTRACE", "1");

    // 日志初始化
    let logger = Logger::try_with_str(
        "trace,eframe=error,egui_glow=error,egui-winit=error,egui=error,mio=error",
    )?
    .log_to_file(
        FileSpec::default()
            .directory("logs")
            .suppress_timestamp()
            .suffix("log"),
    )
    .duplicate_to_stdout(Duplicate::Info)
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
        .map_err(|err| anyhow::anyhow!("logger set error: {}", err))?;

    Ok(())
}
