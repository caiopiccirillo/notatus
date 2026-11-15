# Roadmap

Likely next steps, most valuable first, based on the current implementation.

## Near Term

### More Model Tasks

Runners can only suggest boxes and polygons for object detection. Next:
image classification, interactive segmentation (click to get a mask), and
running a model in the background while annotating.

### Undo Without Copying the Dataset

Undo snapshots the whole dataset on every change, up to 200 times. Recording
only the records each change touches keeps memory flat on large projects and
is a first step toward an operation log.

### Media at Scale

- thumbnails and filters in the media list
- a virtualized list for projects with thousands of images

## Medium Term

### Polygon Editing

Polygons can be drawn and relabeled but not reshaped: vertex dragging,
inserting, and deleting are missing.

### Label Management

- reordering labels, which also sets YOLO class indices
- warning about duplicate label names

### Object Storage

The schema can represent S3-compatible locations. Future work: credentials,
object listing, a local cache, content hashing, and version-aware references.

## Longer Term

### Video Annotation

Frame extraction or timeline rendering, temporal spans, object tracks,
interpolation, and tracking-model integration.

### Dataset Versioning

Dataset snapshots, manifest-level version metadata, and integration points for
DVC, lakeFS, Git, or object-store version IDs.

### Collaboration

Annotator attribution, model-versus-human comparison, and audit-trail views.
