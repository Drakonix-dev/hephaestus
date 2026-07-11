use std::{
    cell::Cell,
    mem::{self},
    sync::mpsc,
    thread, time,
};

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use super::core;
use crate::{
    Application, ApplicationContext, ApplicationInstance,
    events::Event,
    platform::core::PlatformError,
    renderer::{
        RenderGraph, RenderQueue, Renderer, RendererHandle, backend::wgpu::Renderer as WgpuRenderer,
    },
};

pub(crate) struct WinitPlatform;

impl<'a> WinitPlatform {
    pub fn run<A: Application + 'a>(app: A, graph: RenderGraph) -> Result<(), PlatformError> {
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
                        return;
                    }
                };

                *self = Self::Initialized(handler);
            }
            _ => {}
        }
    }
}

impl<A: Application> ApplicationHandler for AppState<A> {
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            AppState::Initialized(handler) => {
                if handler.exit_requested.get() {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, _: DeviceEvent) {}

    fn exiting(&mut self, _: &ActiveEventLoop) {
        let old_state = mem::replace(self, AppState::MaybeUninit);

        if let AppState::Initialized(handler) = old_state {
            let (app, graph) = handler.exit();

            *self = AppState::Uninitialized { app, graph };
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match self {
            AppState::Uninitialized { .. } => self.init(event_loop),
            _ => {}
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let handler = match self {
            AppState::Initialized(handler) => handler,
            _ => return,
        };

        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
            WindowEvent::RedrawRequested => handler.redraw(),
            WindowEvent::Resized(size) => handler.resize(size),
            _ => {}
        }
    }
}

struct AppHandler<A: Application> {
    app: A,
    exit_requested: Cell<bool>,
    instance: A::Instance,
    graph: RenderGraph,
    last_tick: time::Instant,
    renderer: Renderer<WgpuRenderer<Window>>,
    renderer_handle: RendererHandle,
    simulation: thread::JoinHandle<()>,
    tick_rx: mpsc::Receiver<()>,
    timestep: time::Duration,
}

impl<A: Application> AppHandler<A> {
    fn new(
        mut app: A,
        event_loop: &ActiveEventLoop,
        graph: RenderGraph,
    ) -> Result<Self, (A, core::PlatformError)> {
        let attrs = Window::default_attributes().with_title("Engine Window");

        let window = match event_loop.create_window(attrs) {
            Ok(window) => window,
            Err(err) => return Err((app, err.into())),
        };

        let backend = match pollster::block_on(WgpuRenderer::new(window)) {
            Ok(backend) => backend,
            Err(err) => return Err((app, err)),
        };

        let (writer, reader) = RenderQueue::new();
        let mut renderer_handle = RendererHandle::new(writer);

        let renderer = match Renderer::new(backend, &graph, reader) {
            Ok(renderer) => renderer,
            Err(err) => return Err((app, err)),
        };

        let (tick_tx, tick_rx) = mpsc::channel();
        let timestep = time::Duration::from_millis(16); // ~60 hz
        let simulation = match spawn_simulation_ticker(tick_tx, timestep) {
            Ok(simulation) => simulation,
            Err(err) => return Err((app, err)),
        };

        let exit_requested = Cell::new(false);

        let instance = app.create(&ApplicationContext::new(
            &exit_requested,
            &mut renderer_handle,
        ));
        let handler = Self {
            app,
            exit_requested,
            instance,
            graph,
            last_tick: time::Instant::now(),
            renderer,
            renderer_handle,
            simulation,
            tick_rx,
            timestep,
        };

        Ok(handler)
    }

    fn exit(self) -> (A, RenderGraph) {
        let AppHandler {
            app,
            mut instance,
            graph,
            mut renderer,
            simulation,
            tick_rx,
            ..
        } = self;

        instance.quit();
        renderer.shutdown();

        drop(tick_rx);
        let _ = simulation.join();

        (app, graph)
    }

    fn redraw(&mut self) {
        while self.tick_rx.try_recv().is_ok() {
            let now = time::Instant::now();
            let dt = now - self.last_tick;
            self.last_tick = now;

            let ctx = ApplicationContext::new(&self.exit_requested, &mut self.renderer_handle);
            self.instance.update(&ctx, dt);
        }

        let res = self.renderer.render(|phase| self.instance.render(phase));

        if let Err(err) = res {
            self.instance.handle_error(err);
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let width = size.width.max(1);
        let height = size.height.max(1);

        self.renderer.resize(width, height);

        let ctx = ApplicationContext::new(&self.exit_requested, &mut self.renderer_handle);
        self.instance
            .handle_event(&ctx, Event::Resized(width, height));
    }
}

fn spawn_simulation_ticker(
    tx: mpsc::Sender<()>,
    timestep: time::Duration,
) -> Result<thread::JoinHandle<()>, PlatformError> {
    let handle = thread::Builder::new()
        .name("hephaestus-ticker".to_owned())
        .spawn(move || {
            let mut last = time::Instant::now();

            loop {
                let now = time::Instant::now();
                if now - last >= timestep {
                    last = now;

                    if tx.send(()).is_err() {
                        // main thread dropped receiver -> exit
                        break;
                    }
                }

                thread::sleep(time::Duration::from_millis(1));
            }
        })?;

    Ok(handle)
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
