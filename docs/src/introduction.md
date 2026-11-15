# Introduction

Notatus is a local-first desktop tool for visual data annotation: drawing
boxes and polygons on images, tagging whole images, reviewing model
suggestions, and exporting training-ready datasets. The domain model is ready
for video tracking and remote object storage, which have no workflows yet.

The project is written in Rust. The UI is built with GPUI and uses
`gpui-component` for the application shell, sidebar, and resizable panels.

The main implementation rule is simple: the Notatus schema is the source of
truth. UI state, YOLO, COCO, CVAT-style adapters, storage backends, and model
outputs translate into and out of the canonical schema instead of inventing
parallel representations.

## Current Capabilities

- Canonical dataset model with project metadata, labels, assets, annotations,
  image classifications, geometry, provenance, review state, and validation.
- Local project folders of deterministic JSON and JSONL files, saved
  atomically, autosaved, and loaded leniently with a recovery backup when
  records are damaged.
- Image import from files or whole folders, with EXIF orientation.
- Bounding-box and polygon drawing, box moving and resizing, image
  classification, pan and zoom, and undo.
- Keyboard-driven annotation and review: tool and review keys, a review queue
  across all media filtered by label or confidence, application menus, a
  command palette, and a shortcut sheet.
- A virtualized media list with search and status filters, for projects with
  thousands of images.
- Model pre-annotation through external runner processes, with suggestions
  kept as drafts until a person reviews them.
- YOLO detection import and export (Ultralytics layout), COCO detection
  export, and image classification export, leaving out unreviewed model
  suggestions and, by default, unfinished media.

## Current Limitations

- Polygons can be drawn and relabeled but not reshaped.
- Undo copies the whole dataset per step, which costs memory on large
  projects.
- The media list shows names only, without thumbnails.
- Remote S3-compatible storage is represented in the schema but has no storage
  backend yet.
- Video annotation is planned, not implemented.

See the [Roadmap](roadmap.md).
