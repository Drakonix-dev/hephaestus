use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowId}};

use crate::{app::{Application, ApplicationContext}, core::{ecs::World, events::Event}, platform::{WindowHandle, WindowInfo}, renderer::wgpu::Renderer as WgpuRenderer, rendering::Renderer};

pub(crate) struct WinitPlatform;

impl WinitPlatform {
    pub fn run<A: Application + 'static>(app: A) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut handler = AppHandler::new(app);

        if let Err(error) = event_loop.run_app(&mut handler) {
            panic!("error: {}", error);
        }
    }
}

struct AppHandler<A: Application + 'static> {
    app: A,
    context: Option<ApplicationContext>,
    window: Option<Window>,
}

impl<A: Application + 'static> AppHandler<A> {
    fn new(app: A) -> Self {
        Self {
            app,
            context: None,
            window: None,
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

        let handle = WindowHandle::new(
            window.display_handle().unwrap().as_raw(),  
            window.window_handle().unwrap().as_raw(),
        );
        let info = WindowInfo {
            width: window.inner_size().width,
            height: window.inner_size().height,
        };

        let renderer = pollster::block_on(WgpuRenderer::new(&handle, info));
        
        let mut ctx = ApplicationContext {
            renderer: Renderer::new(Box::new(renderer)),
            world: World::new(),
        };

        self.app.init(&mut ctx);
        
        self.context = Some(ctx);
        self.window = Some(window);
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

        if let  Some(ctx) = self.context.as_mut() {
            self.app.handle_event(ctx, event);
        }
    }
}

fn get_window_event(evt: &WindowEvent) -> Option<Event> {
    match evt {
        WindowEvent::Resized(size) => Some(Event::Resized(size.width, size.height)),
        WindowEvent::RedrawRequested => Some(Event::Redraw),
        WindowEvent::CloseRequested => Some(Event::CloseRequested),
        _ => None,
    }
}
