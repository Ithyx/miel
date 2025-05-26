use miel::{application, gfx};

pub struct TestState {}

impl TestState {
    pub fn new(_ctx: &mut gfx::context::Context) -> Self {
        Self {}
    }
}

impl application::ApplicationState for TestState {
    fn update(&self, _context: &mut gfx::context::Context) -> miel::application::ControlFlow {
        log::info!("update !");
        log::info!("...and exit.");

        miel::application::ControlFlow::Exit
    }
}
