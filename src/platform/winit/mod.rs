use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowId}};

use crate::{app::{Application, ApplicationContext}, assets::{MaterialManager, MeshManager, ShaderManager, TextureManager}, core::{ecs::World, events::Event}, platform::core::WindowInfo, renderer::wgpu::Surface, rendering::Renderer};

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

struct AppHandler<'a, A: Application + 'static> {
    app: A,
    initialized: bool,
    materials: Option<MaterialManager<'a>>,
    meshes: Option<MeshManager<'a>>,
    renderer: Option<Renderer<'a>>,
    shaders: Option<ShaderManager<'a>>,
    textures: Option<TextureManager<'a>>,
    window: Option<Window>,
    world: Option<World>,
}

impl<'a, A: Application + 'static> AppHandler<'a, A> {
    fn new(app: A) -> Self {
        Self {
            app,
            initialized: false,
            materials: None,
            meshes: None,
            renderer: None,
            shaders: None,
            textures: None,
            window: None,
            world: None,
        }
    }
}

impl<'a, A: Application + 'static> ApplicationHandler for AppHandler<'a, A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("Engine Window");
        let window = event_loop
            .create_window(attrs)
            .expect("failed to create window");

        let surface = Surface::new(&window, WindowInfo {
            width: window.inner_size().width,
            height: window.inner_size().height,
        });

        let wgpu_renderer = pollster::block_on(WgpuRenderer::new(&handle, info));
        self.renderer = Some(Renderer::new(Box::new(wgpu_renderer)));
        
        self.materials = Some(MaterialManager::new(&self.renderer));
        self.meshes = Some(MeshManager::new(Box::new(wgpu_renderer)));
        self.shaders = Some(ShaderManager::new(Box::new(wgpu_renderer)));
        self.textures =  Some(TextureManager::new(Box::new(wgpu_renderer)));
        self.window = Some(window);
        self.world = Some(World::new());

        let ctx = ApplicationContext {
            materials: &self.materials.unwrap(),
            meshes: &self.meshes.unwrap(),
            rendering: &self.renderer.unwrap(),
            shaders: &self.shaders.unwrap(),
            textures: &self.textures.unwrap(),
            world: &mut self.world.unwrap(),
        };

        self.app.init(&mut ctx);

        self.initialized = true;
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
