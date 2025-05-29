use miel::{
    application,
    ash::vk,
    gfx::{
        self,
        render_graph::{
            RenderGraphInfo, RenderPass,
            resource::{
                ImageAttachmentDescription, ResourceAccessType, ResourceDescriptionRegistry,
            },
        },
    },
};

pub struct TestState {}

impl TestState {
    pub fn new(ctx: &mut gfx::context::Context) -> Self {
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

        let rendergraph_info = RenderGraphInfo::new(resources).push_render_pass(
            RenderPass::new("g-buffer")
                .add_color_attachment(albedo, ResourceAccessType::WriteOnly)
                .add_color_attachment(normal, ResourceAccessType::WriteOnly),
        );

        ctx.bind_rendergraph(rendergraph_info)
            .expect("rendergraph should be valid and bound");

        Self {}
    }
}

impl application::ApplicationState for TestState {
    fn update(&self, _ctx: &mut gfx::context::Context) -> miel::application::ControlFlow {
        log::info!("update !");
        log::info!("...and exit.");

        miel::application::ControlFlow::Exit
    }
}
