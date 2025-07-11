use std::collections::HashMap;

use ash::vk;

use crate::{
    gfx::{device::Device, swapchain::ImageResources},
    utils::ThreadSafeRwRef,
};

use super::resource::{ResourceAccessType, ResourceID, ResourceRegistry};

#[derive(Debug, Default, Clone)]
pub struct AttachmentInfo {
    pub color_attachments: HashMap<ResourceID, ResourceAccessType>,
    pub depth_attachments: HashMap<ResourceID, ResourceAccessType>,

    pub swapchain_resources: Option<ResourceAccessType>,
}

pub trait RenderPass {
    fn name(&self) -> &str;
    fn attachment_infos(&self) -> &AttachmentInfo;

    fn record_commands(
        &mut self,
        resources: &ResourceRegistry,
        swapchain_res: Option<&ImageResources>,
        cmd_buffer: &vk::CommandBuffer,
        device_ref: ThreadSafeRwRef<Device>,
    );
}

pub type SimpleCommandRecorder<UserData> = Box<
    dyn FnMut(
        &mut UserData,
        &ResourceRegistry,
        Option<&ImageResources>,
        &vk::CommandBuffer,
        ThreadSafeRwRef<Device>,
    ),
>;

pub struct SimpleRenderPass<UserData> {
    pub name: String,
    pub attachment_infos: AttachmentInfo,
    pub user_data: UserData,

    pub command_recorder: SimpleCommandRecorder<UserData>,
}

impl<UserData> SimpleRenderPass<UserData> {
    pub fn new(name: &str, user_data: UserData) -> Self {
        Self {
            name: name.to_owned(),
            user_data,
            attachment_infos: AttachmentInfo::default(),
            command_recorder: Box::new(|_, _, _, _, _| {}),
        }
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
        self.attachment_infos
            .color_attachments
            .insert(ressource, access_type);
        self
    }

    pub fn add_depth_attachment(
        mut self,
        ressource: ResourceID,
        access_type: ResourceAccessType,
    ) -> Self {
        self.attachment_infos
            .depth_attachments
            .insert(ressource, access_type);
        self
    }

    pub fn request_swapchain_resources(mut self, access_type: ResourceAccessType) -> Self {
        self.attachment_infos.swapchain_resources = Some(access_type);
        self
    }

    pub fn set_command_recorder(
        mut self,
        command_recorder: SimpleCommandRecorder<UserData>,
    ) -> Self {
        self.command_recorder = command_recorder;
        self
    }
}

impl<UserData> RenderPass for SimpleRenderPass<UserData> {
    fn name(&self) -> &str {
        &self.name
    }

    fn attachment_infos(&self) -> &AttachmentInfo {
        &self.attachment_infos
    }

    fn record_commands(
        &mut self,
        resources: &ResourceRegistry,
        swapchain_res: Option<&ImageResources>,
        cmd_buffer: &vk::CommandBuffer,
        device_ref: ThreadSafeRwRef<Device>,
    ) {
        (self.command_recorder)(
            &mut self.user_data,
            resources,
            swapchain_res,
            cmd_buffer,
            device_ref,
        );
    }
}
