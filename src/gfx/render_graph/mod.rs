pub mod render_pass;
pub mod resource;

use ash::vk;
use render_pass::RenderPass;
use resource::{GraphResourceRegistry, RegistryCreateError, ResourceInfoRegistry};
use thiserror::Error;

use crate::{gfx::render_graph::resource::FrameResourceRegistry, utils::ThreadSafeRwRef};

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
pub enum RenderGraphRunError {}

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
        swapchain_resources: swapchain::ImageResources,
        &cmd_buffer: &vk::CommandBuffer,
        device_ref: &ThreadSafeRwRef<Device>,
    ) -> Result<(), RenderGraphRunError> {
        for render_pass in &mut self.render_passes {
            let attachment_info = render_pass.attachment_infos();
            let resources = FrameResourceRegistry {
                graph_resources: &self.resources,
                frame_resources: &swapchain_resources,
            };
            // todo: prepare input resources

            let rendering_info = &vk::RenderingInfo::default()
                .render_area(vk::Rect2D::default().extent(swapchain_resources.color_image.extent))
                .layer_count(1);

            let color_attachments = vec![];
            for (ca_id, ca_access) in &attachment_info.color_attachments {}

            let rendering_info = rendering_info.color_attachments(&color_attachments);

            unsafe {
                device_ref
                    .read()
                    .cmd_begin_rendering(cmd_buffer, &rendering_info)
            };

            render_pass.record_commands(resources, &cmd_buffer, device_ref.clone());

            unsafe { device_ref.read().cmd_end_rendering(cmd_buffer) };
        }

        Ok(())
    }
}
