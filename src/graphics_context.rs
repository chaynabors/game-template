use std::sync::Arc;

use anyhow::bail;

use wgpu::{
    Adapter, Backends, CommandBuffer, CompositeAlphaMode, Device, DeviceDescriptor, DownlevelCapabilities, DownlevelFlags, Extent3d, Features, Gles3MinorVersion, Instance, InstanceDescriptor, InstanceFlags, Limits, PresentMode, Queue, ShaderModel, Surface, SurfaceConfiguration, SurfaceTexture, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor
};
use winit::{dpi::PhysicalSize, window::Window};

pub struct GraphicsContext<'a> {
    _instance: Instance,
    surface: Surface<'a>,
    _adapter: Adapter,
    pub surface_config: SurfaceConfiguration,
    pub device: Device,
    queue: Queue,
    depth_texture: Texture,
    pub depth_texture_view: TextureView,
}

impl<'a> GraphicsContext<'a> {
    pub fn new(window: Arc<Window>, physical_size: PhysicalSize<u32>) -> anyhow::Result<Self> {
        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

        let instance_flags = InstanceFlags::from_build_config();

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::PRIMARY,
            dx12_shader_compiler,
            flags: instance_flags,
            gles_minor_version: Gles3MinorVersion::Automatic,
        });

        let surface = instance.create_surface(window)?;

        let adapter = match futures::executor::block_on(
            wgpu::util::initialize_adapter_from_env_or_default(&instance, Some(&surface)),
        ) {
            Some(adapter) => adapter,
            None => bail!("Unable to find a suitable display adapter"),
        };

        let capabilities = surface.get_capabilities(&adapter);
        let surface_format = match capabilities
            .formats
            .contains(&TextureFormat::Bgra8UnormSrgb)
        {
            true => TextureFormat::Bgra8UnormSrgb,
            false => bail!("The adapter has no supported surface formats"),
        };

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST,
            format: surface_format,
            width: physical_size.width,
            height: physical_size.height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 5,
        };

        let required_features = Features::PUSH_CONSTANTS
            | Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | Features::CLEAR_TEXTURE
            | Features::CONSERVATIVE_RASTERIZATION;

        let adapter_features = adapter.features();

        if !adapter_features.contains(required_features) {
            bail!("The adapter doesn't contain the required features");
        }

        let required_downlevel_capabilities = DownlevelCapabilities {
            flags: DownlevelFlags::empty(),
            shader_model: ShaderModel::Sm5,
            ..DownlevelCapabilities::default()
        };
        let downlevel_capabilities = adapter.get_downlevel_capabilities();

        if downlevel_capabilities.shader_model < required_downlevel_capabilities.shader_model {
            bail!("The adapter doesn't support the required shader model");
        }

        if !downlevel_capabilities
            .flags
            .contains(required_downlevel_capabilities.flags)
        {
            bail!("The adapter doesn't support the required downlevel capabilties");
        }

        let mut required_limits = Limits::downlevel_defaults();
        required_limits.max_push_constant_size = 128;
        required_limits = required_limits.using_resolution(adapter.limits());

        let (device, queue) = match futures::executor::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                required_features,
                required_limits,
            },
            None,
        )) {
            Ok(adapter) => adapter,
            Err(_) => bail!("Failed to request the graphics device"),
        };

        surface.configure(&device, &surface_config);

        let (depth_texture, depth_texture_view) = create_depth_texture(&device, physical_size);

        Ok(Self {
            _instance: instance,
            surface,
            _adapter: adapter,
            surface_config,
            device,
            queue,
            depth_texture: depth_texture,
            depth_texture_view,
        })
    }

    pub fn resize(&mut self, physical_size: PhysicalSize<u32>) {
        self.surface_config.width = physical_size.width;
        self.surface_config.height = physical_size.height;

        if self.surface_config.width * self.surface_config.height != 0 {
            self.surface.configure(&self.device, &self.surface_config);
            (self.depth_texture, self.depth_texture_view) = create_depth_texture(&self.device, physical_size);
        }
    }

    /// Retrieves the next frame from the surface.
    ///
    /// If the surface has zero area, this will return `None`
    ///
    /// If the surface is lost, it will be recreated.
    ///
    /// If the surface is lost and recreation fails, this function will panic.
    pub fn get_frame(&self) -> Option<(SurfaceTexture, TextureView)> {
        if self.surface_config.width * self.surface_config.height == 0 {
            return None;
        }

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                self.surface.configure(&self.device, &self.surface_config);
                self.surface
                    .get_current_texture()
                    .expect("surface failed to reconstruct")
            }
        };

        let frame_view = frame.texture.create_view(&TextureViewDescriptor::default());

        Some((frame, frame_view))
    }

    pub fn submit<I: IntoIterator<Item = CommandBuffer>>(&self, command_buffers: I) {
        self.queue.submit(command_buffers);
    }
}

fn create_depth_texture(device: &Device, physical_size: PhysicalSize<u32>) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor { label: Some("depth"), size: Extent3d {
        width: physical_size.width,
        height: physical_size.height,
        depth_or_array_layers: 1,
    }, mip_level_count: 1, sample_count: 1, dimension: TextureDimension::D2, format: TextureFormat::Depth32Float, usage: TextureUsages::RENDER_ATTACHMENT, view_formats: &[TextureFormat::Depth32Float]});

    let view = texture.create_view(&TextureViewDescriptor::default());

    (texture, view)
}
