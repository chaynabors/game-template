use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::{
    vertex_attr_array, BlendState, BufferAddress, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, FragmentState, FrontFace, MultisampleState, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, PushConstantRange, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StencilState, TextureFormat, VertexBufferLayout, VertexState, VertexStepMode
};

use super::graphics_context::GraphicsContext;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct PushConstants {
    pub mvp: Mat4,
}

pub fn model_pipeline(ctx: &GraphicsContext) -> RenderPipeline {
    let shader_module = ctx
        .device
        .create_shader_module(wgpu::include_wgsl!("assets/mesh.wgsl"));

    let pipeline_layout = ctx
        .device
        .create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("model_renderer_pipeline_layout_descriptor"),
            bind_group_layouts: &[],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..u32::try_from(std::mem::size_of::<PushConstants>()).unwrap(),
            }],
        });

    ctx.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("model_renderer_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &vertex_attr_array![0 => Float32x3],
                    },
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &vertex_attr_array![1 => Float32x3],
                    },
                ],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: ctx.surface_config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::all(),
                })],
            }),
            multiview: None,
        })
}
