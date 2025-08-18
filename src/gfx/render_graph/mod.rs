pub mod render_pass;
pub mod resource;

use ash::vk;
use render_pass::RenderPass;
use resource::{GraphResourceRegistry, RegistryCreateError, ResourceInfoRegistry};
use thiserror::Error;

use crate::{gfx::render_graph::resource::ResourceAccessType, utils::ThreadSafeRwRef};

use super::{context::Context, device::Device, swapchain};

pub struct RenderGraphInfo {
    render_passes: Vec<Box<dyn RenderPass>>,
    resource_infos: ResourceInfoRegistry,
}

impl RenderGraphInfo {
    pub fn new(resources: ResourceInfoRegistry) -> Self {
        Self {
            render_passes: Default::default(),
            resource_infos: resources,
        }
    }

    pub fn push_render_pass(mut self, render_pass: Box<dyn RenderPass>) -> Self {
        self.render_passes.push(render_pass);
        self
    }
}

pub(crate) struct RenderGraph {
    render_passes: Vec<Box<dyn RenderPass>>,
    resources: GraphResourceRegistry,
}

#[derive(Debug, Error)]
pub enum RenderGraphCreateError {
    #[error("resource registry creation failed")]
    ResourceCreation(#[from] RegistryCreateError),
}

#[derive(Debug, Error)]
pub enum RenderGraphRunError {
    #[error("a resource requested by a render pass is invalid")]
    InvalidResource,
}

impl RenderGraph {
    pub(crate) fn empty() -> Self {
        Self {
            render_passes: vec![],
            resources: GraphResourceRegistry::default(),
        }
    }

    pub(crate) fn new(
        info: RenderGraphInfo,
        ctx: &mut Context,
    ) -> Result<Self, RenderGraphCreateError> {
        let resources = info.resource_infos.create_resources(ctx)?;

        Ok(Self {
            render_passes: info.render_passes,
            resources,
        })
    }

    pub(crate) fn render(
        &mut self,
        swapchain_resources: swapchain::ImageResources<'_>,
        &cmd_buffer: &vk::CommandBuffer,
        device_ref: &ThreadSafeRwRef<Device>,
    ) -> Result<(), RenderGraphRunError> {
        for render_pass in &mut self.render_passes {
            let attachment_info = render_pass.attachment_infos();
            let pipeline_barrier = vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
            for (&res_id, access_type) in &attachment_info.color_attachments {
                let color_attachment = match res_id {
                    resource::ResourceID::SwapchainColorAttachment => {
                        Some(&mut *swapchain_resources.color_image)
                    }
                    resource::ResourceID::SwapchainDSAttachment => None,
                    resource::ResourceID::Other(uuid) => self
                        .resources
                        .attachments
                        .get_mut(&uuid)
                        .map(|attachment| &mut attachment.image.state),
                }
                .ok_or(RenderGraphRunError::InvalidResource)?;
                let dst_access_mask = match access_type {
                    ResourceAccessType::ReadOnly => vk::AccessFlags::COLOR_ATTACHMENT_READ,
                    ResourceAccessType::WriteOnly => vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    ResourceAccessType::ReadWrite => {
                        vk::AccessFlags::COLOR_ATTACHMENT_READ
                            | vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                    }
                };

                let pipeline_barrier = pipeline_barrier.dst_access_mask(dst_access_mask);
                // TODO: THIS SHOULD FAIL BECAUSE THIS FUNCTION REQUIRES MUT BUT IS CLONED
                color_attachment.cmd_layout_transition(
                    device_ref.clone(),
                    cmd_buffer,
                    pipeline_barrier,
                );
            }

            let rendering_info = &vk::RenderingInfo::default()
                .render_area(
                    vk::Rect2D::default().extent(swapchain_resources.color_image.extent_2d),
                )
                .layer_count(1);

            let mut color_attachments = vec![];
            for &ca_id in attachment_info.color_attachments.keys() {
                let color_attachment_state = match ca_id {
                    resource::ResourceID::SwapchainColorAttachment => {
                        Some(&mut *swapchain_resources.color_image)
                    }
                    resource::ResourceID::SwapchainDSAttachment => None,
                    resource::ResourceID::Other(uuid) => self
                        .resources
                        .attachments
                        .get_mut(&uuid)
                        .map(|attachment| &mut attachment.image.state),
                }
                .ok_or(RenderGraphRunError::InvalidResource)?;

                let color_attachment = vk::RenderingAttachmentInfo::default()
                    .image_view(color_attachment_state.view)
                    .image_layout(color_attachment_state.layout)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(vk::ClearValue::default());

                color_attachments.push(color_attachment);
            }
            let rendering_info = rendering_info.color_attachments(&color_attachments);

            let mut depth_attachment = vk::RenderingAttachmentInfo::default();
            if let Some(da_id) = attachment_info.depth_stencil_attachment {
                let depth_attachment_state = match da_id {
                    resource::ResourceID::SwapchainColorAttachment => None,
                    resource::ResourceID::SwapchainDSAttachment => {
                        Some(&mut swapchain_resources.depth_image.state)
                    }
                    resource::ResourceID::Other(uuid) => self
                        .resources
                        .attachments
                        .get_mut(&uuid)
                        .map(|attachment| &mut attachment.image.state),
                }
                .ok_or(RenderGraphRunError::InvalidResource)?;

                depth_attachment = depth_attachment
                    .image_view(depth_attachment_state.view)
                    .image_layout(depth_attachment_state.layout)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(vk::ClearValue::default());
            }
            let rendering_info = rendering_info.depth_attachment(&depth_attachment);

            unsafe {
                device_ref
                    .read()
                    .cmd_begin_rendering(cmd_buffer, &rendering_info)
            };

            render_pass.record_commands(&mut self.resources, &cmd_buffer, device_ref.clone());

            unsafe { device_ref.read().cmd_end_rendering(cmd_buffer) };
        }

        Ok(())
    }
}
