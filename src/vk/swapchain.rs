use ash::{khr, vk};
use thiserror::Error;

use super::{device::Device, instance::Instance, surface::Surface};

pub(crate) struct Swapchain {
    pub handle: vk::SwapchainKHR,
    pub loader: khr::swapchain::Device,

    pub extent: vk::Extent2D,
}

#[derive(Debug, Error)]
pub enum SwapchainCreateError {
    #[error("vulkan call to create the swapchain failed")]
    VulkanCreation(vk::Result),
}

impl Swapchain {
    pub fn create(
        instance: &Instance,
        device: &Device,
        surface: &Surface,
        suggested_size: vk::Extent2D,
    ) -> Result<Self, SwapchainCreateError> {
        let loader = khr::swapchain::Device::new(instance, device);

        let mut min_image_count = surface.capabilities.min_image_count + 1;
        if surface.capabilities.max_image_count > 0
            && min_image_count > surface.capabilities.max_image_count
        {
            min_image_count = surface.capabilities.max_image_count;
        }

        let extent = match surface.capabilities.current_extent {
            vk::Extent2D {
                width: u32::MAX,
                height: u32::MAX,
            } => suggested_size,
            _ => surface.capabilities.current_extent,
        };

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.handle)
            .min_image_count(min_image_count)
            .image_format(surface.format.format)
            .image_color_space(surface.format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(surface.present_mode)
            .clipped(true);

        let handle = unsafe { loader.create_swapchain(&create_info, None) }
            .map_err(SwapchainCreateError::VulkanCreation)?;

        Ok(Self {
            handle,
            loader,
            extent,
        })
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        log::debug!("destroying swapchain");
        unsafe {
            self.loader.destroy_swapchain(self.handle, None);
        };
    }
}
