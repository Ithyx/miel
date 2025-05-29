use std::collections::HashMap;

use ash::vk;
use thiserror::Error;

use crate::gfx::{
    context::Context,
    image::{Image, ImageBuildError, ImageCreateInfo},
};

pub type ResourceID = uuid::Uuid;

#[derive(Debug, Copy, Clone)]
pub enum ResourceAccessType {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[derive(Debug, Copy, Clone)]
pub enum AttachmentSize {
    Swapchain,
    Custom(vk::Extent3D),
}

#[derive(Debug)]
pub struct ImageAttachmentDescription {
    pub(crate) id: ResourceID,
    pub name: String,

    pub size: AttachmentSize,
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
    pub layer_count: u32,
}

impl Default for ImageAttachmentDescription {
    fn default() -> Self {
        Self {
            id: ResourceID::new_v4(),
            name: "".to_owned(),
            size: AttachmentSize::Swapchain,
            format: vk::Format::UNDEFINED,
            usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            layer_count: 1,
        }
    }
}

impl Clone for ImageAttachmentDescription {
    fn clone(&self) -> Self {
        Self {
            id: ResourceID::new_v4(),
            name: self.name.clone(),
            size: self.size,
            format: self.format,
            usage: self.usage,
            layer_count: self.layer_count,
        }
    }
}

impl ImageAttachmentDescription {
    pub fn new(name: &str) -> Self {
        Self::default().name(name)
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_owned();
        self
    }
    pub fn size(mut self, size: AttachmentSize) -> Self {
        self.size = size;
        self
    }
    pub fn format(mut self, format: vk::Format) -> Self {
        self.format = format;
        self
    }
    pub fn usage(mut self, usage: vk::ImageUsageFlags) -> Self {
        self.usage = usage;
        self
    }
    pub fn layer_count(mut self, layer_count: u32) -> Self {
        self.layer_count = layer_count;
        self
    }
}

pub(crate) struct ImageAttachment {
    pub image: Image,
    pub description: ImageAttachmentDescription,
}

#[derive(Debug, Error)]
pub enum ImageAttachmentCreateError {
    #[error("image creation failed")]
    ImageCreation(#[from] ImageBuildError),
}

impl ImageAttachment {
    pub fn from_description(
        description: ImageAttachmentDescription,
        ctx: &mut Context,
    ) -> Result<Self, ImageAttachmentCreateError> {
        let image = ImageCreateInfo::from_attachment_description(&description).build(ctx)?;

        Ok(Self { image, description })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResourceDescriptionRegistry {
    attachments: HashMap<ResourceID, ImageAttachmentDescription>,
}

#[derive(Debug, Clone, Copy, Error)]
pub enum ResourceDescriptionInsertError {
    #[error("resource description is already present in this registry")]
    AlreadyPresent,
}

impl ResourceDescriptionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_image_attachment(
        &mut self,
        resource: ImageAttachmentDescription,
    ) -> Result<ResourceID, ResourceDescriptionInsertError> {
        let id = resource.id;
        let previous = self.attachments.insert(id, resource);

        match previous {
            Some(_) => Err(ResourceDescriptionInsertError::AlreadyPresent),
            None => Ok(id),
        }
    }

    pub(crate) fn create_resources(
        self,
        ctx: &mut Context,
    ) -> Result<ResourceRegistry, RegistryCreateError> {
        let attachments = self
            .attachments
            .into_iter()
            .map(
                |(id, description)| match ImageAttachment::from_description(description, ctx) {
                    Ok(attachment) => Ok((id, attachment)),
                    Err(err) => Err(RegistryCreateError::ImageAttachmentCreation(err)),
                },
            )
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(ResourceRegistry { attachments })
    }
}

#[derive(Debug, Error)]
pub enum RegistryCreateError {
    #[error("image attachment creation failed")]
    ImageAttachmentCreation(#[from] ImageAttachmentCreateError),
}

pub(crate) struct ResourceRegistry {
    attachments: HashMap<ResourceID, ImageAttachment>,
}
