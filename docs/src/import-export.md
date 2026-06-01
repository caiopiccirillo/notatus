# Import and Export

Format adapters live in `crates/notatus-export`.

The adapters translate between external formats and the canonical `Dataset`
schema. They do not own annotation semantics.

## Annotation Filter

Exports use `AnnotationFilter` to decide which annotations should be written.

Implemented filters:

- `accepted_and_reviewed`
- `all_non_rejected`

The default filter exports accepted and reviewed annotations only.

## YOLO Detection Export

YOLO export produces one annotation file per asset:

```rust
pub struct YoloAnnotationFile {
    pub asset_id: AssetId,
    pub image_path: String,
    pub contents: String,
}
```

Each line uses the YOLO detection format:

```text
class_index center_x center_y width height
```

Coordinates are normalized to the image dimensions. The core schema stores
pixel coordinates, so export performs the conversion at the boundary.

Only bounding-box annotations are exported. Polygon annotations are currently
ignored by the YOLO detection adapter.

## YOLO Detection Import

YOLO import reads:

```rust
pub struct YoloImportFile {
    pub asset_id: AssetId,
    pub contents: String,
}
```

For each line, the adapter:

1. Finds the referenced asset.
2. Parses the class index.
3. Resolves the class index against `dataset.labels`.
4. Converts normalized YOLO coordinates to pixel-space `BoundingBox`.
5. Creates an imported draft annotation.

Imported annotations use:

```rust
AnnotationSource::Imported
```

with format set to `yolo`.

## COCO Detection Export

COCO export produces:

```rust
pub struct CocoDataset {
    pub info: CocoInfo,
    pub images: Vec<CocoImage>,
    pub annotations: Vec<CocoAnnotation>,
    pub categories: Vec<CocoCategory>,
}
```

Asset IDs and label IDs are translated to COCO integer IDs at export time.

Each exported annotation includes:

- COCO bounding box
- area
- category ID
- image ID
- `iscrowd = 0`
- Notatus annotation ID in attributes
- review state in attributes
- confidence when present

Only bounding-box annotations are exported by the current COCO adapter.

## Error Handling

`ExportError` covers:

- invalid dataset validation
- invalid geometry
- unknown assets
- unknown labels
- invalid YOLO lines
