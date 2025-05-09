use std::{ffi::CString, ops::Deref};

use ash::{ext, vk};
use thiserror::Error;
use winit::raw_window_handle::RawDisplayHandle;

pub(crate) struct Instance {
    pub handle: ash::Instance,
}

impl Deref for Instance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

#[derive(Debug, Error)]
pub enum InstanceCreateError {
    #[error("query for necessary extensions from ash_window failed")]
    ExtensionQueryError(vk::Result),
    #[error("vulkan call to create instance failed")]
    VulkanCreationError(vk::Result),
}

impl Instance {
    pub fn create(
        entry: &ash::Entry,
        application_name: &CString,
        application_version: u32,
        vk_version: u32,
        display_handle: RawDisplayHandle,
    ) -> Result<Self, InstanceCreateError> {
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
            .application_name(application_name)
            .application_version(application_version)
            .engine_name(c"miel")
            .engine_version(engine_version)
            .api_version(vk_version);
        let mut enabled_extensions = ash_window::enumerate_required_extensions(display_handle)
            .map_err(InstanceCreateError::ExtensionQueryError)?
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
        let handle = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .map_err(InstanceCreateError::VulkanCreationError)?
        };

        Ok(Self { handle })
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: This is safe as long as the entry used to create the loader is still alive.
        unsafe { self.handle.destroy_instance(None) };
    }
}
