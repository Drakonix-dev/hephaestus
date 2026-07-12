use tracing::Level;

use crate::diagnostics::capture::DEFAULT_CAPACITY;

#[derive(Clone, Debug)]
pub struct DiagnosticsConfig {
    pub capture_capacity: usize,
    pub console: bool,
    pub filter: Option<String>,
    pub level: Level,
    pub otlp: Option<OtlpConfig>,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            capture_capacity: DEFAULT_CAPACITY,
            console: true,
            filter: None,
            level: Level::INFO,
            otlp: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OtlpConfig {
    pub endpoint: String,
    pub service_name: String,
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4318/v1/traces".to_owned(),
            service_name: "hephaestus".to_owned(),
        }
    }
}
