use anyhow::bail;
use wgpu::{Backends, CommandBuffer, CommandEncoder, CommandEncoderDescriptor, DeviceDescriptor, DownlevelFlags, Gles3MinorVersion, InstanceDescriptor, InstanceFlags, ShaderModel};

use wgpu::{
    Adapter, CompositeAlphaMode, Device, DownlevelCapabilities, Features, Instance, Limits,
    PresentMode, Queue, Surface, SurfaceConfiguration, SurfaceTexture, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

pub struct GraphicsContext<'a> {
    _instance: Instance,
    surface: Surface<'a>,
    _adapter: Adapter,
    surface_config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
}

impl<'a> GraphicsContext<'a> {
    pub fn new<W>(window: W, window_width: u32, window_height: u32) -> anyhow::Result<Self>
    where
        W: raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle + Send + Sync + 'a,
    {
        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

        let instance_flags = match cfg!(debug_assertions) {
            true => InstanceFlags::DEBUG | InstanceFlags::VALIDATION,
            false => InstanceFlags::DISCARD_HAL_LABELS,
        };

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
            width: window_width,
            height: window_height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let required_features = Features::PUSH_CONSTANTS
            | Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | Features::CLEAR_TEXTURE
            | Features::CONSERVATIVE_RASTERIZATION
            | Features::POLYGON_MODE_LINE;

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

        Ok(Self {
            _instance: instance,
            surface,
            _adapter: adapter,
            surface_config,
            device,
            queue,
        })
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        self.surface_config.width = new_width;
        self.surface_config.height = new_height;

        if self.surface_config.width * self.surface_config.height != 0 {
            self.surface.configure(&self.device, &self.surface_config);
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

    pub fn aspect_ratio(&self) -> f32 {
        self.surface_config.width as f32 / self.surface_config.height as f32
    }

    pub fn create_command_encoder(&self, label: &'static str) -> CommandEncoder {
        self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some(label),
            })
    }

    pub fn submit<I: IntoIterator<Item = CommandBuffer>>(&self, command_buffers: I) {
        self.queue.submit(command_buffers);
    }
}
