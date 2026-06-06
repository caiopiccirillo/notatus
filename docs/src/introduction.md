# Introduction

Notatus is a local-first desktop tool for visual data annotation. The current
implementation focuses on image annotation with bounding boxes, while keeping the
domain model ready for polygons, video tracking, remote object storage, and
model-assisted pre-annotation.

The project is written in Rust. The UI is built with GPUI and uses
`gpui-component` for the application shell, sidebar, and resizable panels.

The main implementation rule is simple: the Notatus schema is the source of
truth. UI state, YOLO, COCO, CVAT-style adapters, storage backends, and model
outputs translate into and out of the canonical schema instead of inventing
parallel representations.

## Current Capabilities

- Canonical dataset model with project metadata, labels, assets, annotations,
  geometry, provenance, review state, and validation.
- Local project folder storage using deterministic JSON and JSONL files.
- YOLO detection export and import.
- COCO detection export.
- External JSON-lines protocol for model pre-annotation.
- GPUI desktop app with a component title bar, bottom dock navigation,
  resizable panels, left-dock project/label/media workflow, local project
  create/open/save commands, selected-image preview, and right-side
  annotation/info docks.

## Current Limitations

- The canvas supports bounding-box drawing, selection, pan/zoom, and fit-to-view.
- Project persistence is local-folder based. Recent project lists and richer
  save/discard/cancel flows are still planned.
- Remote S3-compatible storage is represented in the schema but has no storage
  backend yet.
- Polygon and video tracking schemas are planned, not implemented as workflows.
- The external inference protocol exists, but the desktop process manager that
  launches model runners is not implemented yet.
