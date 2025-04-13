use thiserror::Error;

#[derive(Debug, Clone)]
pub struct WindowCreationData {
    pub title: String,
}

impl From<WindowCreationData> for winit::window::WindowAttributes {
    fn from(value: WindowCreationData) -> Self {
        Self::default().with_title(value.title)
    }
}

pub trait ApplicationState {
    fn update(&self);
}

pub struct Application {
    window_creation_data: WindowCreationData,
    window: Option<winit::window::Window>,

    state: Box<dyn ApplicationState>,
}

#[derive(Debug, Error)]
pub enum ApplicationStartError {
    #[error("event llop creation failed")]
    EventLoopCreationFail(winit::error::EventLoopError),

    #[error("application run failed")]
    ApplicationRunFail(winit::error::EventLoopError),
}

impl Application {
    pub fn build(create_data: WindowCreationData, start_state: Box<dyn ApplicationState>) -> Self {
        Self {
            window_creation_data: create_data,
            window: None,
            state: start_state,
        }
    }

    pub fn run(mut self) -> Result<(), ApplicationStartError> {
        let event_loop = winit::event_loop::EventLoop::new()
            .map_err(ApplicationStartError::EventLoopCreationFail)?;

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        event_loop
            .run_app(&mut self)
            .map_err(ApplicationStartError::ApplicationRunFail)?;

        Ok(())
    }
}

impl winit::application::ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        match event_loop.create_window(self.window_creation_data.clone().into()) {
            Ok(handle) => self.window = Some(handle),
            Err(e) => {
                log::error!("failed to create window after resume event: {e}");
                todo!()
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();
                window.request_redraw();

                self.state.update();
                // window.pre_present_notify();
            }

            _ => (),
        }
    }
}
