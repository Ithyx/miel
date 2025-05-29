pub mod resource;

use std::collections::HashMap;

use resource::{
    RegistryCreateError, ResourceAccessType, ResourceDescriptionRegistry, ResourceID,
    ResourceRegistry,
};
use thiserror::Error;

use super::context::Context;

#[derive(Debug, Clone)]
pub struct RenderPass {
    pub name: String,

    pub color_attachments: HashMap<ResourceID, ResourceAccessType>,
    pub depth_attachments: HashMap<ResourceID, ResourceAccessType>,
}

impl Default for RenderPass {
    fn default() -> Self {
        Self {
            name: "".to_owned(),
            color_attachments: Default::default(),
            depth_attachments: Default::default(),
        }
    }
}

impl RenderPass {
    pub fn new(name: &str) -> Self {
        Self::default().name(name)
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_owned();
        self
    }

    pub fn add_color_attachment(
        mut self,
        ressource: ResourceID,
        access_type: ResourceAccessType,
    ) -> Self {
        self.color_attachments.insert(ressource, access_type);
        self
    }

    pub fn add_depth_attachment(
        mut self,
        ressource: ResourceID,
        access_type: ResourceAccessType,
    ) -> Self {
        self.depth_attachments.insert(ressource, access_type);
        self
    }
}

#[derive(Debug)]
pub struct RenderGraphInfo {
    resource_descriptions: ResourceDescriptionRegistry,
    render_passes: Vec<RenderPass>,
}

impl RenderGraphInfo {
    pub fn new(resources: ResourceDescriptionRegistry) -> Self {
        Self {
            resource_descriptions: resources,
            render_passes: Default::default(),
        }
    }

    pub fn push_render_pass(mut self, render_pass: RenderPass) -> Self {
        self.render_passes.push(render_pass);
        self
    }
}

pub(crate) struct RenderGraph {
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

        Ok(Self { resources })
    }
}
