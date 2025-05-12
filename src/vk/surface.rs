use ash::{khr, vk};
use thiserror::Error;
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use super::{device::PhysicalDevice, instance::Instance};

pub(crate) struct Surface {
    pub handle: vk::SurfaceKHR,
    pub loader: khr::surface::Instance,

    pub format: vk::SurfaceFormatKHR,
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub present_mode: vk::PresentModeKHR,
}

#[derive(Debug, Error)]
pub enum SurfaceCreateError {
    #[error("vulkan call to create the surface failed")]
    VulkanCreation(vk::Result),
}

#[derive(Debug, Error)]
pub enum DeviceSetupError {
    #[error("vulkan call to fetch capabilities from surface failed")]
    CapabilitiesFetching(vk::Result),

    #[error("vulkan call to enumerate present modes from surface failed")]
    PresentMoodeEnumeration(vk::Result),

    #[error("vulkan call to enumerate formats from surface failed")]
    FormatEnumeration(vk::Result),

    #[error("no valid format found")]
    NoFormat,
}

impl Surface {
    pub fn create(
        entry: &ash::Entry,
        instance: &Instance,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Self, SurfaceCreateError> {
        // SAFETY: This surface must have a strictly smaller lifetime than the instance and entry
        // used to create it. We ensure this is the case by storing them accordingly and dropping
        // them in the correct order.
        let handle = unsafe {
            ash_window::create_surface(entry, instance, display_handle, window_handle, None)
                .map_err(SurfaceCreateError::VulkanCreation)?
        };
        let loader = khr::surface::Instance::new(entry, instance);

        Ok(Self {
            handle,
            loader,
            format: vk::SurfaceFormatKHR::default(),
            capabilities: vk::SurfaceCapabilitiesKHR::default(),
            present_mode: vk::PresentModeKHR::FIFO,
        })
    }

    pub fn setup_from_device(
        &mut self,
        physical_device: &PhysicalDevice,
    ) -> Result<(), DeviceSetupError> {
        let capabilities = unsafe {
            self.loader
                .get_physical_device_surface_capabilities(physical_device.handle, self.handle)
        }
        .map_err(DeviceSetupError::CapabilitiesFetching)?;
        self.capabilities = capabilities;

        let present_modes = unsafe {
            self.loader
                .get_physical_device_surface_present_modes(physical_device.handle, self.handle)
        }
        .map_err(DeviceSetupError::PresentMoodeEnumeration)?;
        if let Some(&present_mode) = present_modes
            .iter()
            .find(|&&mode| mode == vk::PresentModeKHR::MAILBOX)
        {
            self.present_mode = present_mode;
        }

        let available_formats = unsafe {
            self.loader
                .get_physical_device_surface_formats(physical_device.handle, self.handle)
        }
        .map_err(DeviceSetupError::FormatEnumeration)?;

        let format_fallback = *available_formats
            .first()
            .ok_or(DeviceSetupError::NoFormat)?;

        let selected_format = available_formats
            .into_iter()
            .find(|&format| {
                format.format == vk::Format::B8G8R8A8_SRGB
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or(format_fallback);

        log::debug!(
            "Selected surface format {:?} with colorspace {:?}",
            selected_format.format,
            selected_format.color_space
        );
        self.format = selected_format;

        Ok(())
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        log::debug!("destroying surface");
        // SAFETY: This is safe as long as the entry used to create the loader is still alive.
        unsafe {
            self.loader.destroy_surface(self.handle, None);
        };
    }
}
