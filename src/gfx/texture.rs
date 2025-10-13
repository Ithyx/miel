use std::fmt::Debug;

use ash::vk;
use thiserror::Error;

use crate::{
    gfx::{
        context::Context,
        debug::debug_name_vk_object,
        device::Device,
        image::{Image, ImageBuildError},
    },
    utils::ThreadSafeRwRef,
};

#[derive(Debug)]
pub struct TextureBuilder<'a> {
    name: &'a str,

    pub format: vk::Format,
    pub layout: vk::ImageLayout,
    pub usage: vk::ImageUsageFlags,

    pub min_filer: vk::Filter,
    pub mag_filter: vk::Filter,
    pub address_mode: vk::SamplerAddressMode,
}

#[derive(Debug, Error)]
pub enum TextureBuildError {
    #[error("building of the underlying image failed")]
    ImageBuilding(#[from] ImageBuildError),

    #[error("vulkan creation of the texture sampler failed")]
    VulkanSamplerCreation(vk::Result),
}

impl<'a> TextureBuilder<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            format: vk::Format::R8G8B8A8_SRGB,
            layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            usage: vk::ImageUsageFlags::SAMPLED,

            min_filer: vk::Filter::NEAREST,
            mag_filter: vk::Filter::NEAREST,
            address_mode: vk::SamplerAddressMode::REPEAT,
        }
    }

    pub fn with_format(mut self, format: vk::Format) -> Self {
        self.format = format;
        self
    }

    pub fn with_layout(mut self, layout: vk::ImageLayout) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_usage(mut self, usage: vk::ImageUsageFlags) -> Self {
        self.usage = usage;
        self
    }

    pub fn with_min_filter(mut self, min_filter: vk::Filter) -> Self {
        self.min_filer = min_filter;
        self
    }

    pub fn with_mag_filter(mut self, mag_filter: vk::Filter) -> Self {
        self.mag_filter = mag_filter;
        self
    }

    pub fn with_address_mode(mut self, address_mode: vk::SamplerAddressMode) -> Self {
        self.address_mode = address_mode;
        self
    }

    pub fn build(self, dimensions: &[u32; 2], ctx: &Context) -> Result<Texture, TextureBuildError> {
        let pattern = [255, 255, 255, 255, 255, 0, 255, 255];
        let data = pattern
            .iter()
            .cycle()
            .take((4 * dimensions[0] * dimensions[1]).try_into().unwrap())
            .copied()
            .collect::<Vec<_>>();

        self.build_from_data(
            &data,
            DataSource::Procedural,
            dimensions[0],
            dimensions[1],
            ctx,
        )
    }

    pub fn build_from_data(
        self,
        data: &[u8],
        data_source: DataSource,
        width: u32,
        height: u32,
        ctx: &Context,
    ) -> Result<Texture, TextureBuildError> {
        let image = Image::builder(
            self.name,
            vk::Extent3D::default().width(width).height(height).depth(1),
        )
        .texture_default(self.format)
        .with_layout(self.layout)
        .with_usage(self.usage)
        .with_data(data.to_vec())
        .build(ctx)?;

        let device = ctx.device_ref.read();
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(self.mag_filter)
            .min_filter(self.min_filer)
            .address_mode_u(self.address_mode)
            .address_mode_v(self.address_mode)
            .address_mode_w(self.address_mode);
        let sampler = unsafe { device.create_sampler(&sampler_info, None) }
            .map_err(TextureBuildError::VulkanSamplerCreation)?;

        if cfg!(debug_assertions) {
            // Not the end of the world if naming fails for whatever reason
            let _ = debug_name_vk_object(ctx, sampler, self.name);
        }

        Ok(Texture {
            name: self.name.to_owned(),
            image,
            sampler,
            data_source,
            dimensions: [width, height],
            format: self.format,
            device_ref: ctx.device_ref.clone(),
        })
    }
}

#[derive(Debug)]
pub enum DataSource {
    Path(String),
    Procedural,
}

pub struct Texture {
    pub name: String,

    pub image: Image,
    pub sampler: vk::Sampler,

    pub data_source: DataSource,
    pub dimensions: [u32; 2],
    format: vk::Format,

    // bookkeeping
    device_ref: ThreadSafeRwRef<Device>,
}

impl Texture {
    pub fn builder(name: &'_ str) -> TextureBuilder<'_> {
        TextureBuilder::new(name)
    }
}

impl Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("name", &self.name)
            .field("image", &self.image)
            .field("sampler", &self.sampler)
            .field("data_source", &self.data_source)
            .field("dimensions", &self.dimensions)
            .field("format", &self.format)
            .finish()
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        let device = self.device_ref.read();

        unsafe { device.destroy_sampler(self.sampler, None) };
    }
}
