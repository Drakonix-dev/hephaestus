use std::sync::Arc;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{platform::core::{FrameError, HasWindowInfo, PlatformError}, renderer::{self, backend::wgpu::{material::MaterialManager, mesh::MeshManager, pipeline::PipelineManager, shader::ShaderManager, texture::TextureManager}, DrawCommand, DrawMesh, MaterialDefinition, MaterialHandle, MeshDefinition, MeshHandle, RenderPhase, RendererBackend, ShaderDefinition, ShaderHandle, TextureDefinition, TextureHandle}};

trait Window: HasWindowHandle + HasDisplayHandle + HasWindowInfo + Send + Sync + 'static {}
impl <T: HasWindowHandle + HasDisplayHandle + HasWindowInfo + Send + Sync + 'static> Window for T {}

pub struct Renderer<W: Window> {
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
}

impl<W: Window> Renderer<W> {
    pub(crate) async fn new(window: W) -> Result<Self, PlatformError> {
        let window = Arc::new(window);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        
        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
               power_preference: wgpu::PowerPreference::default(),
               compatible_surface: Some(&surface),
               force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        Ok(Self {
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
        })
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

    fn draw_mesh(&mut self, rpass: &mut wgpu::RenderPass, draw: &DrawMesh) -> Result<(), PlatformError> {
        let mesh = self.meshes.get_mesh(&draw.mesh)
            .ok_or(PlatformError::AssetNotFound(format!("{:?} not found", draw.mesh)))?;
        let material = self.materials.get_material(&draw.material)
            .ok_or(PlatformError::AssetNotFound(format!("{:?} not found", draw.material)))?;
        let shader = self.shaders.get_shader(&material.shader)
            .ok_or(PlatformError::AssetNotFound(format!("{:?} not found", material.shader)))?;
        let pipeline = self.pipelines.get_or_create_pipeline(
            self.surface_format.ok_or(
                PlatformError::Frame(FrameError::Other(String::from("surface format not found"))))?,
            &self.device, &shader);

        rpass.set_pipeline(&pipeline.pipeline);
        rpass.set_bind_group(0, &material.bind_group, &[]);
        rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        if mesh.index_count > 0 {
            rpass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }

        Ok(())
    }
}

impl<'a, W: Window> RendererBackend<'a, RenderFrame<'a>> for Renderer<W> {
    fn begin_frame(&'a mut self) -> Result<RenderFrame<'a>, PlatformError> {
        if self.config.is_none() {
            self.configure_surface();
        }

        self.window.request_redraw();

        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(RenderFrame::new(RenderFrameDescriptor {
            device: &self.device,
            draw_mesh: self.draw_mesh,
            frame,
            queue: &self.queue,
            view,
        }))
    }
    
    fn create_material(&mut self, handle: MaterialHandle, definition: MaterialDefinition) -> Result<(), PlatformError> {
        self.materials.create_material(&self.device, &self.shaders, &self.textures, handle, &definition)
    }
    
    fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition) {
        self.meshes.create_mesh(&self.device, handle, &definition);
    }
    
    fn create_shader(&mut self, handle: ShaderHandle, definition: ShaderDefinition) -> Result<(), PlatformError> {
        self.shaders.create_shader(&self.device, handle, &definition)
    }
    
    fn create_texture(&mut self, handle: TextureHandle, definition: TextureDefinition) -> Result<(), PlatformError> {
        self.textures.create_texture(&self.device, &self.queue, handle, &definition)
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
        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
    }
}

struct RenderFrame<'a, F>
    where F: FnMut(&mut wgpu::RenderPass, &DrawMesh) -> Result<(), PlatformError>
{
    device: &'a wgpu::Device,
    draw_mesh: F,
    first_pass: bool,
    frame: wgpu::SurfaceTexture,
    queue: &'a wgpu::Queue,
    view: wgpu::TextureView,
}

struct RenderFrameDescriptor<'a, F>
    where F: FnMut(&mut wgpu::RenderPass, &DrawMesh) -> Result<(), PlatformError>
{
    device: &'a wgpu::Device,
    draw_mesh: F,
    frame: wgpu::SurfaceTexture,
    queue: &'a wgpu::Queue,
    view: wgpu::TextureView,
}

impl<'a, F> RenderFrame<'a, F>
    where F: FnMut(&mut wgpu::RenderPass, &DrawMesh) -> Result<(), PlatformError>
{
    fn new(desc: RenderFrameDescriptor<'a, F>) -> Self {
        Self {
            device: desc.device,
            draw_mesh: desc.draw_mesh,
            first_pass: true,
            frame: desc.frame,
            queue: desc.queue,
            view: desc.view,
        }
    }
}

impl<'a, F> renderer::RenderFrame for RenderFrame<'a, F>
    where F: FnMut(&mut wgpu::RenderPass, &DrawMesh) -> Result<(), PlatformError>
{
    fn execute_commands(&mut self, phase: &RenderPhase, cmds: &[DrawCommand]) -> Result<(), PlatformError> {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some(&format!("Render {:?} Encode", phase)),
        });

        let load_op = if self.first_pass {
            self.first_pass = false;
            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
        } else {
            wgpu::LoadOp::Load
        };

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
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
                    DrawCommand::Mesh(mesh) => (self.draw_mesh)(&mut rpass, mesh)?,
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }
    
    fn present_frame(self) {
        self.frame.present();
    }
}

impl From<wgpu::CreateSurfaceError> for PlatformError {
    fn from(err: wgpu::CreateSurfaceError) -> Self {
        PlatformError::RendererCreationFailed(err.to_string())
    }
}

impl From<wgpu::RequestAdapterError> for PlatformError {
    fn from(err: wgpu::RequestAdapterError) -> Self {
        PlatformError::RendererCreationFailed(err.to_string())
    }
}

impl From<wgpu::RequestDeviceError> for PlatformError {
    fn from(err: wgpu::RequestDeviceError) -> Self {
        PlatformError::RendererCreationFailed(err.to_string())
    }
}

impl From<wgpu::SurfaceError> for PlatformError {
    fn from(err: wgpu::SurfaceError) -> Self {
        match err {
            wgpu::SurfaceError::Lost => PlatformError::Frame(FrameError::SwapchainLost),
            wgpu::SurfaceError::OutOfMemory => PlatformError::Frame(FrameError::OutOfMemory),
            wgpu::SurfaceError::Outdated => PlatformError::Frame(FrameError::OutdatedSurface),
            wgpu::SurfaceError::Other => PlatformError::Frame(FrameError::Other(err.to_string())),
            wgpu::SurfaceError::Timeout => PlatformError::Frame(FrameError::Timeout),
        }
    }
}
