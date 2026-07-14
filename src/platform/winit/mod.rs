use std::time;

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
    events::{self, EventBus},
    platform::core::PlatformError,
    renderer::{
        FrameError, RenderError, RenderGraph, Renderer, RendererHandle, Viewport,
        backend::wgpu::Renderer as WgpuRenderer, render_queue_channel,
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
    events: EventBus,
    graph: RenderGraph,
    handler: Option<AppHandler<A::Instance>>,
}

struct AppHandler<I: ApplicationInstance> {
    accumulator: time::Duration,
    cfg: RuntimeConfig,
    instance: I,
    last_frame: time::Instant,
    renderer: Renderer<WgpuRenderer<Window>>,
    renderer_handle: RendererHandle,
}

impl<A: Application> AppState<A> {
    fn new(app: A, cfg: EngineConfig, graph: RenderGraph) -> Self {
        let (engine_writer, engine_reader) = engine_command_channel();

        Self {
            app,
            cfg,
            engine_reader,
            engine_writer,
            events: EventBus::new(),
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

        let runtime_config = RuntimeConfig::from(&self.cfg);
        let instance = self.app.create(&ApplicationContext::new(
            &self.engine_writer,
            &runtime_config,
            &mut self.events,
            &mut renderer_handle,
        ));

        self.handler = Some(AppHandler {
            accumulator: time::Duration::from_nanos(0),
            cfg: runtime_config,
            instance,
            last_frame: time::Instant::now(),
            renderer,
            renderer_handle,
        });

        Ok(())
    }

    fn redraw(&mut self) {
        let Some(handler) = self.handler.as_mut() else {
            return;
        };

        let _frame = tracing::info_span!(target: diag::FRAME, "frame").entered();

        let ctx = ApplicationContext::new(
            &self.engine_writer,
            &handler.cfg,
            &mut self.events,
            &mut handler.renderer_handle,
        );

        let current_frame = time::Instant::now();
        handler.accumulator += current_frame.duration_since(handler.last_frame);
        handler.last_frame = current_frame;
        let mut tick_count = 0;

        ctx.events.swap_all();
        while handler.accumulator >= handler.cfg.tick_freq {
            handler.accumulator -= handler.cfg.tick_freq;
            tick_count += 1;

            let _update = tracing::debug_span!(target: diag::FRAME, "updating game simulation", tick = tick_count).entered();
            handler.instance.update(&ctx, handler.cfg.tick_freq);
        }

        let alpha = if handler.accumulator.is_zero() {
            0.0
        } else {
            handler.accumulator.as_secs_f32() / handler.cfg.tick_freq.as_secs_f32()
        };

        let prepared = match handler
            .renderer
            .prepare(|phase| handler.instance.render(phase, alpha))
        {
            Ok(prepared) => prepared,
            Err(err) => {
                tracing::error!(target: diag::FRAME, error = ?err, "frame render failed");
                handler.instance.handle_error(err);
                return;
            }
        };

        let mut res = handler.renderer.submit(&prepared);

        if let Err(RenderError::Frame(FrameError::SwapchainLost | FrameError::OutdatedSurface)) =
            &res
        {
            tracing::warn!(
                target: diag::FRAME,
                error = ?res.as_ref().err(),
                "surface lost or outdated, reconfiguring and retrying frame"
            );
            handler.renderer.reconfigure();
            res = handler.renderer.submit(&prepared);
        }

        if let Err(err) = res {
            tracing::error!(target: diag::FRAME, error = ?err, "frame render failed");
            handler.instance.handle_error(err);
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let width = size.width.max(1);
        let height = size.height.max(1);

        self.events.publish(events::Resized { height, width });

        let Some(handler) = self.handler.as_mut() else {
            return;
        };

        handler.renderer.resize(width, height);
        handler
            .renderer_handle
            .set_viewport(Viewport::new(width, height));
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
                EngineCommand::SetTickFreq(freq) => {
                    self.cfg.tick_freq = freq.max(time::Duration::from_nanos(1));
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

        handler.cfg = RuntimeConfig::from(&self.cfg);
        handler
            .renderer
            .set_fullscreen(handler.cfg.window_mode == WindowMode::BorderlessFullscreen);
        handler.renderer.set_present_mode(handler.cfg.present_mode);
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, _: DeviceEvent) {}

    fn exiting(&mut self, _: &ActiveEventLoop) {
        let Some(mut handler) = self.handler.take() else {
            return;
        };

        handler.instance.quit();
        handler.renderer.shutdown();
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
