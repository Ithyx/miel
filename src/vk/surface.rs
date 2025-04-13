use ash::{khr, vk};
use thiserror::Error;
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

pub(crate) struct Surface {
    pub handle: vk::SurfaceKHR,
    pub loader: khr::surface::Instance,
    // format: vk::Format,
}

#[derive(Debug, Error)]
pub enum SurfaceCreateError {
    #[error("vulkan call to create the surface failed")]
    VulkanError(vk::Result),
}

impl Surface {
    pub(crate) fn create(
        entry: &ash::Entry,
        instance: &ash::Instance,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Self, SurfaceCreateError> {
        // SAFETY: This surface must have a strictly smaller lifetime than the instance and entry
        // used to create it. We ensure this is the case by storing them accordingly and dropping
        // them in the correct order.
        let handle = unsafe {
            ash_window::create_surface(entry, instance, display_handle, window_handle, None)
                .map_err(SurfaceCreateError::VulkanError)?
        };
        let loader = khr::surface::Instance::new(entry, instance);

        Ok(Self { handle, loader })
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        // SAFETY: This is safe as long as the entry used to create the loader is still alive.
        unsafe {
            self.loader.destroy_surface(self.handle, None);
        };
    }
}
