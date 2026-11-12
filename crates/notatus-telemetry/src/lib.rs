//! OpenTelemetry initialization for the Notatus desktop application.
//!
//! Configures tracing (spans), metrics, and structured logging with OTLP/HTTP
//! export. If no `OTEL_EXPORTER_OTLP_ENDPOINT` is set, telemetry is
//! silently disabled and only a human-readable console layer is active.

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::ExporterBuildError;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::EnvFilter;

const SERVICE_NAME: &str = "notatus";

pub fn init_telemetry() -> TelemetryGuard {
    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("notatus=info"));

    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);

    let resource = Resource::builder().with_service_name(SERVICE_NAME).build();

    let tracer_provider = build_tracer_provider(&resource);
    let meter_provider = build_meter_provider(&resource);

    match tracer_provider {
        Ok(ref provider) => {
            let tracer = provider.tracer(SERVICE_NAME);
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .with(otel_layer)
                .init();
        }
        Err(ref err) => {
            tracing::info!(%err, "no OTLP endpoint configured, running without telemetry export");

            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .init();
        }
    }

    TelemetryGuard {
        tracer_provider: tracer_provider.ok(),
        meter_provider: meter_provider.ok(),
    }
}

fn build_tracer_provider(resource: &Resource) -> Result<SdkTracerProvider, ExporterBuildError> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .build()?;

    // Since opentelemetry_sdk 0.28 the batch processor runs on its own
    // background thread, so no async runtime is required.
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource.clone())
        .build();

    opentelemetry::global::set_tracer_provider(provider.clone());

    Ok(provider)
}

fn build_meter_provider(resource: &Resource) -> Result<SdkMeterProvider, ()> {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .build()
        .map_err(|err| {
            tracing::warn!(%err, "failed to build OTLP metrics exporter");
        })?;

    let reader = PeriodicReader::builder(exporter).build();

    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource.clone())
        .build();

    opentelemetry::global::set_meter_provider(provider.clone());

    Ok(provider)
}

pub struct TelemetryGuard {
    tracer_provider: Option<SdkTracerProvider>,
    meter_provider: Option<SdkMeterProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(ref provider) = self.tracer_provider {
            // shutdown() flushes pending spans before stopping the processor.
            let _ = provider.shutdown();
        }
        if let Some(ref provider) = self.meter_provider {
            let _ = provider.shutdown();
        }
    }
}
