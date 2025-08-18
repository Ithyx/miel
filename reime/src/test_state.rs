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
                FrameResources, ImageAttachmentInfo, ResourceAccessType, ResourceID,
                ResourceInfoRegistry,
            },
        },
    },
    utils::ThreadSafeRwRef,
};

struct GBufferData {
    pub albedo: ResourceID,
    pub normal: ResourceID,
    pub sc_color: ResourceID,
    pub sc_depth: ResourceID,
}
fn record_gbuffer(
    resource_ids: &mut GBufferData,
    resources: &mut FrameResources,
    _cmd_buffer: &vk::CommandBuffer,
    _device_ref: ThreadSafeRwRef<Device>,
) {
    let albedo = resources.get(&resource_ids.albedo).unwrap();
    let normal = resources.get(&resource_ids.normal).unwrap();
    log::info!(
        "found albedo and normal attachments: {:?}, {:?}",
        albedo,
        normal
    );

    let sc_color = resources.get(&resource_ids.sc_color).unwrap();
    let sc_depth = resources.get(&resource_ids.sc_depth).unwrap();
    log::info!(
        "found swapchain color and depth attachments: {:?} {:?}",
        sc_color,
        sc_depth
    );
}

pub struct TestState {}

impl TestState {
    pub fn new(_ctx: &mut gfx::context::Context) -> Self {
        Self {}
    }
}

impl application::ApplicationState for TestState {
    fn on_attach(&mut self, ctx: &mut gfx::context::Context) {
        let mut resources = ResourceInfoRegistry::new();
        let albedo = resources
            .add_image_attachment(
                ImageAttachmentInfo::new("albedo").format(vk::Format::R8G8B8A8_SRGB),
            )
            .expect("resource should be unique");
        let normal = resources
            .add_image_attachment(
                ImageAttachmentInfo::new("normal").format(vk::Format::A2B10G10R10_UNORM_PACK32),
            )
            .expect("resource should be unique");

        let sc_color = ResourceID::SwapchainColorAttachment;
        let sc_depth = ResourceID::SwapchainDSAttachment;

        let gbuffer_data = GBufferData {
            albedo,
            normal,
            sc_color,
            sc_depth,
        };
        let rendergraph_info = RenderGraphInfo::new(resources).push_render_pass(Box::new(
            SimpleRenderPass::new("g-buffer", gbuffer_data)
                .add_color_attachment(albedo, ResourceAccessType::WriteOnly)
                .add_color_attachment(normal, ResourceAccessType::WriteOnly)
                .add_color_attachment(sc_color, ResourceAccessType::WriteOnly)
                .set_depth_stencil_attachment(sc_depth)
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
