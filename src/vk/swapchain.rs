use ash::{khr, vk};
use thiserror::Error;

use crate::utils::ThreadSafeRef;

use super::{
    allocator::Allocator,
    device::Device,
    image::{Image, ImageBuildError, ImageCreateInfo},
    instance::Instance,
    surface::Surface,
};

#[allow(dead_code)]
pub(crate) struct SwapchainImage {
    pub handle: vk::Image,
    pub view: vk::ImageView,
}

#[allow(dead_code)]
pub(crate) struct Swapchain {
    pub handle: vk::SwapchainKHR,
    pub loader: khr::swapchain::Device,

    pub extent: vk::Extent2D,
    pub images: Vec<SwapchainImage>,
    pub depth_image: Image,

    pub present_semaphore: vk::Semaphore,
    pub render_semaphore: vk::Semaphore,
    pub render_fence: vk::Fence,

    // bookkeeping
    device_ref: ThreadSafeRef<Device>,
}

#[derive(Debug, Error)]
pub enum SwapchainCreateError {
    #[error("vulkan call to create the swapchain failed")]
    VulkanCreation(vk::Result),

    #[error("vulkan call to fetch swapchain images failed")]
    ImageFetching(vk::Result),

    #[error("vulkan call to create swapchain image views failed")]
    ImageViewCreation(vk::Result),

    #[error("vulkan call to create sync objects necessary for rendering failed")]
    RenderSyncObjectsCreation(vk::Result),

    #[error("depth image building failed")]
    DepthImageBuilding(ImageBuildError),
}

impl Swapchain {
    pub fn create(
        instance: &Instance,
        device_ref: ThreadSafeRef<Device>,
        surface: &Surface,
        suggested_size: vk::Extent2D,
        allocator_ref: ThreadSafeRef<Allocator>,
    ) -> Result<Self, SwapchainCreateError> {
        let device = device_ref.lock();
        let loader = khr::swapchain::Device::new(instance, &device);

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

        let images_handles = unsafe { loader.get_swapchain_images(handle) }
            .map_err(SwapchainCreateError::ImageFetching)?;
        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(surface.format.format)
            .components(
                vk::ComponentMapping::default()
                    .r(vk::ComponentSwizzle::R)
                    .g(vk::ComponentSwizzle::G)
                    .b(vk::ComponentSwizzle::B)
                    .a(vk::ComponentSwizzle::A),
            )
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );
        let images = images_handles
            .into_iter()
            .map(|handle| {
                let image_view_create_info = image_view_create_info.image(handle);
                let view = unsafe { device.create_image_view(&image_view_create_info, None) }
                    .map_err(SwapchainCreateError::ImageViewCreation)?;

                Ok(SwapchainImage { handle, view })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let present_semaphore = unsafe { device.create_semaphore(&semaphore_info, None) }
            .map_err(SwapchainCreateError::RenderSyncObjectsCreation)?;
        let render_semaphore = unsafe { device.create_semaphore(&semaphore_info, None) }
            .map_err(SwapchainCreateError::RenderSyncObjectsCreation)?;

        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let render_fence = unsafe { device.create_fence(&fence_info, None) }
            .map_err(SwapchainCreateError::RenderSyncObjectsCreation)?;

        drop(device);

        let depth_extent = vk::Extent3D {
            width: extent.width,
            height: extent.height,
            depth: 1,
        };
        let depth_image = ImageCreateInfo::swapchain_depth_image(depth_extent)
            .build_from_base_structs(device_ref.clone(), allocator_ref)
            .map_err(SwapchainCreateError::DepthImageBuilding)?;

        Ok(Self {
            handle,
            loader,
            extent,
            images,
            depth_image,
            present_semaphore,
            render_semaphore,
            render_fence,
            device_ref,
        })
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        let device = self.device_ref.lock();
        log::debug!("Waiting for device to be idle before destroying swapchain");
        unsafe { device.device_wait_idle() }.expect("device should wait before shutting down");

        log::debug!("destroying swapchain");
        unsafe { device.destroy_fence(self.render_fence, None) };
        unsafe { device.destroy_semaphore(self.render_semaphore, None) };
        unsafe { device.destroy_semaphore(self.present_semaphore, None) };
        for image in &self.images {
            unsafe { device.destroy_image_view(image.view, None) };
        }
        unsafe { self.loader.destroy_swapchain(self.handle, None) };
    }
}
