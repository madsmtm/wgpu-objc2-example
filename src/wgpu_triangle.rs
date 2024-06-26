//! Adapted from `wgpu/examples/src/hello_triangle`.
use std::{
    cell::{Cell, RefCell},
    time::{Duration, Instant},
};

#[allow(unused)] // Unsure which of these need to be kept around!
#[derive(Debug)]
pub struct Triangle<'window> {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'window>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,
    config: RefCell<wgpu::SurfaceConfiguration>,
    frame_counter: FrameCounter,
}

impl<'window> Triangle<'window> {
    pub async fn new(
        target: impl Into<wgpu::SurfaceTarget<'window>>,
        width: u32,
        height: u32,
    ) -> Self {
        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(target).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        // Load the shaders from disk
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            #[cfg(feature = "wgpu-unstable")]
            cache: None,
        });

        let config = surface.get_default_config(&adapter, width, height).unwrap();
        surface.configure(&device, &config);

        Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            shader,
            pipeline_layout,
            render_pipeline,
            config: RefCell::new(config),
            frame_counter: FrameCounter::new(),
        }
    }

    pub fn resize(&self, width: u32, height: u32) {
        let mut config = self.config.borrow_mut();
        config.width = width;
        config.height = height;
        self.surface.configure(&self.device, &config);
    }

    pub fn redraw(&self) {
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        self.frame_counter.update();
    }
}

#[derive(Debug)]
struct FrameCounter {
    last_printed_instant: Cell<Instant>,
    frame_count: Cell<u32>,
}

impl FrameCounter {
    fn new() -> Self {
        Self {
            last_printed_instant: Cell::new(Instant::now()),
            frame_count: Cell::new(0),
        }
    }

    fn update(&self) {
        self.frame_count.set(self.frame_count.get() + 1);

        let now = Instant::now();
        let elapsed = now - self.last_printed_instant.get();
        if elapsed > Duration::from_secs(1) {
            let fps = self.frame_count.get() as f32 / elapsed.as_secs_f32();
            eprintln!("FPS: {:.1}", fps);

            self.last_printed_instant.set(now);
            self.frame_count.set(0);
        }
    }
}
