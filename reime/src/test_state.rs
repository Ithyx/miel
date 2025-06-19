use miel::{
    application,
    ash::vk,
    gfx::{
        self,
        device::Device,
        render_graph::{
            RenderGraphInfo,
            render_pass::SimpleRenderPass,
            resource::{
                ImageAttachmentDescription, ResourceAccessType, ResourceDescriptionRegistry,
                ResourceID, ResourceRegistry,
            },
        },
        swapchain::ImageResources,
    },
    utils::ThreadSafeRwRef,
};

struct GBufferData {
    pub albedo: ResourceID,
    pub normal: ResourceID,
}
fn record_gbuffer(
    resource_ids: &mut GBufferData,
    resources: &ResourceRegistry,
    swapchain_res: Option<&ImageResources>,
    _cmd_buffer: &vk::CommandBuffer,
    _device_ref: ThreadSafeRwRef<Device>,
) {
    let albedo = resources.attachments.get(&resource_ids.albedo).unwrap();
    let normal = resources.attachments.get(&resource_ids.normal).unwrap();

    log::info!(
        "found albedo and normal attachments: {:?}, {:?}",
        albedo.image.handle,
        normal.image.handle
    );

    swapchain_res.inspect(|resources| {
        log::info!(
            "\talso got swapchain resources: {:?}, {:?}",
            resources.color_image.handle,
            resources.depth_image.handle
        )
    });
}

pub struct TestState {}

impl TestState {
    pub fn new(_ctx: &mut gfx::context::Context) -> Self {
        Self {}
    }
}

impl application::ApplicationState for TestState {
    fn on_attach(&mut self, ctx: &mut gfx::context::Context) {
        let mut resources = ResourceDescriptionRegistry::new();
        let albedo = resources
            .add_image_attachment(
                ImageAttachmentDescription::new("albedo").format(vk::Format::R8G8B8A8_SRGB),
            )
            .expect("resource should be unique");
        let normal = resources
            .add_image_attachment(
                ImageAttachmentDescription::new("normal")
                    .format(vk::Format::A2B10G10R10_UNORM_PACK32),
            )
            .expect("resource should be unique");

        let gbuffer_data = GBufferData { albedo, normal };
        let rendergraph_info = RenderGraphInfo::new(resources).push_render_pass(Box::new(
            SimpleRenderPass::new("g-buffer", gbuffer_data)
                .add_color_attachment(albedo, ResourceAccessType::WriteOnly)
                .add_color_attachment(normal, ResourceAccessType::WriteOnly)
                .request_swapchain_resources(ResourceAccessType::WriteOnly)
                .set_command_recorder(Box::new(record_gbuffer)),
        ));

        ctx.bind_rendergraph(rendergraph_info)
            .expect("rendergraph should be valid and bound");
    }

    fn update(&mut self, _ctx: &mut gfx::context::Context) -> miel::application::ControlFlow {
        // log::info!("update !");
        // log::info!("...and exit.");

        miel::application::ControlFlow::Continue
    }
}
