use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowId}};

use crate::{events::Event, platform::core::WindowInfo, renderer::{backend::wgpu::Renderer, RenderGraph, RendererHandle, RendererThread}, Application, ApplicationContext};

pub(crate) struct WinitPlatform;

impl WinitPlatform {
    pub fn run<A: Application + 'static>(app: A, graph: RenderGraph) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut handler = AppHandler::new(app, graph);

        if let Err(error) = event_loop.run_app(&mut handler) {
            panic!("error: {}", error);
        }
    }
}

struct AppHandler<A: Application + 'static> {
    app: A,
    graph: RenderGraph,
    renderer: Option<RendererHandle>,
}

impl<A: Application + 'static> AppHandler<A> {
    fn new(app: A, graph: RenderGraph) -> Self {
        Self {
            app,
            graph,
            renderer: None,
        }
    }
}

impl<A: Application + 'static> ApplicationHandler for AppHandler<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("Engine Window");
        let window = event_loop
            .create_window(attrs)
            .expect("failed to create window");

        let wgpu_renderer = pollster::block_on(Renderer::new(&window, WindowInfo {
            width: window.inner_size().width,
            height: window.inner_size().height,
        }));
        let (renderer_handle, join) = RendererThread::spawn(Box::new(wgpu_renderer), self.graph);
        
        let wgpu_renderer = pollster::block_on(WgpuRenderer::new(&handle, info));
        self.renderer = Some(Renderer::new(Box::new(wgpu_renderer)));

        let ctx = ApplicationContext {
            renderer: self.renderer.as_mut().unwrap(),
        };

        self.app.init(&mut ctx);
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(event) = get_window_event(&event) else {
            return;
        };

        if !self.initialized {
            return;
        }

        let ctx = ApplicationContext {
            assets: self.assets.as_ref().unwrap(),
            rendering: self.renderer.as_ref().unwrap(),
            world:  self.world.as_mut().unwrap(),
        };

        self.app.handle_event(&ctx, event);
    }
}

fn get_window_event(evt: &WindowEvent) -> Option<Event> {
    match evt {
        // WindowEvent::Resized(size) => Some(Event::Resized(size.width, size.height)),
        // WindowEvent::RedrawRequested => Some(Event::Redraw),
        WindowEvent::CloseRequested => Some(Event::Quit),
        _ => None,
    }
}
