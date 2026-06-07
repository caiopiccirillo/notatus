# Desktop UI

The UI implementation lives in `crates/notatus-ui`.

The crate has two layers:

- `UiState`, which is renderer-independent and testable.
- `app`, which is compiled by default through the `gpui-ui` feature.

## UI State

`UiState` wraps the canonical `Dataset` and tracks UI-specific state:

- active annotation tool (`Select`, `Draw Box`, `Pan/Zoom`)
- selected asset
- selected annotation
- selected label
- dirty flag

Implemented mutations:

- create a new project
- create UI state from an existing validated dataset
- rename the project
- set active tool
- mark saved
- add label
- select label
- update label name
- update label color
- add local image asset
- select asset
- add human bounding-box annotation

`UiState` always mutates the canonical dataset. It does not define a separate
annotation schema.

## GPUI Feature

The native desktop shell runs by default:

```sh
cargo run -p notatus-ui
```

The feature enables:

- `gpui`
- `gpui-component`
- `gpui-component-assets`
- `image`

The `image` crate is used to probe dimensions for selected image files before
adding them as assets. The `gpui-component-assets` crate provides the SVG icons
used by component controls such as the dock buttons.

## Window Chrome

The current shell uses `gpui-component::TitleBar` with GPUI's default platform
decoration behavior. The titlebar is intentionally informational: it shows the
application identity, current project name, and saved/unsaved status.

## Workflow Navigation

Commands live in the workflow docks instead of the titlebar. The bottom bar
switches the primary left dock in the order users usually work:

- Project
- Labels
- Media

The Project dock owns creating, opening, saving, saving as, and renaming local
projects. The Labels dock owns label creation and label editing. The Media dock
owns media import and media selection.

Project persistence uses `notatus-storage::LocalProjectStore`. A project can be
unsaved in memory or backed by a local folder. Creating or opening a project
while the current project is dirty asks before discarding changes.

Attempts to jump ahead in the workflow, such as importing media before labels
exist or drawing without an active label, redirect to the needed dock and show a
GPUI component notification.

## Media Import

The Media dock has an Import media command. It opens a file picker and currently
accepts image files by probing dimensions with the `image` crate.

Unsupported files are skipped and reported through the Info dock status field.
The command is media-oriented so video and folder import can be added without
changing the left dock structure.

## Left Docks

The left panel is switched by bottom-bar dock buttons.

It contains separate docks for:

- Project: project name, location, and persistence commands
- Labels: label creation, label list, and label customization
- Media: media import and media selection

Media rows show their asset type and annotation count. They are selectable and
update `UiState::selected_asset`. Annotation rows live in the right-side
Annotations dock so media navigation stays compact.

Label rows show their color swatch and annotation count. They are selectable and
update `UiState::selected_label`.

Long filenames are shortened with a middle truncation helper before rendering.
This keeps repeated screenshot-style filenames readable in a narrow panel.

## Bottom Bar

The bottom bar is the persistent dock switcher:

- Project
- Labels
- Media
- Annotations
- Info

The left group controls the left dock. The right group controls the right dock.

## Canvas Area

The center panel previews the selected image with GPUI's `img` element:

- local images are loaded from their selected path
- object fit is `Contain`
- loading and fallback messages are provided
- S3 object previews currently show a not-implemented message

The canvas has a tool-oriented interaction layer. Tool metadata and interaction
state live in the app layer, while completed mutations go through `UiState`.
The initial tools are:

- Draw Box maps window coordinates to image pixel coordinates, draws a preview
  while dragging, clamps positions to image bounds, and creates
  `AnnotationGeometry::Bbox` through `UiState`.
- Select hit-tests bounding boxes in image space, selects the topmost matching
  annotation, and keeps selection changes out of dataset dirty state.
- Pan/Zoom drags the shared image viewport and zooms the viewport with the
  scroll wheel.
- Fit resets the viewport to the canvas-contained image bounds. It is available
  as a toolbar button and by double-clicking the canvas.

Future segmentation tools should add new handlers to the canvas tool layer and
commit completed polygons through the canonical core schema.

## Right Docks

The right panel is split into:

- Annotations
- Info

The Annotations dock lists annotations for the selected media, highlights the
matching canvas annotation while a row is hovered, and exposes compact row
actions for changing an annotation label. The Info dock shows cross-cutting
context such as active tool, active label, counts, review queue, status messages,
and selected-media metadata.

Label customization lives in the Labels left dock.

## Layout

The main workspace uses `gpui-component` resizable panels:

- left dock panel
- center canvas
- right dock panel
- bottom dock switcher

The panel group stores its state by element ID, so resizing is handled by the
component library.
