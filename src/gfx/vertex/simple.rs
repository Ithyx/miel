use std::mem::offset_of;

use ash::vk;
use ply_rs::{parser, ply};
use thiserror::Error;

use crate::{
    gfx::{
        context::Context,
        mesh::{Mesh, MeshDataUploadError, upload_mesh_data},
    },
    math::Vec3,
    utils::ThreadSafeRef,
};

use super::{Face, Vertex, VertexInputDescription};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SimpleVertex {
    pub position: Vec3,
}

impl Vertex for SimpleVertex {
    fn vertex_input_description() -> VertexInputDescription {
        let main_binding = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(
                std::mem::size_of::<SimpleVertex>()
                    .try_into()
                    .expect("unsupported architecture"),
            )
            .input_rate(vk::VertexInputRate::VERTEX);

        let position = vk::VertexInputAttributeDescription::default()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(
                offset_of!(SimpleVertex, position)
                    .try_into()
                    .expect("unsupported architecture"),
            );

        VertexInputDescription {
            bindings: vec![main_binding],
            attributes: vec![position],
        }
    }
}

impl ply::PropertyAccess for SimpleVertex {
    fn new() -> Self {
        Self {
            position: Vec3::default(),
        }
    }

    fn set_property(&mut self, key: String, property: ply::Property) {
        match (key.as_ref(), property) {
            ("x", ply::Property::Float(v)) => self.position.x = v,
            ("y", ply::Property::Float(v)) => self.position.y = v,
            ("z", ply::Property::Float(v)) => self.position.z = v,
            (_, _) => (),
        }
    }
}

#[derive(Error, Debug)]
pub enum SimpleVertexMeshLoadingError {
    #[error("obj file loading failed")]
    OBJLoad(#[from] tobj::LoadError),

    #[error("mesh data upload failed")]
    MeshDataUploadFailed(#[from] MeshDataUploadError),

    #[error("file reading failed")]
    FileReadingError(#[from] std::io::Error),
}

impl SimpleVertex {
    pub fn load_model_from_path_obj(
        path: &std::path::Path,
        ctx: &mut Context,
    ) -> Result<ThreadSafeRef<Mesh<Self>>, SimpleVertexMeshLoadingError> {
        let (load_result, _) = tobj::load_obj(
            path,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )?;

        let mesh = &load_result[0].mesh;

        let positions = mesh
            .positions
            .chunks_exact(3)
            .map(|slice| Vec3::new(slice[0], slice[1], slice[2]))
            .collect::<Vec<Vec3>>();

        let mut vertices = Vec::with_capacity(positions.len());
        for position in positions {
            vertices.push(SimpleVertex { position });
        }

        let indices = mesh.indices.clone();

        let upload_result = upload_mesh_data(&vertices, &indices, ctx)?;

        Ok(ThreadSafeRef::new(Mesh::<Self> {
            vertices,
            indices,
            vertex_buffer: upload_result.vertex_buffer,
            index_buffer: upload_result.index_buffer,
        }))
    }

    pub fn load_model_from_path_ply(
        path: &std::path::Path,
        ctx: &mut Context,
    ) -> Result<ThreadSafeRef<Mesh<Self>>, SimpleVertexMeshLoadingError> {
        let file = std::fs::File::open(path)?;
        let mut file = std::io::BufReader::new(file);

        let vertex_parser = parser::Parser::<Self>::new();
        let face_parser = parser::Parser::<Face>::new();

        let header = vertex_parser.read_header(&mut file)?;

        let mut vertices = vec![];
        let mut faces = vec![];
        for (_, element) in &header.elements {
            #[allow(clippy::single_match)]
            match element.name.as_ref() {
                "vertex" => {
                    vertices =
                        vertex_parser.read_payload_for_element(&mut file, element, &header)?;
                }
                "face" => {
                    faces = face_parser.read_payload_for_element(&mut file, element, &header)?;
                }
                _ => (),
            }
        }

        let mut indices = Vec::with_capacity(faces.len() * 3);
        for face in faces {
            indices.extend(face.indices.iter());
        }

        let upload_result = upload_mesh_data(&vertices, &indices, ctx)?;

        Ok(ThreadSafeRef::new(Mesh::<Self> {
            vertices,
            indices,
            vertex_buffer: upload_result.vertex_buffer,
            index_buffer: upload_result.index_buffer,
        }))
    }
}
