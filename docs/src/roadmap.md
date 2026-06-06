# Roadmap

This page documents likely next steps based on the current implementation.

## Near Term

### Canvas Tools

Extend the canvas tool architecture:

- add annotation edit handles
- add keyboard shortcuts and cursor affordances for tools
- add zoom reset and fit-to-screen controls
- keep new canvas interactions routed through the GPUI tool layer

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

- a segmentation tool entry
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
