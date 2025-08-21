use std::fmt::Debug;

use ash::vk;
use thiserror::Error;

use crate::{
    gfx::{
        allocator::{Allocation, Allocator},
        context::Context,
        device::Device,
    },
    utils::{ThreadSafeRef, ThreadSafeRwRef},
};

pub struct Buffer {
    pub handle: vk::Buffer,
    size: u64,

    pub(crate) allocation: Allocation,

    // bookkeeping
    device_ref: ThreadSafeRwRef<Device>,
}

#[derive(Error, Debug)]
pub enum BufferDataUploadError {
    #[error("conversion of data size from usize to u64 failed (check that {0} <= u64::MAX)")]
    SizeConversion(usize),

    #[error(
        "data size ({data_size}) does not match the buffer's allocation size ({buffer_size}), check that T is #[repr(C)]"
    )]
    SizeMismatch { data_size: usize, buffer_size: u64 },

    #[error("buffer memory mapping failed")]
    MemoryMapping,
}

impl Buffer {
    /// This defaults to a uniform buffer usage
    pub fn builder(size: u64) -> BufferBuilder {
        BufferBuilder::default(size)
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn upload_pod<T: bytemuck::Pod>(&mut self, pod: T) -> Result<(), BufferDataUploadError> {
        if self.allocation.size()
            < std::mem::size_of::<T>()
                .try_into()
                .map_err(|_| BufferDataUploadError::SizeConversion(std::mem::size_of::<T>()))?
        {
            return Err(BufferDataUploadError::SizeMismatch {
                data_size: std::mem::size_of::<T>(),
                buffer_size: self.allocation.size(),
            });
        }

        let raw_data = bytemuck::bytes_of(&pod);
        self.upload_data(raw_data)
    }

    pub fn upload_data(&mut self, data: &[u8]) -> Result<(), BufferDataUploadError> {
        self.allocation
            .mapped_slice_mut()
            .ok_or(BufferDataUploadError::MemoryMapping)?[..data.len()]
            .copy_from_slice(data);

        Ok(())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { self.device_ref.read().destroy_buffer(self.handle, None) };
    }
}

impl Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Buffer")
            .field("handle", &self.handle)
            .field("size", &self.size)
            .field("allocation", &self.allocation)
            .finish()
    }
}

#[derive(Error, Debug)]
pub enum BufferBuildError {
    #[error("vulkan creation failed")]
    VulkanCreation(vk::Result),

    #[error("memory allocation failed")]
    Allocation(#[from] gpu_allocator::AllocationError),

    #[error("vulkan allocation binding failed")]
    AllocationBinding(vk::Result),
}

#[derive(Error, Debug)]
pub enum BufferBuildWithDataError {
    #[error("build failed")]
    BuildFailed(#[from] BufferBuildError),

    #[error("data uploading failed")]
    DataUploadFailed(#[from] BufferDataUploadError),
}

pub struct BufferBuilder {
    pub size: u64,
    pub usage: vk::BufferUsageFlags,
    pub memory_location: gpu_allocator::MemoryLocation,

    pub name: String,
}

/// @TODO(Ithyx): create new type with MemoryLocation::GpuOnly
impl BufferBuilder {
    /// This is equivalent to `uniform_buffer_default`
    pub fn default(size: u64) -> Self {
        Self::uniform_buffer_default(size)
    }

    pub fn uniform_buffer_default(size: u64) -> Self {
        Self {
            size,
            usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
            memory_location: gpu_allocator::MemoryLocation::CpuToGpu,
            name: String::from("unnamed buffer"),
        }
    }

    pub fn staging_buffer_default(size: u64) -> Self {
        Self {
            size,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            memory_location: gpu_allocator::MemoryLocation::CpuToGpu,
            name: String::from("unnamed staging buffer"),
        }
    }

    pub fn with_usage(mut self, usage: vk::BufferUsageFlags) -> Self {
        self.usage = usage;
        self
    }

    pub fn with_memory_location(mut self, memory_location: gpu_allocator::MemoryLocation) -> Self {
        self.memory_location = memory_location;
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }

    pub fn build(self, ctx: &mut Context) -> Result<Buffer, BufferBuildError> {
        self.build_internal(ctx.device_ref.clone(), ctx.allocator_ref.clone())
    }

    pub fn build_with_pod<T: bytemuck::Pod>(
        self,
        pod: T,
        ctx: &mut Context,
    ) -> Result<Buffer, BufferBuildWithDataError> {
        let mut buffer = self.build(ctx)?;

        buffer.upload_pod(pod)?;

        Ok(buffer)
    }

    pub fn build_with_data(
        self,
        data: &[u8],
        ctx: &mut Context,
    ) -> Result<Buffer, BufferBuildWithDataError> {
        let mut buffer = self.build(ctx)?;

        buffer.upload_data(data)?;

        Ok(buffer)
    }

    pub(crate) fn build_internal(
        self,
        device_ref: ThreadSafeRwRef<Device>,
        allocator_ref: ThreadSafeRef<Allocator>,
    ) -> Result<Buffer, BufferBuildError> {
        let buffer_info = vk::BufferCreateInfo {
            size: self.size,
            usage: self.usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let device = device_ref.read();
        let handle = unsafe { device.create_buffer(&buffer_info, None) }
            .map_err(BufferBuildError::VulkanCreation)?;

        let memory_req = unsafe { device.get_buffer_memory_requirements(handle) };
        let allocation = allocator_ref.lock().allocate(
            &gpu_allocator::vulkan::AllocationCreateDesc {
                name: &self.name,
                requirements: memory_req,
                location: self.memory_location,
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::DedicatedBuffer(handle),
            },
            allocator_ref.clone(),
        )?;

        unsafe { device.bind_buffer_memory(handle, allocation.memory(), allocation.offset()) }
            .map_err(BufferBuildError::AllocationBinding)?;

        Ok(Buffer {
            handle,
            allocation,
            size: self.size,
            device_ref: device_ref.clone(),
        })
    }
}
