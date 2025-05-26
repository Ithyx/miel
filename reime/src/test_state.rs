pub struct TestState {}

impl TestState {
    pub fn new(_ctx: &mut miel::vk::context::Context) -> Self {
        Self {}
    }
}

impl miel::application::ApplicationState for TestState {
    fn update(&self, _context: &mut miel::vk::context::Context) -> miel::application::ControlFlow {
        log::info!("update !");
        log::info!("...and exit.");

        miel::application::ControlFlow::Exit
    }
}
