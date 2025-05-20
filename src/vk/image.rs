use ash::vk;
use gpu_allocator::AllocationError;
use thiserror::Error;

use crate::utils::ThreadSafeRef;

use super::{
    allocator::{Allocation, Allocator},
    context::Context,
    device::Device,
};

#[derive(Default)]
pub struct ImageCreateInfo<'a> {
    pub image_info: vk::ImageCreateInfo<'a>,
    pub image_view_info: vk::ImageViewCreateInfo<'a>,
}

#[derive(Debug, Error)]
pub enum ImageBuildError {
    #[error("vulkan creation of the image failed")]
    VulkanImageCreation(vk::Result),

    #[error("memory allocation failed")]
    Allocation(#[from] AllocationError),

    #[error("binding allocated memory to image failed")]
    MemoryBind(vk::Result),

    #[error("vulkan creation of the image view failed")]
    VulkanImageViewCreation(vk::Result),
}

impl ImageCreateInfo<'_> {
    pub(crate) fn swapchain_depth_image(depth_extent: vk::Extent3D) -> Self {
        let image_info = vk::ImageCreateInfo::default()
            .extent(depth_extent)
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let image_view_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        Self {
            image_info,
            image_view_info,
        }
    }

    pub fn build(self, context: &Context) -> Result<Image, ImageBuildError> {
        self.build_from_base_structs(context.device_ref.clone(), context.allocator_ref.clone())
    }

    /// Called under the hood by [`Self::build`], which is the intended method to be called by user
    /// code.
    pub(crate) fn build_from_base_structs(
        mut self,
        device_ref: ThreadSafeRef<Device>,
        allocator_ref: ThreadSafeRef<Allocator>,
    ) -> Result<Image, ImageBuildError> {
        let device = device_ref.lock();
        let mut allocator = allocator_ref.lock();

        let handle = unsafe { device.create_image(&self.image_info, None) }
            .map_err(ImageBuildError::VulkanImageCreation)?;

        let memory_requirements = unsafe { device.get_image_memory_requirements(handle) };
        let allocation_info = gpu_allocator::vulkan::AllocationCreateDesc {
            name: "depth image allocation",
            requirements: memory_requirements,
            location: gpu_allocator::MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::DedicatedImage(handle),
        };
        let allocation = allocator.allocate(&allocation_info, allocator_ref.clone())?;

        unsafe { device.bind_image_memory(handle, allocation.memory(), allocation.offset()) }
            .map_err(ImageBuildError::MemoryBind)?;

        self.image_view_info.image = handle;
        let view = unsafe { device.create_image_view(&self.image_view_info, None) }
            .map_err(ImageBuildError::VulkanImageViewCreation)?;

        drop(allocator);
        drop(device);

        Ok(Image {
            handle,
            allocation,
            view,

            layout: self.image_info.initial_layout,
            format: self.image_info.format,
            extent: self.image_info.extent,

            layer_count: self.image_info.array_layers,

            device_ref,
        })
    }
}

pub struct Image {
    pub(crate) handle: vk::Image,
    pub(crate) allocation: Allocation,
    pub(crate) view: vk::ImageView,

    pub(crate) layout: vk::ImageLayout,
    pub(crate) format: vk::Format,
    pub(crate) extent: vk::Extent3D,

    // useful for cubemaps
    layer_count: u32,

    // bookkeeping
    device_ref: ThreadSafeRef<Device>,
}

impl Drop for Image {
    fn drop(&mut self) {
        let device = self.device_ref.lock();

        unsafe { device.destroy_image_view(self.view, None) };
        unsafe { device.destroy_image(self.handle, None) };
    }
}
