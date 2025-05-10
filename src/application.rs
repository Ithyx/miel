use thiserror::Error;

use crate::{
    debug::ScopeTimer,
    vk::context::{Context, ContextCreateError, ContextCreateInfo},
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

pub trait ApplicationState {
    fn update(&self, event_loop: &winit::event_loop::ActiveEventLoop);
}

pub struct Application {
    window_create_info: WindowCreationInfo,
    window: Option<winit::window::Window>,

    vulkan_context_create_info: ContextCreateInfo,
    vulkan_context: Option<crate::vk::context::Context>,

    state: Box<dyn ApplicationState>,
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

            vulkan_context_create_info,
            vulkan_context: None,

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
        let _timer = ScopeTimer::new(log::Level::Info, "application resmued step".to_owned());

        match event_loop.create_window(self.window_create_info.clone().into()) {
            Ok(window) => {
                self.vulkan_context = Some(
                    Context::create(&window, &self.vulkan_context_create_info)
                        .expect("context should be creatable"),
                );

                self.window = Some(window);
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

                self.state.update(&event_loop);
                // window.pre_present_notify();
            }

            _ => (),
        }
    }
}
