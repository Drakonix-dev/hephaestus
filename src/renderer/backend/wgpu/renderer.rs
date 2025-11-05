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
    pub async fn new<T>(window: &'a T, info: WindowInfo) -> Self
        where T: HasWindowHandle + HasDisplayHandle + Send + Sync
    {
        let instance = wgpu::Instance::default();
        let target = wgpu::SurfaceTarget::Window(Box::new(window));
        
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
        
    }
    
    fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition) {
        
    }
    
    fn create_shader(&mut self, handle: ShaderHandle, definition: ShaderDefinition) {
        
    }
    
    fn create_texture(&mut self, handle: TextureHandle, definition: TextureDefinition) {
        
    }
    
    fn execute_commands(&mut self, phase: RenderPhase, cmds: &[DrawCommand]) {
        
    }
    
    fn present_frame(&mut self) {
        
    }
}
