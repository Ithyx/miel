use std::ffi::CString;

use ash::{ext, vk};
use thiserror::Error;
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

use super::debug::{DUMCreationError, DUMHandle, create_dum};

pub struct ContextCreateInfo {
    pub application_name: CString,
    pub application_version: u32,
}

pub(crate) struct Context {
    _messenger_handle: DUMHandle,

    instance: ash::Instance,
    entry: ash::Entry,
}

#[derive(Debug, Error)]
pub enum ContextCreateError {
    #[error("vulkan library loading failed")]
    VulkanLoadFail(#[from] ash::LoadingError),

    #[error("instance creation failed")]
    InstanceCreationFail(vk::Result),

    #[error("debug utils messenger creation failed")]
    DUMCreationFail(#[from] DUMCreationError),
}

impl Context {
    pub fn create(
        window: &Window,
        create_info: &ContextCreateInfo,
    ) -> Result<Self, ContextCreateError> {
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
            .application_name(&create_info.application_name)
            .application_version(create_info.application_version)
            .engine_name(c"miel")
            .engine_version(engine_version)
            .api_version(vk::make_api_version(0, 1, 2, 197));

        let mut enabled_extensions = ash_window::enumerate_required_extensions(
            window
                .display_handle()
                .expect("window should have a valid diaplay handle")
                .as_raw(),
        )
        .expect("required extensions should be queried correctly from the display handle")
        .to_vec();
        let mut enabled_layers = vec![];
        if cfg!(debug_assertions) {
            enabled_extensions.push(ext::debug_utils::NAME.as_ptr());
            enabled_layers.push(c"VK_LAYER_KHRONOS_validation".as_ptr());
        }

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&enabled_extensions)
            .enabled_layer_names(&enabled_layers);

        // SAFETY: This is only safe is we keep the entry alive for longer than the instance, which
        // we do by storing it as well.
        let instance = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .map_err(ContextCreateError::InstanceCreationFail)?
        };

        let _messenger_handle = create_dum(&entry, &instance)?;

        Ok(Self {
            _messenger_handle,
            instance,
            entry,
        })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        log::info!("dropping vulkan context");

        // SAFETY: This is only valid if the associated entry is still alive, which it is, as we
        // haven't dropped any members yet.
        unsafe { self.instance.destroy_instance(None) };
    }
}
