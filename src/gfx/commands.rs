use ash::vk::{self, CommandBufferLevel};
use thiserror::Error;

use crate::utils::ThreadSafeRef;

use super::{
    device::Device,
    render_graph::RenderGraphRunError,
    swapchain::{ImageResources, Swapchain},
};

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
    CmdPoolCreation(vk::Result),

    #[error("vulkan call to allocated command buffer failed")]
    CmdBufferAllocation(vk::Result),

    #[error("vulkan call to create fence failed")]
    FenceCreation(vk::Result),
}

#[derive(Debug, Error)]
pub enum ImmediateCommandError {
    #[error("immediate command buffer begin failed")]
    Begin(vk::Result),

    #[error("immediate command buffer submission failed")]
    Submission(vk::Result),

    #[error("immediate command fence waiting failed")]
    FenceWaiting(vk::Result),

    #[error("immediate command resources resetting failed")]
    Reset(vk::Result),
}

#[derive(Debug, Error)]
pub enum RenderCommandError {
    #[error("presentation fence sync failed")]
    FenceSync(vk::Result),

    #[error("presentation fence reset failed")]
    FenceReset(vk::Result),

    #[error("render command resources resetting failed")]
    Reset(vk::Result),

    #[error("render command buffer begin failed")]
    Begin(vk::Result),

    #[error("render graph execution failed")]
    RenderGraphRun(#[from] RenderGraphRunError),

    #[error("vulkan call to end command buffer failed")]
    CommandBufferEnd(vk::Result),

    #[error("render command buffer submission failed")]
    Submission(vk::Result),

    #[error("render command fence waiting failed")]
    FenceWaiting(vk::Result),
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
            .map_err(CommandManagerCreateError::CmdPoolCreation)?;

        let cmd_buffer_info = vk::CommandBufferAllocateInfo::default()
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(2)
            .command_pool(cmd_pool);
        let cmd_buffers = unsafe { device.allocate_command_buffers(&cmd_buffer_info) }
            .map_err(CommandManagerCreateError::CmdBufferAllocation)?;

        let fence_info = vk::FenceCreateInfo::default();
        let immediate_fence = unsafe { device.create_fence(&fence_info, None) }
            .map_err(CommandManagerCreateError::FenceCreation)?;

        drop(device);

        Ok(Self {
            cmd_pool,
            rendering_cmd_buffer: cmd_buffers[0],
            immediate_cmd_buffer: cmd_buffers[1],
            immediate_fence,
            device_ref,
        })
    }

    pub(crate) fn render_command<Fn>(
        &self,
        swapchain: &mut Swapchain,
        f: Fn,
    ) -> Result<(), RenderCommandError>
    where
        Fn: FnOnce(&vk::CommandBuffer, ImageResources) -> Result<(), RenderGraphRunError>,
    {
        {
            let device = self.device_ref.lock();

            unsafe {
                device.reset_command_buffer(
                    self.rendering_cmd_buffer,
                    vk::CommandBufferResetFlags::default(),
                )
            }
            .map_err(RenderCommandError::Reset)?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            unsafe { device.begin_command_buffer(self.rendering_cmd_buffer, &begin_info) }
                .map_err(RenderCommandError::Begin)?;
        }

        f(
            &self.rendering_cmd_buffer,
            swapchain.current_image_resources(),
        )?;
        swapchain.ensure_presentable(&self.rendering_cmd_buffer);

        {
            let device = self.device_ref.lock();
            unsafe { device.end_command_buffer(self.rendering_cmd_buffer) }
                .map_err(RenderCommandError::CommandBufferEnd)?;

            let cmd_buffers = [self.rendering_cmd_buffer];
            unsafe {
                device.queue_submit(
                    device.graphics_queue.handle,
                    &[vk::SubmitInfo::default()
                        .command_buffers(&cmd_buffers)
                        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                        .wait_semaphores(&[swapchain.image_acquired_semaphore])
                        .signal_semaphores(&[
                            swapchain.render_semaphores[swapchain.current_image_index as usize]
                        ])],
                    swapchain.present_fence,
                )
            }
            .map_err(RenderCommandError::Submission)?;
        }

        Ok(())
    }

    pub(crate) fn immediate_command<Fn, ReturnType>(
        &self,
        f: Fn,
    ) -> Result<ReturnType, ImmediateCommandError>
    where
        Fn: FnOnce(&vk::CommandBuffer) -> ReturnType,
    {
        {
            let device = self.device_ref.lock();
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            unsafe { device.begin_command_buffer(self.immediate_cmd_buffer, &begin_info) }
                .map_err(ImmediateCommandError::Begin)?;
        }

        let result = f(&self.immediate_cmd_buffer);

        {
            let device = self.device_ref.lock();
            let cmd_buffers = [self.immediate_cmd_buffer];
            let submit_info = vk::SubmitInfo::default().command_buffers(&cmd_buffers);
            unsafe {
                device.queue_submit(
                    device.graphics_queue.handle,
                    &[submit_info],
                    self.immediate_fence,
                )
            }
            .map_err(ImmediateCommandError::Submission)?;

            let fences = [self.immediate_fence];
            unsafe { device.wait_for_fences(&fences, true, u64::MAX) }
                .map_err(ImmediateCommandError::FenceWaiting)?;

            unsafe { device.reset_fences(&fences) }.map_err(ImmediateCommandError::Reset)?;
            unsafe {
                device.reset_command_buffer(
                    self.immediate_cmd_buffer,
                    vk::CommandBufferResetFlags::default(),
                )
            }
            .map_err(ImmediateCommandError::Reset)?;
        }

        Ok(result)
    }
}

impl Drop for CommandManager {
    fn drop(&mut self) {
        let device = self.device_ref.lock();
        log::debug!("Waiting for device to be idle before destroying command manager");
        unsafe { device.device_wait_idle() }.expect("device should wait before shutting down");

        log::debug!("destroying command manager");
        unsafe { device.destroy_fence(self.immediate_fence, None) };
        unsafe { device.destroy_command_pool(self.cmd_pool, None) };
    }
}
