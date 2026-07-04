use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};

use crate::prelude::*;

pub const FRAMES_IN_FLIGHT: u32 = 2;

pub struct WGpuGraphicsSystem {
    queue: wgpu::Queue,
    device: wgpu::Device,
    adaptor: wgpu::Adapter,
    instance: wgpu::Instance,
}

impl WGpuGraphicsSystem {
    pub fn init(
        window: sdl3::video::Window,
    ) -> anyhow::Result<(Self, WGpuPresentationSubSystem)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: wgpu::InstanceFlags::VALIDATION,
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds {
                for_resource_creation: None,
                for_device_loss: None,
            },
            backend_options: wgpu::BackendOptions::from_env_or_default(),
            display: None,
        });

        let raw_window_handle = window.window_handle()
            .expect("SDL3 window should return a valid window handle")
            .as_raw();

        let raw_display_handle = window.display_handle()
            .expect("SDL3 window should return a valid display handle")
            .as_raw();

        let surface = unsafe {
            instance.create_surface_unsafe(
                wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: Some(raw_display_handle),
                    raw_window_handle,
                }
            )
        }
            .expect("WGpu should suceed when creating the surface for desired targets");

        use pollster::FutureExt;

        let adaptor = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            },
        )
            .block_on()
            .expect("WGpu should find a valid adaptor for desired targets.");

        let (device, queue) = adaptor.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Main WGpu device"),
                required_features: wgpu::Features {
                    features_wgpu: wgpu::FeaturesWGPU::POLYGON_MODE_LINE,
                    features_webgpu: wgpu::FeaturesWebGPU::empty(),
                },
                required_limits: wgpu::Limits::defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            },
        )
            .block_on()
            .expect("we should find a valid device");

        let surface_capabilities = surface.get_capabilities(&adaptor);
        let surface_format = surface_capabilities.formats.iter()
            .find(|&format| format.is_srgb())
            .copied()
            .expect("Created WGpu surface should support the SRgb format");

        let (width, height) = window.size_in_pixels();

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: FRAMES_IN_FLIGHT,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };

        surface.configure(&device, &surface_config);

        Ok((
            Self {
                queue: queue.clone(),
                device: device.clone(),
                adaptor,
                instance: instance.clone(),
            },
            WGpuPresentationSubSystem {
                surface_config,
                queue,
                device,
                surface,
                instance,
                window,
            },
        ))
    }
    pub fn shader_module(&self, code: &str) -> wgpu::ShaderModule {
        let source = wgpu::ShaderSource::Wgsl(code.into());

        self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source,
        })
    }
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

impl super::GraphicsSystemImpl for WGpuGraphicsSystem {
}

pub struct WGpuPresentationSubSystem {
    surface_config: wgpu::SurfaceConfiguration,
    queue: wgpu::Queue,
    device: wgpu::Device,
    // lifetime guaranteed by drop order, window outlives surface
    surface: wgpu::Surface<'static>,
    instance: wgpu::Instance,
    window: sdl3::video::Window,
}

impl WGpuPresentationSubSystem {
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }
}

impl super::PresentationSubSystemImpl for WGpuPresentationSubSystem {
    fn window(&self) -> &sdl3::video::Window {
        &self.window
    }
    fn window_size(&self) -> (u32, u32) {
        self.window.size_in_pixels()
    }
    fn resize_surface(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            log_release!(LogType::Info, "Resizing presentation surface, new size (x: {}, y: {}).", width, height);
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(
                &self.device,
                &self.surface_config,
            );
        }
    }
    fn present<F: FnMut(&mut wgpu::RenderPass)>(&mut self, mut f: F) -> anyhow::Result<()> {
        use wgpu::CurrentSurfaceTexture;
        let surface_texture = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                log_debug!(LogType::Info, "suboptimal");
                let (width, height) = self.window.size_in_pixels();
                self.resize_surface(width, height);
                surface_texture
            },
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => return Ok(()),
            CurrentSurfaceTexture::Outdated => {
                log_debug!(LogType::Info, "outdated");
                let (width, height) = self.window.size_in_pixels();
                self.resize_surface(width, height);
                return Ok(());
            },
            CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Validation => panic!("WGpu surface lost or validation error"),
        };

        // TODO: seams bad
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut presentation_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Presentation encoder"),
        });


        let mut render_pass = presentation_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })
            ],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        f(&mut render_pass);

        drop(render_pass);

        self.queue.submit([presentation_encoder.finish()]);

        surface_texture.present();

        Ok(())
    }
}
