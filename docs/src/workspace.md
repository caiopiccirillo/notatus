# Workspace Layout

The Rust workspace is split by responsibility.

```text
notatus/
  Cargo.toml
  crates/
    notatus-core/
    notatus-storage/
    notatus-export/
    notatus-infer/
    notatus-telemetry/
    notatus-ui/
```

## Root Workspace

The root `Cargo.toml` defines shared package metadata:

- Rust edition: `2024`
- Rust version: `1.96.0`
- License: `MIT`
- Shared dependencies such as `serde`, `serde_json`, `time`, `uuid`,
  `thiserror`, and `tempfile`
- Observability stack: `tracing`, `tracing-subscriber`, `tracing-opentelemetry`,
  `opentelemetry`, `opentelemetry_sdk`, and `opentelemetry-otlp`

## `notatus-core`

Owns the canonical dataset schema:

- IDs
- project metadata
- labels
- assets
- asset locations
- bounding boxes
- polygons
- annotations
- provenance
- review state
- validation

No other crate should define a competing annotation model.

## `notatus-storage`

Persists a `Dataset` to a transparent local folder layout.

It defines:

- `ProjectStore` and `LocalProjectStore`
- atomic saves: every file is written to a temporary file, synced, and renamed
  into place
- lenient loading that skips damaged records, reports them in a `LoadReport`,
  and backs the original files up first
- media paths stored relative to the project folder when the media is inside
  it
- storage-specific error types

See [Storage](storage.md).

## `notatus-export`

Translates between Notatus data and external training/interchange formats.

Implemented adapters:

- YOLO detection export, in the Ultralytics layout with `data.yaml`
- YOLO detection import
- COCO detection export
- image classification export (JSON and CSV)

`AnnotationFilter` chooses which records are exported and `finished_media`
limits an export to media marked done. See [Import and Export](import-export.md).

## `notatus-infer`

Runs pre-annotation models as external processes.

- the JSON-lines request/response protocol
- `into_annotations`, which turns a response into draft annotations, clipping
  them to the image and skipping invalid predictions
- `ModelRunner`, which starts a runner program, keeps it alive between
  requests, and reports crashes, bad output, and timeouts
- `notatus-fake-runner`, a test binary that speaks the protocol without a model

It does not run models itself. See [Pre-Annotation](inference.md).

## `notatus-telemetry`

Initializes OpenTelemetry tracing and metrics for the desktop binary.

Responsibilities:

- configure an `EnvFilter` from `RUST_LOG` (defaulting to `notatus=info`)
- attach a human-readable `tracing-subscriber` console layer
- build OTLP/HTTP exporters for spans and metrics when an
  `OTEL_EXPORTER_OTLP_*ENDPOINT` variable is set
- silently fall back to console-only logging when no endpoint is set or the
  OTLP build fails
- return a `TelemetryGuard` that flushes spans and shuts down the meter
  provider on drop

The binary in `notatus-ui` calls `init_telemetry()` during startup and holds
the guard for the lifetime of the process.

## `notatus-ui`

Contains:

- `state`: the renderer-independent `UiState`, one file per topic (labels,
  media, annotations, review, classifications) plus its undo history
- `app`: the GPUI application, compiled through the default `gpui-ui`
  feature

See [Desktop UI](ui.md) for the app's module layout.

## `examples`

`examples/runners/ultralytics_runner.py` is a model runner for Ultralytics
YOLO detection models.
