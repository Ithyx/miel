use std::num::TryFromIntError;

use ash::vk;
use gpu_allocator::AllocationError;
use thiserror::Error;

use crate::{
    gfx::{
        buffer::{BufferBuildError, BufferBuilder, BufferDataUploadError},
        commands::{CommandManager, ImmediateCommandError},
        debug::debug_name_vk_object,
    },
    utils::{ThreadSafeRef, ThreadSafeRwRef},
};

use super::{
    allocator::{Allocation, Allocator},
    context::Context,
    device::Device,
    render_graph::resource::ImageAttachmentInfo,
};

#[derive(Debug, Clone)]
pub struct ImageState {
    pub handle: vk::Image,
    pub view: vk::ImageView,

    pub layout: vk::ImageLayout,
    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub extent_2d: vk::Extent2D,
    pub view_subresource_range: vk::ImageSubresourceRange,
}

impl ImageState {
    pub fn cmd_layout_transition(
        &mut self,
        device_ref: ThreadSafeRwRef<Device>,
        cmd_buffer: vk::CommandBuffer,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        image_memory_barrier: vk::ImageMemoryBarrier,
    ) {
        let image_memory_barrier = image_memory_barrier
            .image(self.handle)
            .old_layout(self.layout);
        self.layout = image_memory_barrier.new_layout;

        let device = device_ref.read();
        unsafe {
            device.cmd_pipeline_barrier(
                cmd_buffer,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_memory_barrier],
            )
        };
    }
}

#[derive(Clone)]
pub struct ImageBuilder<'a> {
    pub name: &'a str,

    pub layout: vk::ImageLayout,
    pub usage: vk::ImageUsageFlags,
    pub data: Option<Vec<u8>>,

    image_info: vk::ImageCreateInfo<'a>,
    image_view_info: vk::ImageViewCreateInfo<'a>,
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

    #[error("uploading of the initial image data failed")]
    ImageDataUploading(#[from] ImageDataUploadError),
}

impl<'a> ImageBuilder<'a> {
    pub fn new(name: &'a str, extent: vk::Extent3D) -> Self {
        let image_info = vk::ImageCreateInfo::default().extent(extent);
        let image_view_info = vk::ImageViewCreateInfo::default();

        Self {
            name,

            layout: vk::ImageLayout::UNDEFINED,
            usage: vk::ImageUsageFlags::empty(),
            data: None,

            image_info,
            image_view_info,
        }
    }

    pub fn from_attachment_info(info: &'a ImageAttachmentInfo) -> Self {
        let extent = match info.size {
            super::render_graph::resource::AttachmentSize::SwapchainBased => {
                vk::Extent3D::default()
            }
            super::render_graph::resource::AttachmentSize::Custom(extent3_d) => extent3_d,
        };
        let usage = info.usage | vk::ImageUsageFlags::TRANSFER_DST;

        let image_info = vk::ImageCreateInfo::default()
            .extent(extent)
            .image_type(vk::ImageType::TYPE_2D)
            .format(info.format)
            .mip_levels(1)
            .array_layers(info.layer_count)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let image_view_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(info.format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: info.layer_count,
            });

        Self {
            name: &info.name,
            image_info,
            image_view_info,

            layout: vk::ImageLayout::GENERAL,
            usage,
            data: None,
        }
    }

    pub(crate) fn swapchain_depth_image_default(mut self) -> Self {
        self.layout = vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL;
        self.usage |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;

        self.image_info = self
            .image_info
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        self.image_view_info = self
            .image_view_info
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        self
    }

    pub fn texture_default(mut self, format: vk::Format) -> Self {
        self.layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        self.usage |= vk::ImageUsageFlags::SAMPLED;

        self.image_info = self
            .image_info
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        self.image_view_info = self
            .image_view_info
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        self
    }

    pub fn with_layout(mut self, layout: vk::ImageLayout) -> Self {
        self.layout = layout;

        self
    }

    pub fn with_usage(mut self, usage: vk::ImageUsageFlags) -> Self {
        self.usage = usage;

        self
    }

    pub fn build(mut self, ctx: &Context) -> Result<Image, ImageBuildError> {
        if self.image_info.extent == vk::Extent3D::default() {
            self.image_info.extent = ctx.swapchain.extent.into();
        }
        let final_layout = self.layout;
        let extent = self.image_info.extent;
        let data = self.data.take().unwrap_or_else(|| {
            std::iter::repeat_n(
                u8::MAX,
                (extent.width * extent.height * 4).try_into().unwrap(),
            )
            .collect()
        });

        let name = self.name;
        let mut image =
            self.build_from_base_structs(ctx.device_ref.clone(), ctx.allocator_ref.clone())?;
        image.upload_data_internal(
            &data,
            Some(final_layout),
            ctx.allocator_ref.clone(),
            &ctx.command_manager,
        )?;

        if cfg!(debug_assertions) {
            // Not the end of the world if naming fails for whatever reason
            let _ = debug_name_vk_object(ctx, image.state.handle, name);
            let _ = debug_name_vk_object(ctx, image.state.view, name);
        }

        Ok(image)
    }

    /// Called under the hood by [`Self::build`], which is the intended method to be called by user
    /// code. This method ignores the data field, and leaves the image in an uninitialized state.
    pub(crate) fn build_from_base_structs(
        mut self,
        device_ref: ThreadSafeRwRef<Device>,
        allocator_ref: ThreadSafeRef<Allocator>,
    ) -> Result<Image, ImageBuildError> {
        let device = device_ref.read();
        let mut allocator = allocator_ref.lock();

        self.usage |= vk::ImageUsageFlags::TRANSFER_DST;
        self.image_info.usage |= self.usage;

        let handle = unsafe { device.create_image(&self.image_info, None) }
            .map_err(ImageBuildError::VulkanCreation)?;

        let memory_requirements = unsafe { device.get_image_memory_requirements(handle) };
        let allocation_info = gpu_allocator::vulkan::AllocationCreateDesc {
            name: self.name,
            requirements: memory_requirements,
            location: gpu_allocator::MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::DedicatedImage(handle),
        };
        let _allocation = allocator.allocate(&allocation_info, allocator_ref.clone())?;
        drop(allocator);

        unsafe { device.bind_image_memory(handle, _allocation.memory(), _allocation.offset()) }
            .map_err(ImageBuildError::MemoryBind)?;

        self.image_view_info.image = handle;
        let view = unsafe { device.create_image_view(&self.image_view_info, None) }
            .map_err(ImageBuildError::ImageViewCreation)?;

        let state = ImageState {
            handle,
            view,

            layout: self.image_info.initial_layout,
            format: self.image_info.format,
            extent: self.image_info.extent,
            extent_2d: vk::Extent2D {
                width: self.image_info.extent.width,
                height: self.image_info.extent.height,
            },
            view_subresource_range: self.image_view_info.subresource_range,
        };

        Ok(Image {
            name: self.name.to_owned(),
            state,
            _allocation,

            device_ref: device_ref.clone(),
        })
    }
}

pub struct Image {
    pub name: String,
    pub state: ImageState,
    pub(crate) _allocation: Allocation,

    // bookkeeping
    device_ref: ThreadSafeRwRef<Device>,
}

impl Drop for Image {
    fn drop(&mut self) {
        let device = self.device_ref.read();

        unsafe { device.destroy_image_view(self.state.view, None) };
        unsafe { device.destroy_image(self.state.handle, None) };
    }
}

#[derive(Debug, Error)]
pub enum ImageDataUploadError {
    #[error("size conversion from usize to u64 failed")]
    InvalidSize(#[from] TryFromIntError),
    #[error("staging buffer creation failed")]
    StagingBufferBuilding(#[from] BufferBuildError),
    #[error("staging buffer data upload failed")]
    StagingBufferUploading(#[from] BufferDataUploadError),
    #[error("immediate command execution failed")]
    ImmediateCommand(#[from] ImmediateCommandError),
}

impl<'a> Image {
    pub fn builder(name: &'a str, extent: vk::Extent3D) -> ImageBuilder<'a> {
        ImageBuilder::new(name, extent)
    }

    pub fn cmd_layout_transition(
        &mut self,
        cmd_buffer: vk::CommandBuffer,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        image_memory_barrier: vk::ImageMemoryBarrier,
    ) {
        self.state.cmd_layout_transition(
            self.device_ref.clone(),
            cmd_buffer,
            src_stage_mask,
            dst_stage_mask,
            image_memory_barrier,
        );
    }

    pub fn upload_data(
        &mut self,
        data: &[u8],
        new_layout: Option<vk::ImageLayout>,
        ctx: &mut Context,
    ) -> Result<(), ImageDataUploadError> {
        self.upload_data_internal(
            data,
            new_layout,
            ctx.allocator_ref.clone(),
            &ctx.command_manager,
        )
    }

    pub(crate) fn upload_data_internal(
        &mut self,
        data: &[u8],
        new_layout: Option<vk::ImageLayout>,
        allocator_ref: ThreadSafeRef<Allocator>,
        cmd_manager: &CommandManager,
    ) -> Result<(), ImageDataUploadError> {
        let new_layout = new_layout.unwrap_or(self.state.layout);

        let mut staging_buffer = BufferBuilder::staging_buffer_default(data.len().try_into()?)
            .build_internal(self.device_ref.clone(), allocator_ref)?;
        staging_buffer.upload_data(data)?;

        cmd_manager.immediate_command(|&cmd_buffer, device| {
            let range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            if self.state.layout != vk::ImageLayout::TRANSFER_DST_OPTIMAL {
                let transfer_dst_barrier = vk::ImageMemoryBarrier::default()
                    .src_access_mask(vk::AccessFlags::NONE)
                    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .old_layout(self.state.layout)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .image(self.state.handle)
                    .subresource_range(range);
                unsafe {
                    device.cmd_pipeline_barrier(
                        cmd_buffer,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        std::slice::from_ref(&transfer_dst_barrier),
                    )
                };
            }

            let copy_region = vk::BufferImageCopy::default()
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_extent(self.state.extent);
            unsafe {
                device.cmd_copy_buffer_to_image(
                    cmd_buffer,
                    staging_buffer.handle,
                    self.state.handle,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    std::slice::from_ref(&copy_region),
                )
            };

            let shader_read_barrier = vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::NONE)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(new_layout)
                .image(self.state.handle)
                .subresource_range(range);
            unsafe {
                device.cmd_pipeline_barrier(
                    cmd_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    std::slice::from_ref(&shader_read_barrier),
                )
            };
        })?;

        Ok(())
    }
}
