use std::{mem::{self}, sync::atomic::{AtomicBool, Ordering}};

use winit::{application::ApplicationHandler, event::{DeviceEvent, DeviceId, WindowEvent}, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowId}};

use crate::{events::Event, platform::core::{HasWindowInfo, WindowInfo}, renderer::{backend::wgpu::Renderer as WgpuRenderer, RenderGraph, RenderQueue, Renderer, RendererHandle}, Application, ApplicationContext};

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
    // fn about_to_wait(&mut self, _: &ActiveEventLoop) {
    //     match self {
    //         AppState::Initialized(handler) => handler.window.request_redraw(),
    //         _ => return,
    //     };
    // }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, _: DeviceEvent) {}
    
    fn exiting(&mut self, _: &ActiveEventLoop) {
        let old_state = mem::replace(self, AppState::MaybeUninit);
        
        if let AppState::Initialized(mut handler) = old_state {
            handler.closed.store(true, Ordering::SeqCst);
            handler.app.quit();
            handler.renderer.shutdown();

            *self = AppState::Uninitialized {
                app: handler.app,
                graph: handler.graph,
            };
        }
    }
    
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            AppState::Uninitialized { .. } => self.init(event_loop),
            _ => {},
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let handler = match self {
            AppState::Initialized(handler) => handler,
            _ => return,
        };

        let ctx = ApplicationContext {
            renderer: &mut handler.renderer_handle,
        };

        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                event_loop.exit();
            },
            WindowEvent::RedrawRequested => {
                if handler.closed.load(Ordering::SeqCst) {
                    return
                }
                
                handler.renderer.render(|phase| {
                    handler.app.render(phase)
                });
            },
            WindowEvent::Resized(size) => {
                if handler.closed.load(Ordering::SeqCst) {
                    return
                }
                
                let width = size.width.max(1);
                let height = size.height.max(1);
                
                handler.renderer.resize(width, height);
                handler.app.handle_event(&ctx, Event::Resized(width, height));
            },
            _ => {},
        }
    }
}

struct AppHandler<'a, A: Application> {
    app: A,
    closed: AtomicBool,
    graph: RenderGraph,
    renderer: Renderer<WgpuRenderer<'a, Window>>,
    renderer_handle: RendererHandle,
}

impl<'a, A: Application> AppHandler<'a, A> {
    fn new(app: A, event_loop: &ActiveEventLoop, graph: RenderGraph) -> Self {
        let attrs = Window::default_attributes()
            .with_title("Engine Window");
        let window = event_loop
            .create_window(attrs)
            .expect("failed to create window");

        let backend = pollster::block_on(WgpuRenderer::new(window));
        let (writer, reader) = RenderQueue::new();
        let renderer = Renderer::new(backend, &graph, reader);
        let renderer_handle = RendererHandle::new(writer);

        let mut handler = Self {
            app,
            closed: AtomicBool::new(false),
            graph,
            renderer,
            renderer_handle,
        };

        handler.app.init(&ApplicationContext{
            renderer: &mut handler.renderer_handle,
        });

        handler
    }
}

impl HasWindowInfo for Window {
    fn get_window_info(&self) -> WindowInfo {
        WindowInfo {
            height: self.inner_size().height,
            width: self.inner_size().width,
        }
    }   
}
