# System Overview

Notatus is organized around a clean adapter boundary:

```text
                 +------------------+
                 |   notatus-core   |
                 | canonical schema |
                 +---------+--------+
                           |
        +------------------+------------------+
        |                  |                  |
+-------v------+   +-------v------+   +-------v------+
| notatus-ui   |   | notatus-     |   | notatus-     |
| GPUI app     |   | storage      |   | export       |
+--------------+   +--------------+   +--------------+
                           |
                    +------v-------+
                    | notatus-     |
                    | infer        |
                    +--------------+
```

`notatus-core` owns the dataset schema and validation rules. Every other crate
depends on it and treats it as the canonical representation.

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
active tool, selected asset, selected annotation, and dirty state, but the
dataset itself remains the authoritative data structure.

### External Model Boundary

Model execution is intentionally outside the desktop binary. A runner may be
Python, ONNX Runtime, TensorRT, a container, or a remote shim, as long as it
speaks the JSON-lines protocol in `notatus-infer`.
