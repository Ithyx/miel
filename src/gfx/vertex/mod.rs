pub mod simple;

use ash::vk;
use ply_rs::ply;

pub struct VertexInputDescription {
    pub bindings: Vec<vk::VertexInputBindingDescription>,
    pub attributes: Vec<vk::VertexInputAttributeDescription>,
}

pub trait Vertex: Copy + Sync + Send + 'static + std::fmt::Debug {
    fn vertex_input_description() -> VertexInputDescription;
    fn position_index() -> usize {
        0
    }
    fn position_offset() -> u32 {
        0
    }
}

// Utilities for ser/deser
pub(crate) struct Face {
    indices: Vec<u32>,
}

impl ply::PropertyAccess for Face {
    fn new() -> Self {
        Self {
            indices: Vec::default(),
        }
    }

    #[allow(clippy::single_match)]
    fn set_property(&mut self, key: String, property: ply::Property) {
        match (key.as_ref(), property) {
            ("vertex_indices", ply::Property::ListUInt(v)) => self.indices = v,
            (_, _) => (),
        }
    }
}
