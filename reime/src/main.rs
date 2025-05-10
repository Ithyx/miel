mod logging;

use miel::{application, vk};

fn get_version() -> u32 {
    let mut engine_version_numbers = option_env!("CARGO_PKG_VERSION")
        .unwrap_or("1.0.0")
        .split('.')
        .flat_map(|value| value.parse::<u32>())
        .chain(std::iter::repeat(0));
    engine_version_numbers.next().unwrap() << 24
        | engine_version_numbers.next().unwrap() << 16
        | engine_version_numbers.next().unwrap() << 8
        | engine_version_numbers.next().unwrap()
}

struct StartupState {}
impl application::ApplicationState for StartupState {
    fn update(&self, event_loop: &miel::winit::event_loop::ActiveEventLoop) {
        log::info!("Update !");
        log::info!("... and exit.");
        event_loop.exit();
    }
}

fn main() {
    let _logger_handle = logging::init();

    let app_info = application::WindowCreationInfo {
        title: "霊夢".to_owned(),
    };
    let vk_info = vk::context::ContextCreateInfo {
        application_name: c"霊夢".to_owned(),
        application_version: get_version(),
    };
    let state = StartupState {};
    let app = application::Application::build(app_info, vk_info, Box::new(state))
        .expect("app should be buildable");

    app.run().expect("app should be able to run");
}
