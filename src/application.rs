use std::ffi::CString;

use thiserror::Error;

use ash::vk;

use crate::debug::ScopeTimer;

#[derive(Debug, Clone)]
pub struct WindowCreationData {
    // For the Vulkan API
    pub application_name: CString,
    pub application_version: u32,

    // For the window
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

    entry: ash::Entry,
    instance: ash::Instance,
}

#[derive(Debug, Error)]
pub enum ApplicationBuildError {
    #[error("vulkan library loading failed")]
    VulkanLoadFail(#[from] ash::LoadingError),

    #[error("instance creation failed")]
    InstanceCreationFail(vk::Result),
}

#[derive(Debug, Error)]
pub enum ApplicationStartError {
    #[error("event loop creation failed")]
    EventLoopCreationFail(winit::error::EventLoopError),

    #[error("application run failed")]
    ApplicationRunFail(winit::error::EventLoopError),
}

impl Application {
    pub fn build(
        create_data: WindowCreationData,
        start_state: Box<dyn ApplicationState>,
    ) -> Result<Self, ApplicationBuildError> {
        let _timer = ScopeTimer::new(log::Level::Info, "application build step".to_owned());

        // SAFETY: This is basically foreign code execution, and there is not way to properly ensure safety
        // here. It is unfortunately an uncontrollable risk we must accept.
        let entry = unsafe { ash::Entry::load()? };

        let mut engine_version_numbers = option_env!("CARGO_PKG_VERSION")
            .unwrap_or("1.0.0")
            .split('.')
            .flat_map(|value| value.parse::<u32>())
            .chain(std::iter::repeat(0));
        let engine_version = vk::make_api_version(
            engine_version_numbers.next().unwrap(),
            engine_version_numbers.next().unwrap(),
            engine_version_numbers.next().unwrap(),
            engine_version_numbers.next().unwrap(),
        );
        let app_info = vk::ApplicationInfo::default()
            .application_name(&create_data.application_name)
            .application_version(create_data.application_version)
            .engine_name(c"miel")
            .engine_version(engine_version)
            .api_version(vk::make_api_version(1, 2, 197, 0));

        let instance_create_info = vk::InstanceCreateInfo::default().application_info(&app_info);

        // SAFETY: This is only safe is we keep the entry alive for longer than the instance, which
        // we do by storing it as well.
        let instance = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .map_err(ApplicationBuildError::InstanceCreationFail)?
        };

        Ok(Self {
            window_creation_data: create_data,
            window: None,
            state: start_state,

            entry,
            instance,
        })
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
