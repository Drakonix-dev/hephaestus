use std::sync::Arc;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{platform::core::{HasWindowInfo, PlatformError}, renderer::{backend::wgpu::{material::MaterialManager, mesh::MeshManager, pipeline::PipelineManager, shader::ShaderManager, texture::TextureManager}, DrawCommand, DrawMesh, MaterialDefinition, MaterialHandle, MeshDefinition, MeshHandle, RenderPhase, RendererBackend, ShaderDefinition, ShaderHandle, TextureDefinition, TextureHandle}};

pub struct Renderer<W>
    where W: HasWindowHandle + HasDisplayHandle + HasWindowInfo + Send + Sync + 'static
{
    adapter: wgpu::Adapter,
    config: Option<wgpu::SurfaceConfiguration>,
    device: wgpu::Device,
    materials: MaterialManager,
    meshes: MeshManager,
    pipelines: PipelineManager,
    queue: wgpu::Queue,
    shaders: ShaderManager,
    surface: wgpu::Surface<'static>,
    surface_format: Option<wgpu::TextureFormat>,
    textures: TextureManager,
    window: Arc<W>,

    current_frame: Option<RenderFrame>,
}

impl<W> Renderer<W>
    where W: HasWindowHandle + HasDisplayHandle + HasWindowInfo + Send + Sync + 'static
{
    pub(crate) async fn new(window: W) -> Self {
        let window = Arc::new(window);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
               power_preference: wgpu::PowerPreference::default(),
               compatible_surface: Some(&surface),
               force_fallback_adapter: false,
            })
            .await
            .expect("failed to get adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("failed to get device");

        Self {
            adapter,
            config: None,
            device,
            materials: MaterialManager::new(),
            meshes: MeshManager::new(),
            pipelines: PipelineManager::new(),
            queue,
            shaders: ShaderManager::new(),
            surface,
            surface_format: None,
            textures: TextureManager::new(),
            window,

            current_frame: None,
        }
    }

    pub(crate) fn configure_surface(&mut self) {
        let window_info = self.window.get_window_info();
        
        let caps = self.surface.get_capabilities(&self.adapter);
        let format = caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);
        let present_mode = caps.present_modes
            .first()
            .copied()
            .unwrap_or(wgpu::PresentMode::Fifo);
        let alpha_mode = caps.alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Opaque);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: format,
            width: window_info.width.max(1),
            height: window_info.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        self.surface.configure(&self.device, &config);
        self.surface_format = Some(format);
        self.config = Some(config);
    }

    fn draw_mesh(&mut self, rpass: &mut wgpu::RenderPass, draw: &DrawMesh) {
        let mesh = self.meshes.get_mesh(&draw.mesh).unwrap();
        let material = self.materials.get_material(&draw.material).unwrap();
        let shader = self.shaders.get_shader(&material.shader).unwrap();
        let pipeline = self.pipelines.get_or_create_pipeline(
            self.surface_format.unwrap(), &self.device, &shader);

        rpass.set_pipeline(&pipeline.pipeline);
        rpass.set_bind_group(0, &material.bind_group, &[]);
        rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        if mesh.index_count > 0 {
            rpass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }
}

impl<W> RendererBackend for Renderer<W>
    where W: HasWindowHandle + HasDisplayHandle + HasWindowInfo + Send + Sync + 'static
{
    fn create_material(&mut self, handle: MaterialHandle, definition: MaterialDefinition) -> Result<(), PlatformError> {
        self.materials.create_material(&self.device, &self.shaders, &self.textures, handle, &definition)
    }
    
    fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition) {
        self.meshes.create_mesh(&self.device, handle, &definition);
    }
    
    fn create_shader(&mut self, handle: ShaderHandle, definition: ShaderDefinition) {
        self.shaders.create_shader(&self.device, handle, &definition);
    }
    
    fn create_texture(&mut self, handle: TextureHandle, definition: TextureDefinition) -> Result<(), PlatformError> {
        self.textures.create_texture(&self.device, &self.queue, handle, &definition)
    }
    
    fn execute_commands(&mut self, phase: &RenderPhase, cmds: &[DrawCommand]) {
        if self.config.is_none() {
            self.configure_surface();
        }

        let frame = self.current_frame.get_or_insert_with(|| {
            self.window.request_redraw();
            
            let frame = self.surface.get_current_texture()
                .expect("failed to acquire frame");
            let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

            RenderFrame {
                frame,
                view,
                first_pass: true,
            }
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some(&format!("Render {:?} Encode", phase)),
        });

        let load_op = if frame.first_pass {
            frame.first_pass = false;
            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
        } else {
            wgpu::LoadOp::Load
        };

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for cmd in cmds {
                match cmd {
                    DrawCommand::Mesh(mesh) => self.draw_mesh(&mut rpass, mesh),
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
    }
    
    fn present_frame(&mut self) {
        if let Some(frame) = self.current_frame.take() {
            frame.frame.present();
        }
    }
    
    fn resize(&mut self, width: u32, height: u32) {
        if self.config.is_none() {
            self.configure_surface();
            return
        }

        self.config.as_mut().unwrap().width = width.max(1);
        self.config.as_mut().unwrap().height = height.max(1);
        self.surface.configure(&self.device, self.config.as_ref().unwrap());
    }

    fn shutdown(&mut self) {
        self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
    }
}

struct RenderFrame {
    first_pass: bool,
    frame: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
}
