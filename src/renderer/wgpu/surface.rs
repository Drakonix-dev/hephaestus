use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{platform::core::WindowInfo};

pub(crate) struct Surface<'a> {
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) surface: wgpu::Surface<'a>,
    pub(crate) surface_format: wgpu::TextureFormat,
}

impl<'a> Surface<'a> {
    pub(crate) async fn new<T>(window: &'a T, info: WindowInfo) -> Self
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
            queue,
            surface,
            surface_format,
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width <= 0 || height <= 0 {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
}
//
// pub(crate) struct Presenter<'a> {
//     current_frame: Option<wgpu::SurfaceTexture>,
//     surface: &'a Surface<'a>,
// }
//
// impl<'a> Presenter<'a> {
//     pub(crate) fn new(surface: &'a Surface) -> Self {
//         Self {
//             current_frame: None,
//             surface,
//         }
//     }
//
//     fn draw_mesh(&mut self, rpass: &mut wgpu::RenderPass, draw: &DrawMesh) {
//         let mesh = self.meshes.get_mesh(&draw.mesh).unwrap();
//         let material = self.materials.get_material(&draw.material).unwrap();
//         let shader = self.shaders.get_shader(&material.shader).unwrap();
//         let pipeline = self.pipelines.get_or_create_pipeline(self.surface_format, &self.device, &shader);
//
//         rpass.set_pipeline(&pipeline.pipeline);
//         rpass.set_bind_group(0, &material.bind_group, &[]);
//         rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
//         rpass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
//         rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
//     }
// }
//
// impl<'a> RendererBackend for Presenter<'a> {
//     fn execute_commands(&mut self, phase: RenderPhase, cmds: &[DrawCommand]) {
//         let frame = self.current_frame.get_or_insert_with(|| {
//             self.surface.surface.get_current_texture()
//                 .expect("Failed to acquire frame")
//         }); 
//
//         let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
//         let mut encoder = self.surface.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
//             label: Some(&format!("Render {:?} Encode", phase)),
//         });
//
//         {
//             let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//                 label: Some("Render Pass"),
//                 color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                     view: &view,
//                     resolve_target: None,
//                     depth_slice: None,
//                     ops: wgpu::Operations {
//                         load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),  
//                         store: wgpu::StoreOp::Store,
//                     },
//                 })],
//                 depth_stencil_attachment: None,
//                 timestamp_writes: None,
//                 occlusion_query_set: None,
//             });
//
//             for cmd in cmds {
//                 match cmd {
//                     DrawCommand::Mesh(mesh) => self.draw_mesh(&mut rpass, mesh),
//                 }
//             }
//         }
//
//         self.surface.queue.submit(Some(encoder.finish()));
//     }
//
//     fn present(&mut self) {
//         if let Some(frame) = self.current_frame.take() {
//             frame.present();
//         }
//     }
// }
