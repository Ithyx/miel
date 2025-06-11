use std::collections::HashMap;

use ash::vk;

use crate::gfx;

use super::resource::{ResourceAccessType, ResourceID};

#[derive(Debug, Default, Clone)]
pub struct AttachmentInfo {
    pub color_attachments: HashMap<ResourceID, ResourceAccessType>,
    pub depth_attachments: HashMap<ResourceID, ResourceAccessType>,
}

pub trait RenderPass {
    fn name(&self) -> &str;
    fn attachment_infos(&self) -> &AttachmentInfo;

    fn record_commands(&mut self, cmd_buffer: &vk::CommandBuffer, ctx: &mut gfx::context::Context);
}

pub type SimpleCommandRecorder<UserData> =
    Box<dyn FnMut(&mut UserData, &vk::CommandBuffer, &mut gfx::context::Context)>;

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
            command_recorder: Box::new(|_, _, _| {}),
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

    fn record_commands(&mut self, cmd_buffer: &vk::CommandBuffer, ctx: &mut gfx::context::Context) {
        (self.command_recorder)(&mut self.user_data, cmd_buffer, ctx);
    }
}
