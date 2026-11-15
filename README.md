# notatus

Local-first visual data annotation in Rust.

Notatus is being built as a desktop annotation tool for computer-vision
datasets, starting with image bounding boxes and a schema that can grow into
segmentation, video tracking, remote object storage, and model-assisted
pre-annotation.

## Current Shape

This repository is a Rust workspace split by responsibility:

- `notatus-core`: canonical dataset schema, IDs, geometry, provenance, review
  state, and validation.
- `notatus-storage`: local project folder persistence using deterministic
  JSON/JSONL files.
- `notatus-export`: YOLO detection import/export, COCO detection export, and
  image classification export.
- `notatus-infer`: JSON-lines protocol for external pre-annotation model
  runners.
- `notatus-telemetry`: OpenTelemetry tracing and metrics init with OTLP/HTTP
  export and a console fallback when no endpoint is configured.
- `notatus-ui`: testable desktop UI state plus an optional GPUI launch feature.

The core design rule is that YOLO, COCO, CVAT, model outputs, and UI state are
adapters around the Notatus schema. They are not the source of truth.

## Project Format

A local project folder contains:

- `notatus.project.json`: schema version and project metadata.
- `labels.json`: class taxonomy.
- `assets.jsonl`: stable asset records with dimensions, location, split, and
  optional content hash.
- `annotations.jsonl`: annotation records with geometry, provenance, confidence,
  review state, timestamps, and metadata.
- `classifications.jsonl`: whole-image labels, with the same provenance and
  review state.

Geometry is stored in original image pixel coordinates. Exporters convert to
format-specific coordinates, such as YOLO normalized center coordinates.

## Model Pre-Annotation

Pre-annotation is intentionally an external-process boundary in v1. A runner can
use Python, ONNX Runtime, TensorRT, a container, or a remote shim as long as it
speaks the JSON-lines request/response protocol in `notatus-infer`.

In the app, choose a runner with "Choose model runner…" in the command palette,
then press Ctrl+R to suggest annotations on the current image or Ctrl+Shift+R on
every unfinished one. `examples/runners/ultralytics_runner.py` runs Ultralytics
YOLO models. See `docs/src/inference.md`.

Model predictions become draft annotations with model provenance and confidence.
They are never silently treated as accepted ground truth: they show their
confidence until reviewed, and exports leave out suggestions nobody reviewed.

## Development

Requires Rust 1.96.0 or newer.

On Linux, the desktop UI links against the X11 keyboard libraries, so building
`notatus-ui` (including its tests) needs their development packages:

```sh
sudo apt install libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev
```

Fontconfig is loaded at runtime (see `.cargo/config.toml`), so it needs no
development package.

Run all tests with default features:

```sh
cargo test --workspace
```

Run the UI binary shell without GPUI:

```sh
cargo run -p notatus-ui --no-default-features
```

Run the GPUI shell:

```sh
cargo run -p notatus-ui
```

Build or serve the implementation guide with mdBook:

```sh
mdbook serve docs
```

## Near-Term Roadmap

- Review drafts and model suggestions across all media, not one image at a
  time, filtered by label or confidence.
- Make undo record changes instead of copying the dataset.

See `docs/src/roadmap.md` for the longer list.
