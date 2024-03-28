use std::{
    sync::{
        mpsc::{self, SyncSender, TryRecvError},
        Arc,
    },
    thread::JoinHandle,
};

use anyhow::anyhow;
use glam::Vec3;
use vulkano::{
    buffer::BufferContents,
    command_buffer::allocator::StandardCommandBufferAllocator,
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
    },
    image::{view::ImageView, Image, ImageUsage},
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo, InstanceExtensions},
    memory::allocator::StandardMemoryAllocator,
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    swapchain::{CompositeAlpha, Surface, Swapchain, SwapchainCreateInfo},
    Version, VulkanLibrary,
};
use winit::{dpi::PhysicalSize, event_loop::EventLoop, window::Window};

const EVENT_BUFFER_SIZE: usize = 16;

enum GraphicsCommand {
    LoadModel(usize, &'static [u8]),
    LoadTexture(usize, &'static [u8]),
    ResizeSwapchain(PhysicalSize<u32>),
}

#[derive(Debug)]
pub struct Model(usize);

#[derive(Debug)]
pub struct Texture(usize);

pub struct GraphicsSubsystem {
    model_index: usize,
    texture_index: usize,
    render_thread: JoinHandle<anyhow::Result<()>>,
    tx: SyncSender<GraphicsCommand>,
}

impl GraphicsSubsystem {
    pub fn new(event_loop: &EventLoop<()>, window: Arc<Window>) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::sync_channel(EVENT_BUFFER_SIZE);

        let instance_extensions = Surface::required_extensions(event_loop)?;
        let mut ctx = RenderContext::new(instance_extensions, window.clone())?;

        let render_thread = std::thread::spawn(move || loop {
            match rx.try_recv() {
                Ok(command) => match command {
                    GraphicsCommand::LoadModel(index, model) => todo!(),
                    GraphicsCommand::LoadTexture(index, texture) => todo!(),
                    GraphicsCommand::ResizeSwapchain(new_size) => ctx.recreate_swapchain(new_size)?,
                },
                Err(TryRecvError::Disconnected) => return Ok(()),
                Err(TryRecvError::Empty) => (),
            }

            window.pre_present_notify();
            // window.request_redraw();
        });

        Ok(Self {
            model_index: 0,
            texture_index: 0,
            render_thread,
            tx,
        })
    }

    pub fn load_model(&mut self, model: &'static [u8]) -> anyhow::Result<Model> {
        // This skips zero which is intentional
        self.model_index += 1;
        self.tx
            .try_send(GraphicsCommand::LoadModel(self.texture_index, model))?;
        Ok(Model(self.model_index))
    }

    pub fn load_texture(&mut self, texture: &'static [u8]) -> anyhow::Result<Texture> {
        // This skips zero which is intentional
        self.texture_index += 1;
        self.tx
            .try_send(GraphicsCommand::LoadTexture(self.texture_index, texture))?;
        Ok(Texture(self.texture_index))
    }

    pub fn resize_window(&mut self, new_size: PhysicalSize<u32>) -> anyhow::Result<()> {
        Ok(self
            .tx
            .try_send(GraphicsCommand::ResizeSwapchain(new_size))?)
    }
}

struct RenderContext {
    library: Arc<VulkanLibrary>,
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    physical_device: Arc<PhysicalDevice>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    images: Vec<Arc<Image>>,
    memory_allocator: StandardMemoryAllocator,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    viewport: Viewport,
    framebuffers: Vec<Arc<Framebuffer>>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
}

impl RenderContext {
    fn new(instance_extensions: InstanceExtensions, window: Arc<Window>) -> anyhow::Result<Self> {
        let library = VulkanLibrary::new()?;

        let instance = Instance::new(
            library.clone(),
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                application_name: Some(env!("CARGO_BIN_NAME").to_owned()),
                // TODO
                application_version: Version::V1_0, // env!("CARGO_VERSION"),
                enabled_extensions: instance_extensions,
                ..Default::default()
            },
        )?;

        let swapchain_size = window.inner_size();
        let surface = Surface::from_window(instance.clone(), window)?;

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()?
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.intersects(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .ok_or(anyhow!("No suitable graphics devices"))?;

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )?;

        let queue = queues
            .next()
            .ok_or(anyhow!("No suitable graphics queue on selected device"))?;

        let (swapchain, images) = {
            let surface_caps =
                physical_device.surface_capabilities(&surface, Default::default())?;

            let image_format = physical_device
                .surface_formats(&surface, Default::default())?
                .get(0)
                .ok_or(anyhow!("No suitable surface formats on device"))?
                .0;

            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    // Some drivers report an `min_image_count` of 1, but fullscreen mode requires at
                    // least 2. Therefore we must ensure the count is at least 2, otherwise the program
                    // would crash when entering fullscreen mode on those drivers.
                    min_image_count: surface_caps.min_image_count.max(2),
                    image_format,
                    image_extent: swapchain_size.into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha: CompositeAlpha::Opaque,
                    ..Default::default()
                },
            )?
        };

        let memory_allocator = StandardMemoryAllocator::new_default(device.clone());

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        )?;

        let pipeline = {
            let vs = vs::load(device.clone())?.entry_point("main").unwrap();

            let fs = fs::load(device.clone())?.entry_point("main").unwrap();

            let vertex_input_state =
                Vert::per_vertex().definition(&vs)?;

            let stages = [
                PipelineShaderStageCreateInfo::new(vs),
                PipelineShaderStageCreateInfo::new(fs),
            ];

            let layout = PipelineLayout::new(
                device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(device.clone())?,
            )?;

            let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

            GraphicsPipeline::new(
                device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState::default()),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.num_color_attachments(),
                        ColorBlendAttachmentState::default(),
                    )),
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(subpass.into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )?
        };

        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [0.0, 0.0],
            depth_range: 0.0..=1.0,
        };

        let framebuffers = create_framebuffers(&images, render_pass.clone(), &mut viewport)?;

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        Ok(Self {
            library,
            instance,
            surface,
            physical_device,
            device,
            queue,
            swapchain,
            images,
            memory_allocator,
            render_pass,
            pipeline,
            viewport,
            framebuffers,
            command_buffer_allocator,
        })
    }

    fn recreate_swapchain(&mut self, new_size: PhysicalSize<u32>) -> anyhow::Result<()> {
        let (new_swapchain, new_images) = self.swapchain
        .recreate(SwapchainCreateInfo {
            image_extent: new_size.into(),
            ..self.swapchain.create_info()
        })?;

        self.swapchain = new_swapchain;

        self.framebuffers = create_framebuffers(
            &new_images,
            self.render_pass.clone(),
            &mut self.viewport,
        )?;

        Ok(())
    }
}

#[derive(BufferContents, vulkano::pipeline::graphics::vertex_input::Vertex)]
#[repr(C)]
struct Vert {
    #[format(R32G32B32_SFLOAT)]
    position: Vec3,
    #[format(R32G32B32_SFLOAT)]
    color: Vec3,
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec3 color;

            void main() {
                gl_Position = vec4(position, 1.0);
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        ",
    }
}

/// This function is called once during initialization, then again whenever the window is resized.
fn create_framebuffers(
    images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> anyhow::Result<Vec<Arc<Framebuffer>>> {
    let extent = images[0].extent();
    viewport.extent = [extent[0] as f32, extent[1] as f32];

    let mut framerbuffers = vec![];
    for image in images {
        let view = ImageView::new_default(image.clone())?;
        framerbuffers.push(Framebuffer::new(
            render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![view],
                ..Default::default()
            },
        )?)
    }

    Ok(framerbuffers)
}
