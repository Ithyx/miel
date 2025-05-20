use ash::vk::{self, CommandBufferLevel};
use thiserror::Error;

use crate::utils::ThreadSafeRef;

use super::device::Device;

pub(crate) struct CommandManager {
    pub(crate) cmd_pool: vk::CommandPool,

    pub(crate) rendering_cmd_buffer: vk::CommandBuffer,

    pub(crate) immediate_cmd_buffer: vk::CommandBuffer,
    pub(crate) immediate_fence: vk::Fence,

    //bookkeeping
    device_ref: ThreadSafeRef<Device>,
}

#[derive(Debug, Error)]
pub enum CommandManagerCreateError {
    #[error("vulkan call to create command pool failed")]
    VulkanCmdPoolCreation(vk::Result),

    #[error("vulkan call to allocated command buffer failed")]
    VulkanCmdBufferAllocation(vk::Result),

    #[error("vulkan call to create fence failed")]
    VulkanFenceCreation(vk::Result),
}

impl CommandManager {
    pub(crate) fn try_new(
        device_ref: ThreadSafeRef<Device>,
    ) -> Result<Self, CommandManagerCreateError> {
        let device = device_ref.lock();

        let cmd_pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(device.graphics_queue.family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let cmd_pool = unsafe { device.create_command_pool(&cmd_pool_info, None) }
            .map_err(CommandManagerCreateError::VulkanCmdPoolCreation)?;

        let cmd_buffer_info = vk::CommandBufferAllocateInfo::default()
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(2)
            .command_pool(cmd_pool);
        let cmd_buffers = unsafe { device.allocate_command_buffers(&cmd_buffer_info) }
            .map_err(CommandManagerCreateError::VulkanCmdBufferAllocation)?;

        let immediate_fence_info = vk::FenceCreateInfo::default();
        let immediate_fence = unsafe { device.create_fence(&immediate_fence_info, None) }
            .map_err(CommandManagerCreateError::VulkanFenceCreation)?;

        drop(device);

        Ok(Self {
            cmd_pool,
            rendering_cmd_buffer: cmd_buffers[0],
            immediate_cmd_buffer: cmd_buffers[1],
            immediate_fence,
            device_ref,
        })
    }
}

impl Drop for CommandManager {
    fn drop(&mut self) {
        log::debug!("destroying command manager");

        let device = self.device_ref.lock();
        unsafe { device.destroy_fence(self.immediate_fence, None) };
        unsafe { device.destroy_command_pool(self.cmd_pool, None) };
    }
}
