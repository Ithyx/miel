use std::collections::HashMap;

use ash::vk;
use thiserror::Error;
use uuid::Uuid;

use crate::gfx::{
    context::Context,
    image::{Image, ImageBuildError, ImageCreateInfo},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ResourceID {
    SwapchainColorAttachment,
    SwapchainDSAttachment,
    Other(Uuid),
}

#[derive(Debug, Copy, Clone)]
pub enum ResourceAccessType {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[derive(Debug, Copy, Clone)]
pub enum AttachmentSize {
    SwapchainBased,
    Custom(vk::Extent3D),
}

#[derive(Debug)]
pub struct ImageAttachmentInfo {
    pub(crate) id: ResourceID,
    pub name: String,

    pub size: AttachmentSize,
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
    pub layer_count: u32,
}

impl Default for ImageAttachmentInfo {
    fn default() -> Self {
        Self {
            id: ResourceID::Other(Uuid::new_v4()),
            name: "".to_owned(),
            size: AttachmentSize::SwapchainBased,
            format: vk::Format::UNDEFINED,
            usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            layer_count: 1,
        }
    }
}

impl Clone for ImageAttachmentInfo {
    fn clone(&self) -> Self {
        Self {
            id: ResourceID::Other(Uuid::new_v4()),
            name: self.name.clone(),
            size: self.size,
            format: self.format,
            usage: self.usage,
            layer_count: self.layer_count,
        }
    }
}

impl ImageAttachmentInfo {
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

pub struct ImageAttachment {
    pub image: Image,
    pub info: ImageAttachmentInfo,
}

#[derive(Debug, Error)]
pub enum ImageAttachmentCreateError {
    #[error("image creation failed")]
    ImageCreation(#[from] ImageBuildError),
}

impl ImageAttachment {
    pub fn from_info(
        attachment_info: ImageAttachmentInfo,
        ctx: &mut Context,
    ) -> Result<Self, ImageAttachmentCreateError> {
        let image = ImageCreateInfo::from_attachment_info(&attachment_info).build(ctx)?;

        Ok(Self {
            image,
            info: attachment_info,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ResourceInfoRegistry {
    infos: HashMap<Uuid, ImageAttachmentInfo>,
}

#[derive(Debug, Clone, Copy, Error)]
pub enum ResourceInfoInsertError {
    #[error("resource info is already present in this registry")]
    AlreadyPresent,
}

impl ResourceInfoRegistry {
    pub fn new() -> Self {
        Self {
            infos: Default::default(),
        }
    }

    pub fn add_image_attachment(
        &mut self,
        info: ImageAttachmentInfo,
    ) -> Result<ResourceID, ResourceInfoInsertError> {
        let uuid = match info.id {
            ResourceID::SwapchainColorAttachment => {
                unreachable!("Only a local resource can be added")
            }
            ResourceID::SwapchainDSAttachment => unreachable!("Only a local resource can be added"),
            ResourceID::Other(uuid) => uuid,
        };
        let previous = self.infos.insert(uuid, info);

        match previous {
            Some(_) => Err(ResourceInfoInsertError::AlreadyPresent),
            None => Ok(ResourceID::Other(uuid)),
        }
    }

    pub(crate) fn create_resources(
        self,
        ctx: &mut Context,
    ) -> Result<GraphResourceRegistry, RegistryCreateError> {
        let attachments = self
            .infos
            .into_iter()
            .map(|(id, info)| match ImageAttachment::from_info(info, ctx) {
                Ok(attachment) => Ok((id, attachment)),
                Err(err) => Err(RegistryCreateError::ImageAttachmentCreation(err)),
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(GraphResourceRegistry { attachments })
    }
}

impl Default for ResourceInfoRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum RegistryCreateError {
    #[error("image attachment creation failed")]
    ImageAttachmentCreation(#[from] ImageAttachmentCreateError),
}

#[derive(Default)]
pub struct GraphResourceRegistry {
    pub attachments: HashMap<Uuid, ImageAttachment>,
}

impl GraphResourceRegistry {
    pub fn get(&self, id: &ResourceID) -> Option<&ImageAttachment> {
        match id {
            ResourceID::SwapchainColorAttachment => todo!(),
            ResourceID::SwapchainDSAttachment => todo!(),
            ResourceID::Other(uuid) => self.attachments.get(uuid),
        }
    }

    pub fn get_mut(&mut self, id: &ResourceID) -> Option<&mut ImageAttachment> {
        match id {
            ResourceID::SwapchainColorAttachment => todo!(),
            ResourceID::SwapchainDSAttachment => todo!(),
            ResourceID::Other(uuid) => self.attachments.get_mut(uuid),
        }
    }
}
