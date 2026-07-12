use std::sync::Arc;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{
    config::EngineConfig,
    math::Mat4,
    platform::core::{FrameError, HasWindowInfo, PlatformError},
    renderer::{
        self, DrawCommand, DrawMesh, MaterialDefinition, MaterialHandle, MeshDefinition,
        MeshHandle, PresentMode, RenderDomain, RenderPhase, RendererBackend, ShaderDefinition,
        ShaderHandle, TextureDefinition, TextureHandle,
        backend::wgpu::{
            globals::{FrameGlobals, ModelUniformPool},
            material::MaterialManager,
            mesh::MeshManager,
            pipeline::{DEPTH_FORMAT, PipelineManager, RenderState},
            shader::ShaderManager,
            texture::TextureManager,
        },
    },
};

fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn resolve_present_mode(
    desired: PresentMode,
    caps: &wgpu::SurfaceCapabilities,
) -> wgpu::PresentMode {
    let wanted = match desired {
        PresentMode::Vsync => wgpu::PresentMode::Fifo,
        PresentMode::Immediate => wgpu::PresentMode::Immediate,
        PresentMode::Mailbox => wgpu::PresentMode::Mailbox,
    };

    if caps.present_modes.contains(&wanted) {
        wanted
    } else {
        caps.present_modes
            .first()
            .copied()
            .unwrap_or(wgpu::PresentMode::Fifo)
    }
}

pub(crate) trait Window:
    HasWindowHandle + HasDisplayHandle + HasWindowInfo + Send + Sync + 'static
{
}
impl<T: HasWindowHandle + HasDisplayHandle + HasWindowInfo + Send + Sync + 'static> Window for T {}

pub struct Renderer<W: Window> {
    adapter: wgpu::Adapter,
    config: Option<wgpu::SurfaceConfiguration>,
    depth_view: Option<wgpu::TextureView>,
    desired_present_mode: PresentMode,
    device: wgpu::Device,
    globals: FrameGlobals,
    materials: MaterialManager,
    meshes: MeshManager,
    model_pool: ModelUniformPool,
    pipelines: PipelineManager,
    queue: wgpu::Queue,
    shaders: ShaderManager,
    surface: wgpu::Surface<'static>,
    surface_format: Option<wgpu::TextureFormat>,
    textures: TextureManager,
    window: Arc<W>,
}

impl<W: Window> Renderer<W> {
    pub(crate) async fn new(cfg: &EngineConfig, window: W) -> Result<Self, PlatformError> {
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

        let globals = FrameGlobals::new(&device);
        let model_pool = ModelUniformPool::new(&device);

        Ok(Self {
            adapter,
            config: None,
            depth_view: None,
            desired_present_mode: cfg.present_mode,
            device,
            globals,
            materials: MaterialManager::new(),
            meshes: MeshManager::new(),
            model_pool,
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
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);
        let present_mode = resolve_present_mode(self.desired_present_mode, &caps);
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Opaque);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: window_info.width.max(1),
            height: window_info.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        self.surface.configure(&self.device, &config);
        self.surface_format = Some(format);
        self.depth_view = Some(create_depth_view(&self.device, config.width, config.height));
        self.config = Some(config);
    }

    fn draw_mesh(
        &mut self,
        rpass: &mut wgpu::RenderPass,
        draw: &DrawMesh,
        model_offset: u32,
        depth_enabled: bool,
    ) -> Result<(), PlatformError> {
        let mesh = self
            .meshes
            .get_mesh(&draw.mesh)
            .ok_or(PlatformError::AssetNotFound(format!(
                "{:?} not found",
                draw.mesh
            )))?;

        let material =
            self.materials
                .get_material(&draw.material)
                .ok_or(PlatformError::AssetNotFound(format!(
                    "{:?} not found",
                    draw.material
                )))?;

        let shader =
            self.shaders
                .get_shader(&material.shader)
                .ok_or(PlatformError::AssetNotFound(format!(
                    "{:?} not found",
                    material.shader
                )))?;

        let pipeline = self.pipelines.get_or_create_pipeline(
            self.surface_format
                .ok_or(PlatformError::Frame(FrameError::Other(String::from(
                    "surface format not found",
                ))))?,
            &self.device,
            shader,
            &self.globals.layout,
            &self.model_pool.layout,
            RenderState { depth_enabled },
        );

        rpass.set_pipeline(&pipeline.pipeline);
        rpass.set_bind_group(0, &self.globals.bind_group, &[]);
        rpass.set_bind_group(1, &material.bind_group, &[]);
        rpass.set_bind_group(2, &self.model_pool.bind_group, &[model_offset]);
        rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        if mesh.index_count > 0 {
            rpass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }

        Ok(())
    }
}

impl<W: Window> RendererBackend for Renderer<W> {
    type Frame<'a> = RenderFrame<'a, W>;

    fn begin_frame<'a>(&'a mut self) -> Result<Self::Frame<'a>, PlatformError> {
        if self.config.is_none() {
            self.configure_surface();
        }

        self.window.request_redraw();

        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        Ok(RenderFrame::new(self, frame, view))
    }

    fn create_material(
        &mut self,
        handle: MaterialHandle,
        definition: MaterialDefinition,
    ) -> Result<(), PlatformError> {
        self.materials.create_material(
            &self.device,
            &self.shaders,
            &self.textures,
            handle,
            &definition,
        )
    }

    fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition) {
        self.meshes.create_mesh(&self.device, handle, &definition);
    }

    fn create_shader(
        &mut self,
        handle: ShaderHandle,
        definition: ShaderDefinition,
    ) -> Result<(), PlatformError> {
        self.shaders
            .create_shader(&self.device, handle, &definition)
    }

    fn create_texture(
        &mut self,
        handle: TextureHandle,
        definition: TextureDefinition,
    ) -> Result<(), PlatformError> {
        self.textures
            .create_texture(&self.device, &self.queue, handle, &definition)
    }

    fn reserve_draw_capacity(&mut self, count: u64) {
        self.model_pool.reserve(&self.device, count);
    }

    fn resize(&mut self, width: u32, height: u32) {
        if self.config.is_none() {
            self.configure_surface();
            return;
        }

        self.config.as_mut().unwrap().width = width.max(1);
        self.config.as_mut().unwrap().height = height.max(1);
        self.surface
            .configure(&self.device, self.config.as_ref().unwrap());
        self.depth_view = Some(create_depth_view(&self.device, width.max(1), height.max(1)));
    }

    fn set_camera(&mut self, view_proj: Mat4) {
        self.globals.write(&self.queue, view_proj);
    }

    fn set_fullscreen(&mut self, enabled: bool) {
        self.window.set_fullscreen_enabled(enabled);
    }

    fn set_present_mode(&mut self, mode: PresentMode) {
        self.desired_present_mode = mode;

        if self.config.is_some() {
            self.configure_surface();
        }
    }

    fn shutdown(&mut self) {
        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
    }
}

pub(crate) struct RenderFrame<'a, W: Window> {
    first_depth_pass: bool,
    first_pass: bool,
    frame: wgpu::SurfaceTexture,
    model_cursor: u64,
    renderer: &'a mut Renderer<W>,
    view: wgpu::TextureView,
}

impl<'a, W: Window> RenderFrame<'a, W> {
    fn new(
        renderer: &'a mut Renderer<W>,
        frame: wgpu::SurfaceTexture,
        view: wgpu::TextureView,
    ) -> Self {
        Self {
            first_depth_pass: true,
            first_pass: true,
            frame,
            model_cursor: 0,
            renderer,
            view,
        }
    }
}

impl<'a, W: Window> renderer::RenderFrame<'a> for RenderFrame<'a, W> {
    fn execute_commands(
        &mut self,
        phase: &RenderPhase,
        cmds: &[DrawCommand],
    ) -> Result<(), PlatformError> {
        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(&format!("Render {phase:?} Encode")),
                });

        let is_first_pass = self.first_pass;
        self.first_pass = false;

        let color_load_op = if is_first_pass {
            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
        } else {
            wgpu::LoadOp::Load
        };

        let depth_enabled = matches!(phase.domain, RenderDomain::Other(_) | RenderDomain::World3D);

        let depth_stencil_attachment = if depth_enabled {
            let is_first_depth_pass = self.first_depth_pass;
            self.first_depth_pass = false;

            let depth_load_op = if is_first_depth_pass {
                wgpu::LoadOp::Clear(1.0)
            } else {
                wgpu::LoadOp::Load
            };

            Some(wgpu::RenderPassDepthStencilAttachment {
                view: self
                    .renderer
                    .depth_view
                    .as_ref()
                    .expect("depth view should be configured before the first frame"),
                depth_ops: Some(wgpu::Operations {
                    load: depth_load_op,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            })
        } else {
            None
        };

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: color_load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for cmd in cmds {
                match cmd {
                    DrawCommand::Mesh(mesh) => {
                        let offset = self.renderer.model_pool.write(
                            &self.renderer.queue,
                            self.model_cursor,
                            &mesh.transform,
                        )?;
                        self.model_cursor += 1;
                        self.renderer
                            .draw_mesh(&mut rpass, mesh, offset, depth_enabled)?;
                    }
                }
            }
        }

        self.renderer.queue.submit(Some(encoder.finish()));

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
