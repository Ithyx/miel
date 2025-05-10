use thiserror::Error;

use super::{
    device::{Device, PhysicalDevice},
    instance::Instance,
};

pub(crate) struct Allocator {
    inner: gpu_allocator::vulkan::Allocator,
}

#[derive(Debug, Error)]
pub enum AllocatorCreateError {
    #[error("base memory allocation for allocator failed")]
    BasePoolsAllocations(#[from] gpu_allocator::AllocationError),
}

impl Allocator {
    pub fn create(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        device: &Device,
    ) -> Result<Self, AllocatorCreateError> {
        let create_info = gpu_allocator::vulkan::AllocatorCreateDesc {
            instance: instance.loader.clone(),
            device: device.loader.clone(),
            physical_device: physical_device.handle,
            debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
            buffer_device_address: false,
            allocation_sizes: gpu_allocator::AllocationSizes::default(),
        };
        let inner = gpu_allocator::vulkan::Allocator::new(&create_info)?;

        Ok(Self { inner })
    }
}
