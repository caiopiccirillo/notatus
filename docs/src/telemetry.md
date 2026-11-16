# Telemetry

Telemetry initialization lives in `crates/notatus-telemetry`. The crate wires
the desktop binary into OpenTelemetry tracing and metrics through a single
entry point.

## Entry Point

```rust
pub fn init_telemetry() -> TelemetryGuard
```

`init_telemetry` is called once during process startup, typically from the
`notatus-ui` binary. The returned `TelemetryGuard` must be held for the
lifetime of the process so its `Drop` implementation can flush spans and
shut down the meter provider.

## Layers

The subscriber always includes:

- an `EnvFilter` built from `RUST_LOG`, defaulting to `notatus=info`
- a `tracing_subscriber::fmt` layer for human-readable console output

Exporters are built only when `OTEL_EXPORTER_OTLP_ENDPOINT`,
`OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`, or `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`
is set, because the OTLP exporter otherwise falls back to `localhost:4318` and
logs failed exports every few seconds when no collector runs there. Then an
OpenTelemetry layer is appended so spans flow into the configured collector.
If no endpoint is set or the exporter build fails, the binary logs a single
info line and continues with the console layer only. Telemetry export failure must
never break the desktop app.

## Exporters

Both exporters use OTLP over HTTP:

- `opentelemetry_otlp::SpanExporter` with a batched `TracerProvider`
- `opentelemetry_otlp::MetricExporter` with a `PeriodicReader`

Both providers run on the `TokioCurrentThread` runtime and register a
`service.name` resource attribute of `notatus`.

## Shutdown

`TelemetryGuard` flushes the tracer provider and shuts down the meter
provider when dropped. Dropping the guard is the only thing that guarantees
pending spans and metrics reach the collector, so the binary should keep it
on the stack or in its top-level state until exit.
