use ash::{
    khr,
    vk::{self, ImageAspectFlags},
};
use thiserror::Error;

use crate::{
    gfx::image::ImageState,
    utils::{ThreadSafeRef, ThreadSafeRwRef},
};

use super::{
    allocator::Allocator,
    device::Device,
    image::{Image, ImageBuildError, ImageCreateInfo},
    instance::Instance,
    surface::Surface,
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum NextImageState {
    Ok,
    Suboptimal,
    OutOfDate,
}

pub struct ImageResources<'a> {
    pub color_image: &'a mut ImageState,
    pub depth_image: &'a mut Image,
}

pub(crate) struct ImageContext {
    pub color_attachment: ImageState,
    pub depth_attachment: Image,

    pub render_semaphore: vk::Semaphore,
}

pub(crate) struct Swapchain {
    pub handle: vk::SwapchainKHR,
    pub loader: khr::swapchain::Device,

    pub extent: vk::Extent2D,
    pub images: Vec<ImageContext>,

    pub image_acquired_semaphore: vk::Semaphore,
    pub present_fence: vk::Fence,

    pub current_image_index: usize,

    // bookkeeping
    device_ref: ThreadSafeRwRef<Device>,
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

#[derive(Debug, Error)]
pub enum NextImageAcquireError {
    #[error("vulkan call to acquire next image index failed")]
    NextIndexAcquisition(vk::Result),

    #[error("acquired index is out of range ({0}, max is {1})")]
    InvalidIndex(u32, usize),
}

#[derive(Debug, Error)]
pub enum PresentError {
    #[error("vulkan call to present swapchain image failed")]
    Present(vk::Result),
}

impl Swapchain {
    pub fn new(
        instance: &Instance,
        device_ref: ThreadSafeRwRef<Device>,
        surface: &Surface,
        suggested_size: vk::Extent2D,
        allocator_ref: ThreadSafeRef<Allocator>,
    ) -> Result<Self, SwapchainCreateError> {
        let device = device_ref.read();
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

        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let present_semaphore = unsafe { device.create_semaphore(&semaphore_info, None) }
            .map_err(SwapchainCreateError::RenderSyncObjectsCreation)?;

        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let present_fence = unsafe { device.create_fence(&fence_info, None) }
            .map_err(SwapchainCreateError::RenderSyncObjectsCreation)?;

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

        let image_extent = extent.into();
        let depth_image_info = ImageCreateInfo::swapchain_depth_image(image_extent);

        let images = images_handles
            .into_iter()
            .map(|handle| {
                let render_semaphore = unsafe { device.create_semaphore(&semaphore_info, None) }
                    .map_err(SwapchainCreateError::RenderSyncObjectsCreation)?;

                let image_view_create_info = image_view_create_info.image(handle);
                let view = unsafe { device.create_image_view(&image_view_create_info, None) }
                    .map_err(SwapchainCreateError::ImageViewCreation)?;

                let color_attachment = ImageState {
                    handle,
                    view,
                    layout: vk::ImageLayout::UNDEFINED,
                    format: surface.format.format,
                    extent: image_extent,
                    extent_2d: extent,
                };

                let depth_attachment = depth_image_info
                    .clone()
                    .build_from_base_structs(device_ref.clone(), allocator_ref.clone())
                    .map_err(SwapchainCreateError::DepthImageBuilding)?;

                Ok(ImageContext {
                    color_attachment,
                    depth_attachment,
                    render_semaphore,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            handle,
            loader,
            extent,
            images,
            image_acquired_semaphore: present_semaphore,
            present_fence,
            current_image_index: usize::MAX,
            device_ref: device_ref.clone(),
        })
    }

    pub fn next_image(&mut self) -> Result<NextImageState, NextImageAcquireError> {
        match unsafe {
            self.loader.acquire_next_image(
                self.handle,
                u64::MAX,
                self.image_acquired_semaphore,
                vk::Fence::null(),
            )
        } {
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(NextImageState::OutOfDate),
            Ok((index, is_suboptimal)) => {
                self.current_image_index = index as usize;

                match is_suboptimal {
                    false => Ok(NextImageState::Ok),
                    true => Ok(NextImageState::Suboptimal),
                }
            }
            Err(err) => Err(NextImageAcquireError::NextIndexAcquisition(err)),
        }
    }

    pub fn current_image_resources(&mut self) -> ImageResources {
        let image = self.images.get_mut(self.current_image_index).unwrap();
        ImageResources {
            color_image: &mut image.color_attachment,
            depth_image: &mut image.depth_attachment,
        }
    }

    pub fn ensure_presentable(&mut self, &cmd_buffer: &vk::CommandBuffer) {
        let current_image_res = self.current_image_resources();

        let mut image_barriers = vec![];
        if current_image_res.color_image.layout != vk::ImageLayout::PRESENT_SRC_KHR {
            image_barriers.push(
                vk::ImageMemoryBarrier::default()
                    .image(current_image_res.color_image.handle)
                    .old_layout(current_image_res.color_image.layout)
                    .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .dst_access_mask(vk::AccessFlags::empty())
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .base_array_layer(0)
                            .level_count(1)
                            .base_mip_level(0),
                    ),
            );

            current_image_res.color_image.layout = vk::ImageLayout::PRESENT_SRC_KHR;
        }

        let device = self.device_ref.read();
        unsafe {
            device.cmd_pipeline_barrier(
                cmd_buffer,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &image_barriers,
            )
        };
    }

    pub fn present(&self) -> Result<(), PresentError> {
        let device = self.device_ref.read();

        unsafe {
            self.loader.queue_present(
                device.graphics_queue.handle,
                &vk::PresentInfoKHR::default()
                    .wait_semaphores(&[self.images[self.current_image_index].render_semaphore])
                    .swapchains(&[self.handle])
                    .image_indices(&[self.current_image_index as u32]),
            )
        }
        .map_err(PresentError::Present)?;

        Ok(())
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        let device = self.device_ref.read();
        log::debug!("Waiting for device to be idle before destroying swapchain");
        unsafe { device.device_wait_idle() }.expect("device should wait before shutting down");

        log::debug!("destroying swapchain");
        unsafe { device.destroy_fence(self.present_fence, None) };
        unsafe { device.destroy_semaphore(self.image_acquired_semaphore, None) };
        for image in &self.images {
            unsafe { device.destroy_semaphore(image.render_semaphore, None) };
            unsafe { device.destroy_image_view(image.color_attachment.view, None) };
        }
        unsafe { self.loader.destroy_swapchain(self.handle, None) };
    }
}
