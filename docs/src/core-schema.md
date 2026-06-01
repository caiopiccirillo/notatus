# Core Schema

The core schema lives in `crates/notatus-core`.

## IDs

The schema uses strongly typed UUID-backed IDs:

- `ProjectId`
- `AssetId`
- `LabelId`
- `AnnotationId`

Each ID is generated with UUID v7. This gives stable identifiers with useful
time-ordering properties.

## Dataset

`Dataset` is the root object:

```rust
pub struct Dataset {
    pub manifest: ProjectManifest,
    pub labels: Vec<Label>,
    pub assets: Vec<AssetRecord>,
    pub annotations: Vec<AnnotationRecord>,
}
```

It provides helpers to add labels, assets, and annotations, and lookup helpers
for assets and labels.

## Project Manifest

`ProjectManifest` stores:

- schema version
- project ID
- project name
- optional description
- created timestamp
- updated timestamp
- arbitrary metadata

The current schema version is `1`.

## Labels

`Label` stores:

- stable label ID
- display name
- optional color
- arbitrary metadata

Validation rejects empty label names and duplicate label IDs.

## Assets

`AssetRecord` stores image or video inputs:

- stable asset ID
- asset kind
- asset location
- dimensions
- optional content hash
- split
- arbitrary metadata

Supported asset kinds:

- `image`
- `video`

Supported locations:

- local path
- S3-compatible object reference

The S3 location is already represented in the schema:

```rust
S3Object {
    endpoint: Option<String>,
    bucket: String,
    key: String,
    version_id: Option<String>,
}
```

The storage backend for remote objects is not implemented yet.

## Geometry

Implemented geometry types:

- `BoundingBox`
- `Point`
- `Polygon`
- `AnnotationGeometry`

Bounding boxes use original image pixel coordinates:

```text
x, y, width, height
```

`x` and `y` are the top-left origin. Width and height must be positive.

Polygons require at least three points. Every point must be inside the image
bounds.

## Annotations

`AnnotationRecord` stores:

- stable annotation ID
- asset ID
- label ID
- geometry
- source provenance
- optional confidence
- review state
- timestamps
- arbitrary metadata

Supported sources:

- `Human`
- `Model`
- `Imported`

Supported review states:

- `Draft`
- `Reviewed`
- `Accepted`
- `Rejected`

## Validation

`Dataset::validate()` checks:

- supported schema version
- non-empty label names
- duplicate labels
- duplicate assets
- valid image dimensions
- duplicate annotations
- annotation references to known assets and labels
- confidence values in `0.0..=1.0`
- geometry bounds against the referenced asset dimensions

Every storage save/load and export path validates the dataset before using it.
