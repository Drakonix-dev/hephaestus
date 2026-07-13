use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
        mpsc::{self, Receiver},
    },
    thread::{self, JoinHandle},
    time,
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
    commands::{EngineCommand, EngineCommandReader, EngineCommandWriter, engine_command_channel},
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

        let mut app_state = AppState::new(app, cfg, graph);
        event_loop.run_app(&mut app_state)?;

        Ok(())
    }
}

struct AppState<A: Application> {
    app: A,
    cfg: EngineConfig,
    engine_reader: EngineCommandReader,
    engine_writer: EngineCommandWriter,
    graph: RenderGraph,
    handler: Option<AppHandler<A::Instance>>,
}

struct AppHandler<I: ApplicationInstance> {
    instance: I,
    last_tick: time::Instant,
    renderer: Renderer<WgpuRenderer<Window>>,
    renderer_handle: RendererHandle,
    runtime_config: RuntimeConfig,
    simulation: JoinHandle<()>,
    tick_rx: Receiver<()>,
}

impl<A: Application> AppState<A> {
    fn new(app: A, cfg: EngineConfig, graph: RenderGraph) -> Self {
        let (engine_writer, engine_reader) = engine_command_channel();

        Self {
            app,
            cfg,
            engine_reader,
            engine_writer,
            graph,
            handler: None,
        }
    }

    fn init_handler(&mut self, event_loop: &ActiveEventLoop) -> Result<(), EngineError> {
        let mut attrs = Window::default_attributes().with_title(self.cfg.window_title.clone());
        if self.cfg.window_mode == WindowMode::BorderlessFullscreen {
            attrs = attrs.with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
        }

        let window = match event_loop.create_window(attrs) {
            Ok(window) => window,
            Err(err) => return Err(PlatformError::from(err).into()),
        };

        let initial_size = window.inner_size();
        let viewport = Viewport::new(initial_size.width, initial_size.height);

        let (writer, reader) = render_queue_channel();
        let mut renderer_handle = RendererHandle::new(writer, viewport);

        let backend = pollster::block_on(WgpuRenderer::new(&self.cfg, window))?;
        let renderer = Renderer::new(backend, &self.graph, reader)?;

        let (tick_tx, tick_rx) = mpsc::channel();
        let tick_rate_hz = Arc::new(AtomicU32::new(self.cfg.tick_rate_hz));

        let simulation = spawn_simulation_ticker(tick_tx, tick_rate_hz.clone())?;

        let runtime_config = RuntimeConfig::from(&self.cfg);
        let instance = self.app.create(&ApplicationContext::new(
            &self.engine_writer,
            &runtime_config,
            &mut renderer_handle,
        ));

        self.handler = Some(AppHandler {
            instance,
            last_tick: time::Instant::now(),
            renderer,
            renderer_handle,
            runtime_config,
            simulation,
            tick_rx,
        });

        Ok(())
    }

    fn redraw(&mut self) {
        let Some(handler) = self.handler.as_mut() else {
            return;
        };

        let _frame = tracing::info_span!(target: diag::FRAME, "frame").entered();

        while handler.tick_rx.try_recv().is_ok() {
            let now = time::Instant::now();
            let dt = now - handler.last_tick;
            handler.last_tick = now;

            let _tick = tracing::debug_span!(
                target: diag::SIM,
                "update",
                dt_us = dt.as_micros() as u64,
            )
            .entered();

            let ctx = ApplicationContext::new(
                &self.engine_writer,
                &handler.runtime_config,
                &mut handler.renderer_handle,
            );
            handler.instance.update(&ctx, dt);
        }

        let res = handler
            .renderer
            .render(|phase| handler.instance.render(phase));

        if let Err(err) = res {
            tracing::error!(target: diag::FRAME, error = ?err, "frame render failed");
            handler.instance.handle_error(err);
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let Some(handler) = self.handler.as_mut() else {
            return;
        };

        let width = size.width.max(1);
        let height = size.height.max(1);

        handler.renderer.resize(width, height);
        handler
            .renderer_handle
            .set_viewport(Viewport::new(width, height));

        let ctx = ApplicationContext::new(
            &self.engine_writer,
            &handler.runtime_config,
            &mut handler.renderer_handle,
        );
        handler
            .instance
            .handle_event(&ctx, Event::Resized(width, height));
    }
}

impl<A: Application> ApplicationHandler for AppState<A> {
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let mut dirty = false;

        for cmd in self.engine_reader.drain() {
            match cmd {
                EngineCommand::RequestExit => event_loop.exit(),
                EngineCommand::SetPresentMode(mode) => {
                    self.cfg.present_mode = mode;
                    dirty = true;
                }
                EngineCommand::SetTickRate(hz) => {
                    self.cfg.tick_rate_hz = hz.max(1);
                    dirty = true;
                }
                EngineCommand::SetWindowMode(mode) => {
                    self.cfg.window_mode = mode;
                    dirty = true;
                }
            }
        }

        if !dirty {
            return;
        }

        let Some(handler) = self.handler.as_mut() else {
            return;
        };

        handler.runtime_config = RuntimeConfig::from(&self.cfg);
        handler
            .renderer
            .set_fullscreen(handler.runtime_config.window_mode == WindowMode::BorderlessFullscreen);
        handler
            .renderer
            .set_present_mode(handler.runtime_config.present_mode);
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, _: DeviceEvent) {}

    fn exiting(&mut self, _: &ActiveEventLoop) {
        let Some(mut handler) = self.handler.take() else {
            return;
        };

        handler.instance.quit();
        handler.renderer.shutdown();

        drop(handler.tick_rx);
        let _ = handler.simulation.join();
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.handler.is_some() {
            return;
        }

        if let Err(err) = self.init_handler(event_loop) {
            self.app.handle_error(err)
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Resized(size) => self.resize(size),
            _ => {}
        }
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
