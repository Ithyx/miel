pub struct LogFilter;
impl flexi_logger::filter::LogLineFilter for LogFilter {
    fn write(
        &self,
        now: &mut flexi_logger::DeferredNow,
        record: &log::Record,
        log_line_writer: &dyn flexi_logger::filter::LogLineWriter,
    ) -> std::io::Result<()> {
        let should_log = !record.module_path().unwrap_or("").contains("smithay");
        if should_log {
            log_line_writer.write(now, record)?;
        }
        Ok(())
    }
}
pub fn init() -> flexi_logger::LoggerHandle {
    #[cfg(debug_assertions)]
    let log_level = ("debug", flexi_logger::Duplicate::Debug);
    #[cfg(not(debug_assertions))]
    let log_level = ("info", flexi_logger::Duplicate::Info);

    let file_spec = flexi_logger::FileSpec::default().suppress_timestamp();

    flexi_logger::Logger::try_with_env_or_str(log_level.0)
        .expect("Failed to setup logging")
        .log_to_file(file_spec)
        .write_mode(flexi_logger::WriteMode::BufferAndFlush)
        .duplicate_to_stdout(log_level.1)
        .set_palette("b9;3;2;8;7".to_owned())
        .filter(Box::new(LogFilter))
        .start()
        .expect("Failed to build logger")
}
