# Import and Export

Format adapters live in `crates/notatus-export`.

The adapters translate between external formats and the canonical `Dataset`
schema. They do not own annotation semantics.

## Annotation Filter

Exports use `AnnotationFilter` to decide which annotations should be written.

Implemented filters:

- `accepted_and_reviewed`
- `all_non_rejected`
- `ready_for_training`: reviewed and accepted records, plus drafts a person
  drew or imported. Model predictions still in draft are left out, because
  nobody has checked them. The desktop app exports with this filter, for
  object annotations and image labels alike.

The default filter exports accepted and reviewed annotations only.

## Finished Media

`finished_media(dataset)` returns a copy holding only media marked done and
the records on them. Every exported image without annotations tells a model
there is nothing in it, so exporting unfinished media teaches it to miss
objects. Labels are kept whole so class indices match a full export.

The desktop export dialog applies it through its "Only media marked done"
switch, which is on by default.

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

Only bounding-box annotations are exported. Polygon annotations are left out
of the YOLO detection export and counted in `YoloExportSummary::skipped_polygons`.

The filesystem writer creates a dataset Ultralytics can train on directly
(`yolo detect train data=yolo/data.yaml`):

```text
yolo/
  .notatus-export        marks the folder as replaceable by the next export
  data.yaml              absolute dataset path, split folders, class names
  classes.txt            class names in dataset order
  images/<split>/<name>  copies of the images
  labels/<split>/<stem>.txt
```

- Splits come from each asset's `split`; unassigned media goes to `train`.
  When no media is in the validation split, `val` points at `images/train`.
- Images keep their file names. When two images in a split share a stem, the
  later one gets a short asset-id suffix, and its label file matches.
- Images are copied rather than hard-linked, so tools that edit exported
  images in place cannot change the originals. `fs::copy` clones files on
  file systems that support it.
- The export is built in a hidden sibling folder and swapped in at the end. A
  failure, such as a missing image, leaves any earlier export intact, and no
  label files from an earlier export are left behind.
- An existing output folder is replaced only when it is empty or carries the
  `.notatus-export` marker; any other folder is refused with
  `ExportError::OutputExists`.
- Only media stored at local paths can be exported for now.

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
4. Converts normalized YOLO coordinates to pixel-space `BoundingBox`. Values
   up to `1e-3` outside `0..=1`, which rounding produces for boxes touching
   the image edge, are accepted and the box is clipped to the image.
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

The filesystem writer creates:

```text
coco/
  annotations.json
```

## Classification Export

`write_classification_export` writes the image classifications the filter
accepts as both `classifications.json` and `classifications.csv`, one row per
image label with its media path, label name, review state, and confidence.

```text
classification/
  classifications.json
  classifications.csv
```

## Export From the App

File → Export annotations (Ctrl+E) opens a dialog with a checkbox per format,
the finished-media switch, and a summary of what will be written, which
follows the choices as they change. At least one format stays checked. It then
asks for a folder and writes each chosen format into its own subfolder:
`yolo/`, `coco/`, and `classification/`. The choices last for the session.

## Error Handling

`ExportError` covers:

- invalid dataset validation
- invalid geometry
- unknown assets
- unknown labels
- invalid YOLO lines
- images that cannot be read or copied, and media stored remotely
- output folders that Notatus did not write
