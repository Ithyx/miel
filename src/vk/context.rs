use std::ffi::CString;

use thiserror::Error;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use super::{
    debug::{DUMCreationError, DUMessenger},
    instance::{Instance, InstanceCreateError},
    surface::{Surface, SurfaceCreateError},
};

pub struct ContextCreateInfo {
    pub application_name: CString,
    pub application_version: u32,
}

pub(crate) struct Context {
    du_messenger: Option<DUMessenger>,
    surface: Surface,
    instance: Instance,
    entry: ash::Entry,
}

#[derive(Debug, Error)]
pub enum ContextCreateError {
    #[error("vulkan library loading failed")]
    VulkanLoadFail(#[from] ash::LoadingError),

    #[error("instance creation failed")]
    InstanceCreationFail(#[from] InstanceCreateError),

    #[error("debug utils messenger creation failed")]
    DUMCreationFail(#[from] DUMCreationError),

    #[error("surface creation failed")]
    SurfaceCreationFail(#[from] SurfaceCreateError),
}

impl Context {
    pub fn create(
        window: &Window,
        create_info: &ContextCreateInfo,
    ) -> Result<Self, ContextCreateError> {
        let window_handle = window
            .window_handle()
            .expect("window should have a valid window handle")
            .as_raw();
        let display_handle = window
            .display_handle()
            .expect("window should have a valid diaplay handle")
            .as_raw();

        // SAFETY: This is basically foreign code execution, and there is not way to properly ensure safety
        // here. It is unfortunately an uncontrollable risk we must accept.
        let entry = unsafe { ash::Entry::load()? };
        let instance = Instance::create(
            &entry,
            &create_info.application_name,
            create_info.application_version,
            display_handle,
        )?;
        let du_messenger = DUMessenger::create(&entry, &instance.handle)?;
        let surface = Surface::create(&entry, &instance.handle, display_handle, window_handle)?;

        Ok(Self {
            du_messenger,
            surface,
            instance,
            entry,
        })
    }
}
