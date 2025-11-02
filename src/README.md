Main thread (Platform)
  ├── owns: Winit EventLoop + Window + wgpu::Surface
  ├── receives: Winit events (resize, input, close)
  └── communicates via channels

Render thread (GPU)
  ├── owns: wgpu::Device + Queue
  ├── processes: RenderCommands
  └── submits: draw calls, frame submission

     ┌────────────────────┐
     │      game          │
     │ (your gameplay)    │
     └────────┬───────────┘
              │ depends on
              ▼
     ┌────────────────────┐
     │     engine         │
     │ (ECS, systems)     │
     └────────┬───────────┘
              │ depends on
              ▼
     ┌────────────────────┐
     │    platform        │
     │ (window, loop,     │
     │  renderer bridge)  │
     └────────┬───────────┘
              │ depends on
              ▼
     ┌────────────────────┐
     │    renderer        │
     │ (wgpu wrapper,     │
     │  render thread)    │
     └────────────────────┘

# Commands
enum RenderCommand {
    CreateMaterial(MaterialDefinition),
    DrawFrame,
}

enum EventCommand {
    Resized(width: u32, height: u32),
    Redraw,
    Quit,
}
