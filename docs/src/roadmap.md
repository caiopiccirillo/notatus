# Roadmap

This page documents likely next steps based on the current implementation.

## Near Term

### Interactive Bounding Boxes

Add a real canvas interaction model:

- map window coordinates to image pixel coordinates
- draw a box preview while dragging
- clamp boxes to image bounds
- create `AnnotationGeometry::Bbox`
- select and edit existing boxes

### Label Management

Add UI controls for:

- creating labels
- editing label names
- setting label colors
- selecting an active label for new annotations

### Project Persistence in UI

Wire the existing `LocalProjectStore` into the desktop shell:

- create project
- open project folder
- save project
- dirty-state prompts

### Export Commands

Add UI commands around the implemented adapters:

- export YOLO detection dataset
- import YOLO detections
- export COCO detection dataset

## Medium Term

### Polygon Segmentation

The core schema already includes polygons. The UI needs:

- point placement
- polygon closure
- vertex editing
- polygon validation

### Model Runner Integration

Build a process manager around `notatus-infer`:

- configure runner specs
- send JSON-lines requests
- receive predictions
- convert predictions to draft annotations
- surface diagnostics

### Object Storage

The schema can represent S3-compatible object locations. Future work should add:

- credential configuration
- object listing
- local cache
- content hashing
- version-aware references

## Longer Term

### Video Annotation

Extend assets and UI workflows for video:

- frame extraction or timeline rendering
- temporal annotation spans
- object tracks
- interpolation
- tracking model integration

### Dataset Versioning

Build on the local-first project format:

- dataset snapshots
- manifest-level version metadata
- integration points for DVC, lakeFS, Git, or object-store version IDs

### Review Workflow

Add higher-level quality controls:

- review queues
- accept/reject shortcuts
- annotator attribution
- model-vs-human comparison
- audit trail views
