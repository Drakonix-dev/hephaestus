mod capture;
mod config;

pub mod diag;

use tracing_subscriber::{
    EnvFilter, Layer, filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

pub use capture::{
    CaptureGuard, CaptureHandle, CapturedRecord, FieldValue, RecordKind, capture,
    capture_with_capacity,
};
pub use config::{DiagnosticsConfig, OtlpConfig};

pub struct DiagnosticsGuard {
    capture: CaptureHandle,
    #[cfg(feature = "otel")]
    provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

impl DiagnosticsGuard {
    pub fn capture(&self) -> CaptureHandle {
        self.capture.clone()
    }
}

impl Drop for DiagnosticsGuard {
    fn drop(&mut self) {
        #[cfg(feature = "otel")]
        if let Some(provider) = &self.provider {
            let _ = provider.shutdown();
        }
    }
}

pub fn init(config: &DiagnosticsConfig) -> DiagnosticsGuard {
    let handle = CaptureHandle::new(config.capture_capacity);

    let fmt_layer = config
        .console
        .then(|| fmt::layer().with_filter(env_filter(config)));

    let capture_layer = handle
        .layer()
        .with_filter(LevelFilter::from_level(config.level));

    let subscriber = tracing_subscriber::registry()
        .with(fmt_layer)
        .with(capture_layer);

    #[cfg(feature = "otel")]
    let (subscriber, provider) = {
        let (otel_layer, provider) = otel::build(config);
        (subscriber.with(otel_layer), provider)
    };

    let _ = subscriber.try_init();

    DiagnosticsGuard {
        capture: handle,
        #[cfg(feature = "otel")]
        provider,
    }
}

fn env_filter(config: &DiagnosticsConfig) -> EnvFilter {
    if let Ok(filter) = EnvFilter::try_from_env("HEPHAESTUS_LOG") {
        return filter;
    }
    if let Ok(filter) = EnvFilter::try_from_default_env() {
        return filter;
    }
    let directive = config
        .filter
        .clone()
        .unwrap_or_else(|| config.level.to_string().to_lowercase());
    EnvFilter::new(directive)
}

#[cfg(feature = "otel")]
mod otel {
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_otlp::WithExportConfig as _;
    use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
    use tracing_subscriber::{Layer, registry::LookupSpan};

    use super::DiagnosticsConfig;

    pub(super) fn build<S>(
        config: &DiagnosticsConfig,
    ) -> (
        Option<Box<dyn Layer<S> + Send + Sync>>,
        Option<SdkTracerProvider>,
    )
    where
        S: tracing::Subscriber + for<'a> LookupSpan<'a> + Send + Sync,
    {
        let Some(otlp) = &config.otlp else {
            return (None, None);
        };

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(otlp.endpoint.clone())
            .build()
            .expect("failed to build OTLP span exporter");

        let provider = SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(
                Resource::builder()
                    .with_service_name(otlp.service_name.clone())
                    .build(),
            )
            .build();

        let tracer = provider.tracer("hephaestus");
        let layer = tracing_opentelemetry::layer().with_tracer(tracer).boxed();

        (Some(layer), Some(provider))
    }
}
