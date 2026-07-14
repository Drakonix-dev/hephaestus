use std::time;

use crate::{diagnostics::DiagnosticsConfig, renderer::PresentMode};

pub struct EngineConfig {
    pub diagnostics: DiagnosticsConfig,
    pub present_mode: PresentMode,
    pub tick_freq: time::Duration,
    pub window_mode: WindowMode,
    pub window_title: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            diagnostics: DiagnosticsConfig::default(),
            present_mode: PresentMode::Vsync,
            tick_freq: time::Duration::from_secs(1) / 20,
            window_mode: WindowMode::Windowed,
            window_title: "Engine Window".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeConfig {
    pub present_mode: PresentMode,
    pub tick_freq: time::Duration,
    pub window_mode: WindowMode,
}

impl From<&EngineConfig> for RuntimeConfig {
    fn from(cfg: &EngineConfig) -> Self {
        Self {
            present_mode: cfg.present_mode,
            tick_freq: cfg.tick_freq,
            window_mode: cfg.window_mode,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    Windowed,
    BorderlessFullscreen,
}
