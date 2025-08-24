use ash::vk;
use thiserror::Error;

use crate::gfx::{
    buffer::{Buffer, BufferBuildError},
    commands::ImmediateCommandError,
    context::Context,
    vertex::Vertex,
};

#[derive(Debug)]
pub struct Mesh<VertexType>
where
    VertexType: Vertex,
{
    pub name: String,

    pub vertices: Vec<VertexType>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
}

#[derive(Error, Debug)]
pub enum UploadError {
    #[error("staging buffer creation failed")]
    StagingBufferCreation(BufferBuildError),

    #[error("staging buffer memory mapping failed")]
    MemoryMapping,

    #[error("main buffer creation failed")]
    MainBufferCreation(BufferBuildError),

    #[error("memory copy failed")]
    CopyCommand(ImmediateCommandError),
}

pub fn upload_vertex_buffer<VertexType>(
    name: &str,
    vertices: &[VertexType],
    ctx: &mut Context,
) -> Result<Buffer, UploadError>
where
    VertexType: Vertex,
{
    let vertex_data_size: u64 = std::mem::size_of_val(vertices).try_into().unwrap();
    let vertex_staging_buffer = Buffer::builder(vertex_data_size)
        .with_name(&format!("{} vertex staging", name))
        .with_usage(vk::BufferUsageFlags::TRANSFER_SRC)
        .with_memory_location(gpu_allocator::MemoryLocation::CpuToGpu)
        .build(ctx)
        .map_err(UploadError::StagingBufferCreation)?;

    let vertex_staging_ptr = vertex_staging_buffer
        .allocation
        .mapped_ptr()
        .ok_or(UploadError::MemoryMapping)?
        .cast::<VertexType>()
        .as_ptr();

    unsafe {
        std::ptr::copy_nonoverlapping(vertices.as_ptr(), vertex_staging_ptr, vertices.len());
    };

    let buffer_usage_flags =
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER;

    let vertex_buffer = Buffer::builder(vertex_data_size)
        .with_name(&format!("{} vertex data", name))
        .with_usage(buffer_usage_flags)
        .with_memory_location(gpu_allocator::MemoryLocation::GpuOnly)
        .build(ctx)
        .map_err(UploadError::MainBufferCreation)?;

    ctx.command_manager
        .immediate_command(|cmd_buffer| {
            let copy_info = vk::BufferCopy::default().size(vertex_data_size);

            unsafe {
                ctx.device_ref.read().cmd_copy_buffer(
                    *cmd_buffer,
                    vertex_staging_buffer.handle,
                    vertex_buffer.handle,
                    std::slice::from_ref(&copy_info),
                );
            }
        })
        .map_err(UploadError::CopyCommand)?;

    Ok(vertex_buffer)
}

pub fn upload_index_buffer(
    name: &str,
    indices: &[u32],
    ctx: &mut Context,
) -> Result<Buffer, UploadError> {
    let index_data_size: u64 = std::mem::size_of_val(indices).try_into().unwrap();
    let mut index_staging_buffer = Buffer::builder(index_data_size)
        .with_name(&format!("{} index staging", name))
        .with_usage(vk::BufferUsageFlags::TRANSFER_SRC)
        .with_memory_location(gpu_allocator::MemoryLocation::CpuToGpu)
        .build(ctx)
        .map_err(UploadError::StagingBufferCreation)?;

    let raw_indices =
        bytemuck::try_cast_slice(indices).expect("casting from u32 to u8 should always (?) work");
    index_staging_buffer
        .allocation
        .mapped_slice_mut()
        .ok_or(UploadError::MemoryMapping)?[..raw_indices.len()]
        .copy_from_slice(raw_indices);

    let buffer_usage_flags =
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER;

    let index_buffer = Buffer::builder(index_data_size)
        .with_name(&format!("{} index data", name))
        .with_usage(buffer_usage_flags)
        .with_memory_location(gpu_allocator::MemoryLocation::GpuOnly)
        .build(ctx)
        .map_err(UploadError::MainBufferCreation)?;

    ctx.command_manager
        .immediate_command(|cmd_buffer| {
            let copy_info = vk::BufferCopy::default().size(index_data_size);

            unsafe {
                ctx.device_ref.read().cmd_copy_buffer(
                    *cmd_buffer,
                    index_staging_buffer.handle,
                    index_buffer.handle,
                    std::slice::from_ref(&copy_info),
                );
            }
        })
        .map_err(UploadError::CopyCommand)?;

    Ok(index_buffer)
}

pub struct UploadData {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
}

#[derive(Error, Debug)]
pub enum MeshDataUploadError {
    #[error("upload of vertex data failed")]
    VertexBufferUpload(UploadError),

    #[error("upload of index data failed")]
    IndexBufferUpload(UploadError),
}

pub fn upload_mesh_data<VertexType>(
    name: &str,
    vertices: &[VertexType],
    indices: &[u32],
    ctx: &mut Context,
) -> Result<UploadData, MeshDataUploadError>
where
    VertexType: Vertex,
{
    let vertex_buffer = upload_vertex_buffer(name, vertices, ctx)
        .map_err(MeshDataUploadError::VertexBufferUpload)?;
    let index_buffer =
        upload_index_buffer(name, indices, ctx).map_err(MeshDataUploadError::IndexBufferUpload)?;

    Ok(UploadData {
        vertex_buffer,
        index_buffer,
    })
}
