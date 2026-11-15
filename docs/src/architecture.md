# System Overview

Notatus is organized around one canonical schema with adapters around it:

```text
                      +---------------------------+
                      |        notatus-ui         |
                      | GPUI app around UiState   |
                      +---+--------+--------+--+--+
                          |        |        |  |
          +---------------+  +-----+   +----+  +-----------------+
          |                  |         |                         |
+---------v------+  +--------v-----+  +v-------------+  +--------v--------+
| notatus-       |  | notatus-     |  | notatus-     |  | notatus-        |
| storage        |  | export       |  | infer        |  | telemetry       |
| project folder |  | YOLO, COCO,  |  | runner       |  | tracing, OTLP   |
|                |  | classif.     |  | protocol and |  |                 |
|                |  |              |  | processes    |  |                 |
+-------+--------+  +------+-------+  +------+-------+  +-----------------+
        |                  |                 |
        +------------------+--------+--------+
                                    |
                           +--------v---------+
                           |   notatus-core   |
                           | canonical schema |
                           +------------------+
```

The UI is the only crate that combines the others. Storage, export, and
inference each depend on `notatus-core` alone, so each can be used and tested
without the app. Telemetry depends on nothing else in the workspace.

`notatus-core` owns the dataset schema and validation rules. Every crate but
telemetry depends on it, the UI included, and treats it as the canonical
representation.

## Design Principles

### Canonical Schema First

Annotations are stored in Notatus-native records. Format-specific concerns such
as YOLO normalized center coordinates or COCO category IDs are handled only at
adapter boundaries.

This avoids coupling the app to any one training format and keeps future
exporters, importers, and dataset versioning workflows straightforward.

### Local-First, Versionable Data

The local storage backend writes deterministic project files:

- `notatus.project.json`
- `labels.json`
- `assets.jsonl`
- `annotations.jsonl`

These files are friendly to Git, DVC, lakeFS, object storage manifests, and
future dataset versioning workflows.

### Explicit Provenance

Annotations record their source:

- `human`
- `model`
- `imported`

Model predictions are stored as draft annotations with confidence and model
metadata. They are never silently treated as accepted labels.

### UI State Is Not the Schema

`notatus-ui::UiState` is a testable mutation layer around `Dataset`. It tracks
the active tool, the selection, undo history, and unsaved changes, but the
dataset itself remains the authoritative data structure. Its fields are
private, so every change goes through a method that validates it, records an
undo step, and marks the project dirty for autosave.

### External Model Boundary

Model execution is intentionally outside the desktop binary. A runner may be
Python, ONNX Runtime, TensorRT, a container, or a remote shim, as long as it
speaks the JSON-lines protocol in `notatus-infer`, which also starts and
supervises the runner process. Which runner a project uses is kept in the
user's own settings, never in the project, because a shared project must not
be able to name a command to run.

### Observability Boundary

Telemetry is initialized through `notatus-telemetry`, not directly by the UI
shell. The crate owns OTLP exporter configuration and the console fallback so
that the UI does not need to know whether an OTLP collector is reachable. It
returns a `TelemetryGuard` that the binary must hold for the lifetime of the
process so spans and metrics flush on shutdown.
