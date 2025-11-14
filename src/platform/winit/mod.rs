use std::{mem::{self}, sync::atomic::{AtomicBool, Ordering}};

use winit::{application::ApplicationHandler, event::{DeviceEvent, DeviceId, WindowEvent}, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowId}};

use crate::{events::Event, platform::core::PlatformError, renderer::{backend::wgpu::Renderer as WgpuRenderer, RenderGraph, RenderQueue, Renderer, RendererHandle}, Application, ApplicationContext};
use super::core;

pub(crate) struct WinitPlatform;

impl WinitPlatform {
    pub fn run<A: Application + 'static>(app: A, graph: RenderGraph) -> Result<(), PlatformError> {
        env_logger::init();
        
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app_state = AppState::Uninitialized { app, graph };
        let res = event_loop.run_app(&mut app_state)?;

        Ok(res)
    }
}

enum AppState<A: Application> {
    Initialized(AppHandler<A>),
    MaybeUninit,
    Uninitialized { app: A, graph: RenderGraph },
}

impl<A: Application> AppState<A> {
    fn init(&mut self, event_loop: &ActiveEventLoop) {
        match mem::replace(self, AppState::MaybeUninit) {
            AppState::Initialized(_) => panic!("Already initialized"),
            AppState::Uninitialized { app, graph } => {
                let handler = match AppHandler::new(app, event_loop, graph) {
                    Ok(handler) => handler,
                    Err((mut app, err)) => {
                        app.handle_error(err);
                        event_loop.exit();
                        return
                    },
                };

                *self = Self::Initialized(handler);
            },
            _ => {},
        }
    }
}

impl <A: Application> ApplicationHandler for AppState<A> {
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

                let res = handler.renderer.render(|phase| {
                    handler.app.render(phase)
                });

                if let Err(err) = res {
                    handler.app.handle_error(err);
                }
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

struct AppHandler<A: Application> {
    app: A,
    closed: AtomicBool,
    graph: RenderGraph,
    renderer: Renderer<WgpuRenderer<Window>>,
    renderer_handle: RendererHandle,
}

impl<A: Application> AppHandler<A> {
    fn new(app: A, event_loop: &ActiveEventLoop, graph: RenderGraph) -> Result<Self, (A, core::PlatformError)> {
        let attrs = Window::default_attributes()
            .with_title("Engine Window");

        let window = match event_loop.create_window(attrs) {
            Ok(window) => window,
            Err(err) => {
                return Err((app, err.into()))
            },
        };

        let backend = match pollster::block_on(WgpuRenderer::new(window)) {
            Ok(backend) => backend,
            Err(err) => {
                return Err((app, err))
            },
        };
        
        let (writer, reader) = RenderQueue::new();
        let renderer_handle = RendererHandle::new(writer);
        
        let renderer = match Renderer::new(backend, &graph, reader) {
            Ok(renderer) => renderer,
            Err(err) => {
                return Err((app, err))
            },
        };

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

        Ok(handler)
    }
}

impl core::HasWindowInfo for Window {
    fn get_window_info(&self) -> core::WindowInfo {
        core::WindowInfo {
            height: self.inner_size().height,
            width: self.inner_size().width,
        }
    }   

    fn request_redraw(&self) {
        self.request_redraw()
    }
}

impl From<winit::error::EventLoopError> for PlatformError {
    fn from(err: winit::error::EventLoopError) -> Self {
        Self::LoopError(err.to_string())
    }
}

impl From<winit::error::OsError> for PlatformError {
    fn from(err: winit::error::OsError) -> Self {
        Self::WindowCreationFailed(err.to_string())
    }
}
