use std::collections::HashMap;

use ash::vk;
use thiserror::Error;

use crate::gfx::{
    context::Context,
    image::{Image, ImageBuildError, ImageCreateInfo, ImageState},
    swapchain::ImageResources,
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
            id: ResourceID::new_v4(),
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
            id: ResourceID::new_v4(),
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

#[derive(Debug, Clone, Default)]
pub struct ResourceInfoRegistry {
    infos: HashMap<ResourceID, ImageAttachmentInfo>,
    swapchain_color_attachment: ResourceID,
    swapchain_ds_attachment: ResourceID,
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
            swapchain_color_attachment: ResourceID::new_v4(),
            swapchain_ds_attachment: ResourceID::new_v4(),
        }
    }

    pub fn add_image_attachment(
        &mut self,
        info: ImageAttachmentInfo,
    ) -> Result<ResourceID, ResourceInfoInsertError> {
        let id = info.id;
        let previous = self.infos.insert(id, info);

        match previous {
            Some(_) => Err(ResourceInfoInsertError::AlreadyPresent),
            None => Ok(id),
        }
    }

    pub fn swapchain_color_attachment(&self) -> ResourceID {
        self.swapchain_color_attachment
    }

    pub fn swapchain_ds_attachment(&self) -> ResourceID {
        self.swapchain_ds_attachment
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

        Ok(GraphResourceRegistry {
            attachments,
            swapchain_color_attachment: self.swapchain_color_attachment,
            swapchain_ds_attachment: self.swapchain_ds_attachment,
        })
    }
}

#[derive(Debug, Error)]
pub enum RegistryCreateError {
    #[error("image attachment creation failed")]
    ImageAttachmentCreation(#[from] ImageAttachmentCreateError),
}

#[derive(Default)]
pub struct GraphResourceRegistry {
    pub attachments: HashMap<ResourceID, ImageAttachment>,
    pub(crate) swapchain_color_attachment: ResourceID,
    pub(crate) swapchain_ds_attachment: ResourceID,
}

pub struct FrameResourceRegistry<'a> {
    pub(crate) graph_resources: &'a GraphResourceRegistry,
    pub(crate) frame_resources: &'a ImageResources<'a>,
}

impl FrameResourceRegistry<'_> {
    pub fn get_image_state(&self, id: ResourceID) -> Option<ImageState> {
        if self.graph_resources.swapchain_color_attachment == id {
            return Some(*self.frame_resources.color_image);
        }
        if self.graph_resources.swapchain_ds_attachment == id {
            return Some(self.frame_resources.depth_image.state);
        }

        self.graph_resources
            .attachments
            .get(&id)
            .map(|attachment| attachment.image.state)
    }
}
