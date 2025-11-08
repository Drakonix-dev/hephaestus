use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{platform::core::WindowInfo, renderer::{backend::wgpu::{material::MaterialManager, mesh::MeshManager, pipeline::PipelineManager, shader::ShaderManager, texture::TextureManager}, DrawCommand, DrawMesh, MaterialDefinition, MaterialHandle, MeshDefinition, MeshHandle, RenderPhase, RendererBackend, ShaderDefinition, ShaderHandle, TextureDefinition, TextureHandle}};

pub struct Renderer<'a> {
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    materials: MaterialManager,
    meshes: MeshManager,
    pipelines: PipelineManager,
    queue: wgpu::Queue,
    shaders: ShaderManager,
    surface: wgpu::Surface<'a>,
    surface_format: wgpu::TextureFormat,
    textures: TextureManager,

    current_frame: Option<wgpu::SurfaceTexture>,
}

impl<'a> Renderer<'a> {
    pub async fn new<T>(window: Box<T>, info: WindowInfo) -> Self
        where T: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static
    {
        let instance = wgpu::Instance::default();
        let target = wgpu::SurfaceTarget::Window(window);
        
        let surface = instance.create_surface(target)
            .expect("failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
               power_preference: wgpu::PowerPreference::default(),
               compatible_surface: Some(&surface),
               force_fallback_adapter: false,
            })
            .await
            .expect("failed to get adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("failed to get device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: info.width,
            height: info.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Self {
            config,
            device,
            materials: MaterialManager::new(),
            meshes: MeshManager::new(),
            pipelines: PipelineManager::new(),
            queue,
            shaders: ShaderManager::new(),
            surface,
            surface_format,
            textures: TextureManager::new(),

            current_frame: None,
        }
    }

    fn draw_mesh(&mut self, rpass: &mut wgpu::RenderPass, draw: &DrawMesh) {
        let mesh = self.meshes.get_mesh(&draw.mesh).unwrap();
        let material = self.materials.get_material(&draw.material).unwrap();
        let shader = self.shaders.get_shader(&material.shader).unwrap();
        let pipeline = self.pipelines.get_or_create_pipeline(self.surface_format, &self.device, &shader);

        rpass.set_pipeline(&pipeline.pipeline);
        rpass.set_bind_group(0, &material.bind_group, &[]);
        rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        rpass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}

impl<'a> RendererBackend for Renderer<'a> {
    fn create_material(&mut self, handle: MaterialHandle, definition: MaterialDefinition) {
        self.materials.create_material(&self.device, &self.shaders, &self.textures, handle, &definition);
    }
    
    fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition) {
        self.meshes.create_mesh(&self.device, handle, &definition);
    }
    
    fn create_shader(&mut self, handle: ShaderHandle, definition: ShaderDefinition) {
        self.shaders.create_shader(&self.device, handle, &definition);
    }
    
    fn create_texture(&mut self, handle: TextureHandle, definition: TextureDefinition) {
        self.textures.create_texture(&self.device, &self.queue, handle, &definition);
    }
    
    fn execute_commands(&mut self, phase: RenderPhase, cmds: &[DrawCommand]) {
        let frame = self.current_frame.get_or_insert_with(|| {
            self.surface.get_current_texture()
                .expect("Failed to acquire frame")
        });

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some(&format!("Render {:?} Encode", phase)),
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
            frame.present();
        }
    }
    
    fn resize(&mut self, width: u32, height: u32) {
        if width <= 0 || height <= 0 {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
}
