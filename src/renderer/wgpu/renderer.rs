use crate::{platform::{WindowHandle, WindowInfo}, renderer::wgpu::{material::MaterialManager, mesh::MeshManager, shader::ShaderManager, texture::TextureManager}, rendering::{DrawCommand, MaterialDefinition, MaterialHandle, MeshDefinition, MeshHandle, RenderPhase, RendererBackend, ShaderDefinition, ShaderHandle, TextureDefinition, TextureHandle}};

pub struct Renderer {
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    materials: MaterialManager,
    meshes: MeshManager,
    queue: wgpu::Queue,
    shaders: ShaderManager,
    surface: wgpu::Surface<'static>,
    textures: TextureManager,
}

impl Renderer {
    pub async fn new(window: &WindowHandle, info: WindowInfo) -> Self {
        let instance = wgpu::Instance::default();
        let target = wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: window.display_handle,
            raw_window_handle: window.window_handle,
        };

        // SAFETY: The display and window handles come from a live Winit window
        // and are guaranteed to outlive the surface.
        let surface = unsafe {
          instance.create_surface_unsafe(target)
              .expect("failed to create surface")
        };

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
            queue,
            shaders: ShaderManager::new(),
            surface,
            textures: TextureManager::new(),
        }
    }
}

impl RendererBackend for Renderer {
    fn create_material(&mut self, def: &MaterialDefinition) -> MaterialHandle {
        self.materials.create_material(&self.device, &self.shaders, &self.textures, def)
    }

    fn create_mesh(&mut self, def: &MeshDefinition) -> MeshHandle {
        self.meshes.create_mesh(&self.device, def)
    }

    fn create_shader(&mut self, def: &ShaderDefinition) -> ShaderHandle {
        self.shaders.create_shader(&self.device, def)
    }

    fn create_texture(&mut self, def: &TextureDefinition) -> TextureHandle {
        self.textures.create_texture(&self.device, &self.queue, def)
    }
    
    fn execute_commands(&mut self, phase: RenderPhase, cmds: &[DrawCommand]) {
        println!("Executing {:?} commands for {:?}", cmds.len(), phase);
    }
    
    fn present(&mut self) {
        println!("Presenting frame");
    }
}
