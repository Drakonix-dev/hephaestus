use std::{
    mem::{self},
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
        mpsc,
    },
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
    Application, ApplicationContext, ApplicationInstance, EngineError,
    commands::{self, EngineCommand, EngineCommandReader, EngineCommandWriter},
    config::{EngineConfig, RuntimeConfig, WindowMode},
    diagnostics::diag,
    events::Event,
    platform::core::PlatformError,
    renderer::{
        RenderGraph, Renderer, RendererHandle, Viewport, backend::wgpu::Renderer as WgpuRenderer,
        render_queue_channel,
    },
};

pub(crate) struct WinitPlatform;

impl WinitPlatform {
    pub fn run<'a, A: Application + 'a>(
        app: A,
        cfg: EngineConfig,
        graph: RenderGraph,
    ) -> Result<(), PlatformError> {
        let _diagnostics = crate::diagnostics::init(&cfg.diagnostics);

        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app_state = AppState::Uninitialized { app, cfg, graph };
        event_loop.run_app(&mut app_state)?;

        Ok(())
    }
}

#[allow(clippy::large_enum_variant)]
enum AppState<A: Application> {
    Initialized(AppHandler<A>),
    MaybeUninit,
    Uninitialized {
        app: A,
        cfg: EngineConfig,
        graph: RenderGraph,
    },
}

impl<A: Application> AppState<A> {
    fn init(&mut self, event_loop: &ActiveEventLoop) {
        match mem::replace(self, AppState::MaybeUninit) {
            AppState::Initialized(_) => panic!("Already initialized"),
            AppState::Uninitialized { app, cfg, graph } => {
                let handler = match AppHandler::new(app, cfg, event_loop, graph) {
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
        if let AppState::Initialized(handler) = self
            && handler.exit_requested
        {
            event_loop.exit();
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, _: DeviceEvent) {}

    fn exiting(&mut self, _: &ActiveEventLoop) {
        let old_state = mem::replace(self, AppState::MaybeUninit);

        if let AppState::Initialized(handler) = old_state {
            let (app, cfg, graph) = handler.exit();

            *self = AppState::Uninitialized { app, cfg, graph };
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let AppState::Uninitialized { .. } = self {
            self.init(event_loop)
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
    cfg: EngineConfig,
    engine_commands: EngineCommandReader,
    engine_writer: EngineCommandWriter,
    exit_requested: bool,
    instance: A::Instance,
    graph: RenderGraph,
    last_tick: time::Instant,
    renderer: Renderer<WgpuRenderer<Window>>,
    renderer_handle: RendererHandle,
    runtime_config: RuntimeConfig,
    simulation: thread::JoinHandle<()>,
    tick_rate_hz: Arc<AtomicU32>,
    tick_rx: mpsc::Receiver<()>,
}

impl<A: Application> AppHandler<A> {
    fn new(
        mut app: A,
        cfg: EngineConfig,
        event_loop: &ActiveEventLoop,
        graph: RenderGraph,
    ) -> Result<Self, (A, EngineError)> {
        let mut attrs = Window::default_attributes().with_title(cfg.window_title.clone());
        if cfg.window_mode == WindowMode::BorderlessFullscreen {
            attrs = attrs.with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
        }

        let window = match event_loop.create_window(attrs) {
            Ok(window) => window,
            Err(err) => return Err((app, PlatformError::from(err).into())),
        };

        let initial_size = window.inner_size();
        let viewport = Viewport::new(initial_size.width, initial_size.height);

        let backend = match pollster::block_on(WgpuRenderer::new(&cfg, window)) {
            Ok(backend) => backend,
            Err(err) => return Err((app, err.into())),
        };

        let (writer, reader) = render_queue_channel();
        let mut renderer_handle = RendererHandle::new(writer, viewport);

        let renderer = match Renderer::new(backend, &graph, reader) {
            Ok(renderer) => renderer,
            Err(err) => return Err((app, err.into())),
        };

        let (engine_writer, engine_commands) = commands::engine_command_channel();
        let tick_rate_hz = Arc::new(AtomicU32::new(cfg.tick_rate_hz));

        let (tick_tx, tick_rx) = mpsc::channel();
        let simulation = match spawn_simulation_ticker(tick_tx, tick_rate_hz.clone()) {
            Ok(simulation) => simulation,
            Err(err) => return Err((app, err.into())),
        };

        let runtime_config = RuntimeConfig::from_config(&cfg);

        let instance = app.create(&ApplicationContext::new(
            &engine_writer,
            &runtime_config,
            &mut renderer_handle,
        ));
        let handler = Self {
            app,
            cfg,
            engine_commands,
            engine_writer,
            exit_requested: false,
            instance,
            graph,
            last_tick: time::Instant::now(),
            renderer,
            renderer_handle,
            runtime_config,
            simulation,
            tick_rate_hz,
            tick_rx,
        };

        Ok(handler)
    }

    fn exit(self) -> (A, EngineConfig, RenderGraph) {
        let AppHandler {
            app,
            cfg,
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

        (app, cfg, graph)
    }

    fn redraw(&mut self) {
        let _frame = tracing::info_span!(target: diag::FRAME, "frame").entered();

        for cmd in self.engine_commands.drain() {
            match cmd {
                EngineCommand::RequestExit => self.exit_requested = true,
                EngineCommand::SetPresentMode(mode) => {
                    self.renderer.set_present_mode(mode);
                    self.runtime_config.present_mode = mode;
                }
                EngineCommand::SetTickRate(hz) => {
                    let hz = hz.max(1);
                    self.tick_rate_hz.store(hz, Ordering::Relaxed);
                    self.runtime_config.tick_rate_hz = hz;
                }
                EngineCommand::SetWindowMode(mode) => {
                    self.renderer
                        .set_fullscreen(mode == WindowMode::BorderlessFullscreen);
                    self.runtime_config.window_mode = mode;
                }
            }
        }

        while self.tick_rx.try_recv().is_ok() {
            let now = time::Instant::now();
            let dt = now - self.last_tick;
            self.last_tick = now;

            let _tick = tracing::debug_span!(
                target: diag::SIM,
                "update",
                dt_us = dt.as_micros() as u64,
            )
            .entered();

            let ctx = ApplicationContext::new(
                &self.engine_writer,
                &self.runtime_config,
                &mut self.renderer_handle,
            );
            self.instance.update(&ctx, dt);
        }

        let res = self.renderer.render(|phase| self.instance.render(phase));

        if let Err(err) = res {
            tracing::error!(target: diag::FRAME, error = ?err, "frame render failed");
            self.instance.handle_error(err);
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let width = size.width.max(1);
        let height = size.height.max(1);

        self.renderer.resize(width, height);
        self.renderer_handle
            .set_viewport(Viewport::new(width, height));

        let ctx = ApplicationContext::new(
            &self.engine_writer,
            &self.runtime_config,
            &mut self.renderer_handle,
        );
        self.instance
            .handle_event(&ctx, Event::Resized(width, height));
    }
}

fn spawn_simulation_ticker(
    tx: mpsc::Sender<()>,
    tick_rate_hz: Arc<AtomicU32>,
) -> Result<thread::JoinHandle<()>, PlatformError> {
    let handle = thread::Builder::new()
        .name("hephaestus-ticker".to_owned())
        .spawn(move || {
            let mut last = time::Instant::now();
            let mut tick: u64 = 0;

            loop {
                let hz = tick_rate_hz.load(Ordering::Relaxed).max(1);
                let timestep = time::Duration::from_secs_f64(1.0 / hz as f64);

                let now = time::Instant::now();
                if now - last >= timestep {
                    last = now;

                    if tx.send(()).is_err() {
                        break;
                    }

                    tracing::trace!(target: diag::SIM, tick, "tick");
                    tick += 1;
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

    fn set_fullscreen_enabled(&self, enabled: bool) {
        let fullscreen = enabled.then_some(winit::window::Fullscreen::Borderless(None));
        self.set_fullscreen(fullscreen);
    }
}

impl From<winit::error::EventLoopError> for PlatformError {
    fn from(err: winit::error::EventLoopError) -> Self {
        Self::EventLoop {
            source: Box::new(err),
        }
    }
}

impl From<winit::error::OsError> for PlatformError {
    fn from(err: winit::error::OsError) -> Self {
        Self::WindowCreation {
            source: Box::new(err),
        }
    }
}
