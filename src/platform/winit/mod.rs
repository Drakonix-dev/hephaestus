use std::{mem::{self}, sync::Arc};

use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowId}};

use crate::{events::Event, platform::core::WindowInfo, renderer::{backend::wgpu::Renderer as WgpuRenderer, RenderGraph, RenderQueue, Renderer, RendererHandle}, Application, ApplicationContext};

pub(crate) struct WinitPlatform;

impl WinitPlatform {
    pub fn run<A: Application + 'static>(app: A, graph: RenderGraph) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app_state = AppState::Uninitialized { app, graph };

        if let Err(error) = event_loop.run_app(&mut app_state) {
            panic!("error: {}", error);
        }
    }
}

enum AppState<'a, A: Application> {
    Initialized(AppHandler<'a, A>),
    MaybeUninit,
    Uninitialized { app: A, graph: RenderGraph },
}

impl<'a, A: Application> AppState<'a, A> {
    fn init(&mut self, event_loop: &ActiveEventLoop) {
        match mem::replace(self, AppState::MaybeUninit) {
            AppState::Initialized(_) => panic!("Already initialized"),
            AppState::Uninitialized { app, graph } => {
                *self = Self::Initialized(AppHandler::new(app, event_loop, graph));
            },
            _ => {},
        }
    }
}

impl <'a, A: Application> ApplicationHandler for AppState<'a, A> {
    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        match self {
            AppState::Initialized(handler) => handler.window.request_redraw(),
            _ => return,
        };
    }
    
    fn exiting(&mut self, _: &ActiveEventLoop) {
        let handler = match self {
            AppState::Initialized(handler) => handler,  
            _ => return,
        };
        handler.app.quit();
    }
    
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            AppState::Uninitialized { .. } => self.init(event_loop),
            _ => {},
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let handler = match self {
            AppState::Initialized(handler) => handler,
            _ => return,
        };

        let Some(event) = handler.get_event(&event) else {
            return;
        };

        let ctx = ApplicationContext {
            renderer: &mut handler.renderer_handle,
        };

        handler.app.handle_event(&ctx, event);
    }
}

struct AppHandler<'a, A: Application> {
    app: A,
    renderer: Renderer<WgpuRenderer<'a>>,
    renderer_handle: RendererHandle,
    window: Arc<Window>,
}

impl<'a, A: Application> AppHandler<'a, A> {
    fn new(app: A, event_loop: &ActiveEventLoop, graph: RenderGraph) -> Self {
        let attrs = Window::default_attributes()
            .with_title("Engine Window");
        let window = Arc::new(event_loop
            .create_window(attrs)
            .expect("failed to create window"));
        let info = WindowInfo {
            width: window.inner_size().width,
            height: window.inner_size().height,
        };
        
        let backend = pollster::block_on(WgpuRenderer::new(Box::new(window.clone()), info));
        let (writer, reader) = RenderQueue::new();
        let renderer = Renderer::new(backend, &graph, reader);
        let renderer_handle = RendererHandle::new(writer);
        
        Self {
            app,
            renderer,
            renderer_handle,
            window,
        }
    }

    fn get_event(&mut self, event: &WindowEvent) -> Option<Event> {
        match event {
            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
                Some(Event::Resized(size.width, size.height))
            },   
            _ => None,
        }
    }
}
