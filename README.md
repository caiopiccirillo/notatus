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
- `notatus-export`: YOLO detection import/export and COCO detection export.
- `notatus-infer`: JSON-lines protocol for external pre-annotation model
  runners.
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

Geometry is stored in original image pixel coordinates. Exporters convert to
format-specific coordinates, such as YOLO normalized center coordinates.

## Model Pre-Annotation

Pre-annotation is intentionally an external-process boundary in v1. A runner can
use Python, ONNX Runtime, TensorRT, a container, or a remote shim as long as it
speaks the JSON-lines request/response protocol in `notatus-infer`.

Model predictions become draft annotations with model provenance and confidence.
They are never silently treated as accepted ground truth.

## Development

Requires Rust 1.96.0 or newer.

Run the non-GPUI core checks:

```sh
cargo test --workspace
```

Run the UI binary shell without GPUI:

```sh
cargo run -p notatus-ui
```

Run the GPUI shell when the native GPUI dependency is available:

```sh
cargo run -p notatus-ui --features gpui-ui
```

Build or serve the implementation guide with mdBook:

```sh
mdbook serve docs
```

## Near-Term Roadmap

- Build the GPUI canvas and project navigation around `notatus-ui::UiState`.
- Add image-folder import with dimension probing and content hashing.
- Add write-to-disk export helpers for YOLO and COCO.
- Add an external model runner process manager around the existing protocol.
- Add polygon annotation once the bounding-box workflow is reliable.
