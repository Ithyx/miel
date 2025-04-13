use miel::application;

fn init_logging() {
    #[cfg(debug_assertions)]
    let log_level = ("debug", flexi_logger::Duplicate::Debug);
    #[cfg(not(debug_assertions))]
    let log_level = ("info", flexi_logger::Duplicate::Info);

    let file_spec = flexi_logger::FileSpec::default().suppress_timestamp();

    let _logger = flexi_logger::Logger::try_with_env_or_str(log_level.0)
        .expect("Failed to setup logging")
        .log_to_file(file_spec)
        .write_mode(flexi_logger::WriteMode::BufferAndFlush)
        .duplicate_to_stdout(log_level.1)
        .set_palette("b9;3;2;8;7".to_owned())
        .start()
        .expect("Failed to build logger");
}

struct StartupState {}
impl application::ApplicationState for StartupState {
    fn update(&self) {
        log::info!("UPDATE")
    }
}

fn main() {
    init_logging();

    let app_config = application::WindowCreationData {
        title: "reime".to_owned(),
    };
    let state = StartupState {};
    let app = application::Application::build(app_config, Box::new(state));

    let _ = app.run();
}
