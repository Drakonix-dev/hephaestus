use crate::{diagnostics::DiagnosticsConfig, renderer::PresentMode};

const TICK_RATE_HZ: u32 = 60;

pub struct EngineConfig {
    pub diagnostics: DiagnosticsConfig,
    pub present_mode: PresentMode,
    pub tick_rate_hz: u32,
    pub window_mode: WindowMode,
    pub window_title: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            diagnostics: DiagnosticsConfig::default(),
            present_mode: PresentMode::Vsync,
            tick_rate_hz: TICK_RATE_HZ,
            window_mode: WindowMode::Windowed,
            window_title: "Engine Window".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    Windowed,
    BorderlessFullscreen,
}
