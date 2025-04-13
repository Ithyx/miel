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

fn get_version() -> u32 {
    let mut engine_version_numbers = option_env!("CARGO_PKG_VERSION")
        .unwrap_or("1.0.0")
        .split('.')
        .flat_map(|value| value.parse::<u32>())
        .chain(std::iter::repeat(0));
    engine_version_numbers.next().unwrap() << 24
        & engine_version_numbers.next().unwrap() << 16
        & engine_version_numbers.next().unwrap() << 8
        & engine_version_numbers.next().unwrap()
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
        application_name: c"霊夢".to_owned(),
        application_version: get_version(),
    };
    let state = StartupState {};
    let app = application::Application::build(app_config, Box::new(state))
        .expect("app should be buildable");

    app.run().expect("app should be able to run");
}
