use thiserror::Error;

use crate::{
    debug::ScopeTimer,
    gfx::context::{Context, ContextCreateError, ContextCreateInfo},
};

#[derive(Debug, Clone)]
pub struct WindowCreationInfo {
    pub title: String,
}

impl From<WindowCreationInfo> for winit::window::WindowAttributes {
    fn from(value: WindowCreationInfo) -> Self {
        Self::default().with_title(value.title)
    }
}

pub enum ControlFlow {
    Continue,
    SwitchState(Box<dyn ApplicationState>),
    Exit,
}

pub trait ApplicationState {
    fn on_attach(&mut self, _ctx: &mut Context) {}

    fn update(&mut self, _ctx: &mut Context) -> ControlFlow {
        ControlFlow::Continue
    }
}

pub struct Application {
    state: Box<dyn ApplicationState>,

    gfx_context_create_info: ContextCreateInfo,
    gfx_context: Option<crate::gfx::context::Context>,

    window_create_info: WindowCreationInfo,
    window: Option<winit::window::Window>,
}

#[derive(Debug, Error)]
pub enum ApplicationBuildError {
    #[error("vulkan context creation failed")]
    VkContextCreation(#[from] ContextCreateError),
}

#[derive(Debug, Error)]
pub enum ApplicationStartError {
    #[error("event loop creation failed")]
    EventLoopCreation(winit::error::EventLoopError),

    #[error("application run failed")]
    ApplicationRun(winit::error::EventLoopError),
}

impl Application {
    pub fn build(
        window_create_info: WindowCreationInfo,
        vulkan_context_create_info: ContextCreateInfo,
        start_state: Box<dyn ApplicationState>,
    ) -> Result<Self, ApplicationBuildError> {
        Ok(Self {
            window_create_info,
            window: None,

            gfx_context_create_info: vulkan_context_create_info,
            gfx_context: None,

            state: start_state,
        })
    }

    pub fn run(mut self) -> Result<(), ApplicationStartError> {
        let event_loop = winit::event_loop::EventLoop::new()
            .map_err(ApplicationStartError::EventLoopCreation)?;

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        event_loop
            .run_app(&mut self)
            .map_err(ApplicationStartError::ApplicationRun)?;

        Ok(())
    }
}

impl winit::application::ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _timer = ScopeTimer::new(log::Level::Info, "application \"resumed\" step".to_owned());

        match event_loop.create_window(self.window_create_info.clone().into()) {
            Ok(window) => {
                self.gfx_context = Some(
                    Context::new(&window, &self.gfx_context_create_info)
                        .expect("context should be creatable"),
                );
                self.window = Some(window);

                self.state.on_attach(self.gfx_context.as_mut().unwrap());
            }
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

                let gfx_ctx = self.gfx_context.as_mut();
                let flow = match gfx_ctx {
                    Some(context) => {
                        let flow = self.state.update(context);

                        context
                            .render_frame(window)
                            .expect("frame should render correctly");

                        flow
                    }
                    _ => {
                        log::warn!("no valid context for update state, skipping");
                        ControlFlow::Continue
                    }
                };

                match flow {
                    ControlFlow::Continue => (),
                    ControlFlow::SwitchState(new_state) => {
                        self.state = new_state;

                        self.state.on_attach(self.gfx_context.as_mut().unwrap());
                    }
                    ControlFlow::Exit => event_loop.exit(),
                }
            }

            _ => (),
        }
    }
}
