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
    notatus-ui/
```

## Root Workspace

The root `Cargo.toml` defines shared package metadata:

- Rust edition: `2024`
- Rust version: `1.96.0`
- License: `MIT`
- Shared dependencies such as `serde`, `serde_json`, `time`, `uuid`,
  `thiserror`, and `tempfile`

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

- `ProjectStore`
- `LocalProjectStore`
- JSON and JSONL readers/writers
- storage-specific error types

## `notatus-export`

Translates between Notatus data and external training/interchange formats.

Implemented adapters:

- YOLO detection export
- YOLO detection import
- COCO detection export

## `notatus-infer`

Defines the pre-annotation request/response protocol for external model
runners.

It does not run models directly. It serializes and deserializes JSON-lines
messages that a model runner can consume.

## `notatus-ui`

Contains:

- renderer-independent `UiState`
- mutation helpers for labels, assets, selected asset, and bounding boxes
- default GPUI application shell through the `gpui-ui` feature

The GPUI shell currently includes:

- `gpui-component` title bar and Linux client-side window border
- command bar
- image picker
- resizable asset/canvas/inspector layout
- component sidebar navigation
- selected-image preview
