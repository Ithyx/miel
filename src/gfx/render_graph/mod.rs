pub mod render_pass;
pub mod resource;

use ash::vk;
use render_pass::RenderPass;
use resource::{RegistryCreateError, ResourceDescriptionRegistry, ResourceRegistry};
use thiserror::Error;

use crate::utils::ThreadSafeRef;

use super::{context::Context, device::Device, swapchain};

pub struct RenderGraphInfo {
    render_passes: Vec<Box<dyn RenderPass>>,

    resource_descriptions: ResourceDescriptionRegistry,
}

impl RenderGraphInfo {
    pub fn new(resources: ResourceDescriptionRegistry) -> Self {
        Self {
            resource_descriptions: resources,
            render_passes: Default::default(),
        }
    }

    pub fn push_render_pass(mut self, render_pass: Box<dyn RenderPass>) -> Self {
        self.render_passes.push(render_pass);
        self
    }
}

pub(crate) struct RenderGraph {
    render_passes: Vec<Box<dyn RenderPass>>,

    resources: ResourceRegistry,
}

#[derive(Debug, Error)]
pub enum RenderGraphCreateError {
    #[error("resource registry creation failed")]
    ResourceCreation(#[from] RegistryCreateError),
}

#[derive(Debug, Error)]
pub enum RenderGraphRunError {
    #[error("resource registry creation failed")]
    ResourceCreation(#[from] RegistryCreateError),
}

impl RenderGraph {
    pub(crate) fn empty() -> Self {
        Self {
            render_passes: vec![],
            resources: ResourceRegistry::default(),
        }
    }

    pub(crate) fn new(
        info: RenderGraphInfo,
        ctx: &mut Context,
    ) -> Result<Self, RenderGraphCreateError> {
        let resources = info.resource_descriptions.create_resources(ctx)?;

        Ok(Self {
            render_passes: info.render_passes,
            resources,
        })
    }

    pub(crate) fn render(
        &self,
        mut _swapchain_resources: swapchain::ImageResources,
        cmd_buffer: &vk::CommandBuffer,
        device_ref: ThreadSafeRef<Device>,
    ) -> Result<(), RenderGraphRunError> {
        // actually render here

        Ok(())
    }
}
