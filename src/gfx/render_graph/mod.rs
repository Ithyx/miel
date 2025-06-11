pub mod render_pass;
pub mod resource;

use render_pass::RenderPass;
use resource::{RegistryCreateError, ResourceDescriptionRegistry, ResourceRegistry};
use thiserror::Error;

use super::context::Context;

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

impl RenderGraph {
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
}
