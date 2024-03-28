use anyhow::Result;
use gltf::mesh::{util::ReadIndices, Mode};
use tracing::warn;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages,
};

use crate::graphics_context::GraphicsContext;

pub struct Mesh {
    pub positions: Buffer,
    pub colors: Buffer,
    pub indices: Buffer,
    pub index_count: u32,
}

impl Mesh {
    pub fn load(asset: &[u8], gfx: &mut GraphicsContext) -> Result<Self> {
        let (document, buffers, _images) = gltf::import_slice(asset)?;

        let mut positions = vec![];
        let mut indices = vec![];
        let mut colors = vec![];
        let mut position_offset ;
        for scene in document.scenes() {
            for node in scene.nodes() {
                position_offset = positions.len();

                let Some(mesh) = node.mesh() else {
                    continue;
                };

                for primitive in mesh.primitives() {
                    if primitive.mode() != Mode::Triangles {
                        warn!("encountered non-triangle geometry during geometry import");
                        continue;
                    }

                    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                    let Some(pos_iter) = reader.read_positions() else {
                        warn!("encountered geometry with no position attribute during geometry import");
                        continue;
                    };

                    let Some(col_iter) = reader.read_colors(0) else {
                        warn!(
                            "encountered geometry with no color attribute 0 during geometry import"
                        );
                        continue;
                    };

                    let Some(ReadIndices::U16(indices_iter)) = reader.read_indices() else {
                        warn!(
                            "encountered geometry with an unsupported index type during geometry import"
                        );
                        continue;
                    };

                    positions.extend(pos_iter);
                    colors.extend(col_iter.into_rgb_f32());
                    indices.extend(indices_iter.map(|i| i + position_offset as u16));
                }
            }
        }

        Ok(Self {
            positions: gfx.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&positions),
                usage: BufferUsages::VERTEX,
            }),
            colors: gfx.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&colors),
                usage: BufferUsages::VERTEX,
            }),
            indices: gfx.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&indices),
                usage: BufferUsages::INDEX,
            }),
            index_count: indices.len().try_into().unwrap(),
        })
    }
}
