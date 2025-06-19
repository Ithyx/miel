use ash::vk;
use gpu_allocator::AllocationError;
use thiserror::Error;

use crate::utils::{ThreadSafeRef, ThreadSafeRwRef};

use super::{
    allocator::{Allocation, Allocator},
    context::Context,
    device::Device,
    render_graph::resource::ImageAttachmentDescription,
};

#[derive(Default, Clone)]
pub struct ImageCreateInfo<'a> {
    pub image_info: vk::ImageCreateInfo<'a>,
    pub image_view_info: vk::ImageViewCreateInfo<'a>,
    pub allocation_name: &'a str,
}

#[derive(Debug, Error)]
pub enum ImageBuildError {
    #[error("vulkan creation of the image failed")]
    VulkanCreation(vk::Result),

    #[error("memory allocation failed")]
    Allocation(#[from] AllocationError),

    #[error("binding allocated memory to image failed")]
    MemoryBind(vk::Result),

    #[error("vulkan creation of the image view failed")]
    ImageViewCreation(vk::Result),
}

impl<'a> ImageCreateInfo<'a> {
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
            allocation_name: "swapchain depth image",
        }
    }

    pub(crate) fn from_attachment_description(description: &'a ImageAttachmentDescription) -> Self {
        let extent = match description.size {
            super::render_graph::resource::AttachmentSize::Swapchain => vk::Extent3D::default(),
            super::render_graph::resource::AttachmentSize::Custom(extent3_d) => extent3_d,
        };

        let image_info = vk::ImageCreateInfo::default()
            .extent(extent)
            .image_type(vk::ImageType::TYPE_2D)
            .format(description.format)
            .mip_levels(1)
            .array_layers(description.layer_count)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(description.usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let image_view_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(description.format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: description.layer_count,
            });

        Self {
            image_info,
            image_view_info,
            allocation_name: &description.name,
        }
    }

    pub fn build(mut self, context: &Context) -> Result<Image, ImageBuildError> {
        if self.image_info.extent == vk::Extent3D::default() {
            self.image_info.extent = context.swapchain.extent.into();
        }

        self.build_from_base_structs(context.device_ref.clone(), context.allocator_ref.clone())
    }

    /// Called under the hood by [`Self::build`], which is the intended method to be called by user
    /// code.
    pub(crate) fn build_from_base_structs(
        mut self,
        device_ref: ThreadSafeRwRef<Device>,
        allocator_ref: ThreadSafeRef<Allocator>,
    ) -> Result<Image, ImageBuildError> {
        let device = device_ref.read();
        let mut allocator = allocator_ref.lock();

        let handle = unsafe { device.create_image(&self.image_info, None) }
            .map_err(ImageBuildError::VulkanCreation)?;

        let memory_requirements = unsafe { device.get_image_memory_requirements(handle) };
        let allocation_info = gpu_allocator::vulkan::AllocationCreateDesc {
            name: self.allocation_name,
            requirements: memory_requirements,
            location: gpu_allocator::MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::DedicatedImage(handle),
        };
        let _allocation = allocator.allocate(&allocation_info, allocator_ref.clone())?;

        unsafe { device.bind_image_memory(handle, _allocation.memory(), _allocation.offset()) }
            .map_err(ImageBuildError::MemoryBind)?;

        self.image_view_info.image = handle;
        let view = unsafe { device.create_image_view(&self.image_view_info, None) }
            .map_err(ImageBuildError::ImageViewCreation)?;

        Ok(Image {
            handle,
            _allocation,
            view,

            layout: self.image_info.initial_layout,
            format: self.image_info.format,
            extent: self.image_info.extent,

            _layer_count: self.image_info.array_layers,

            device_ref: device_ref.clone(),
        })
    }
}

pub struct Image {
    pub handle: vk::Image,
    pub(crate) _allocation: Allocation,
    pub view: vk::ImageView,

    pub layout: vk::ImageLayout,
    pub format: vk::Format,
    pub extent: vk::Extent3D,

    // useful for cubemaps
    _layer_count: u32,

    // bookkeeping
    device_ref: ThreadSafeRwRef<Device>,
}

impl Drop for Image {
    fn drop(&mut self) {
        let device = self.device_ref.read();

        unsafe { device.destroy_image_view(self.view, None) };
        unsafe { device.destroy_image(self.handle, None) };
    }
}

impl<'a> Image {
    pub fn create_info() -> ImageCreateInfo<'a> {
        ImageCreateInfo::default()
    }
}
