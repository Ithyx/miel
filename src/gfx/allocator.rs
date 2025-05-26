use std::{fmt::Debug, ops::Deref};

use thiserror::Error;

use crate::utils::ThreadSafeRef;

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
        let is_debug = cfg!(debug_assertions);
        let debug_settings = gpu_allocator::AllocatorDebugSettings {
            log_memory_information: true,
            log_leaks_on_shutdown: true,
            store_stack_traces: false,
            log_allocations: is_debug,
            log_frees: is_debug,
            log_stack_traces: false,
        };
        let create_info = gpu_allocator::vulkan::AllocatorCreateDesc {
            instance: instance.loader.clone(),
            device: device.loader.clone(),
            physical_device: physical_device.handle,
            debug_settings,
            buffer_device_address: false,
            allocation_sizes: gpu_allocator::AllocationSizes::default(),
        };
        let inner = gpu_allocator::vulkan::Allocator::new(&create_info)?;

        Ok(Self { inner })
    }

    pub fn allocate(
        &mut self,
        desc: &gpu_allocator::vulkan::AllocationCreateDesc<'_>,
        allocator_ref: ThreadSafeRef<Self>,
    ) -> Result<Allocation, gpu_allocator::AllocationError> {
        self.inner.allocate(desc).map(|handle| Allocation {
            handle: Some(handle),
            allocator_ref,
        })
    }
}

// A useful wrapper type to hold an allocation and destroy it on drop
pub(crate) struct Allocation {
    handle: Option<gpu_allocator::vulkan::Allocation>,
    allocator_ref: ThreadSafeRef<Allocator>,
}

impl Deref for Allocation {
    type Target = gpu_allocator::vulkan::Allocation;

    fn deref(&self) -> &Self::Target {
        // There is no way to have a None in this option, unless after dropping which is
        // impossible, this unwrap is guarateed safe
        self.handle.as_ref().unwrap()
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        if let Some(allocation) = self.handle.take() {
            let _ = self.allocator_ref.lock().inner.free(allocation);
        }
    }
}
