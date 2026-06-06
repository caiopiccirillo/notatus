# Desktop UI

The UI implementation lives in `crates/notatus-ui`.

The crate has two layers:

- `UiState`, which is renderer-independent and testable.
- `gpui_shell`, which is compiled by default through the `gpui-ui` feature.

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
used by component controls such as the titlebar and dock buttons.

## Window Chrome

Linux uses client-side window decorations.

The current shell uses `gpui-component::TitleBar` with the component window
border:

- visible minimize button
- visible maximize button
- visible close button
- draggable titlebar
- double-click maximize
- right-click window menu
- resize edges

## Titlebar Menus

Global commands live in classical titlebar menus:

- Project
- Media
- Labels
- Export

The menu triggers open dropdown menus below the titlebar button. Current actions
include opening the relevant dock, importing media, creating labels, and export
placeholders.

## Media Import

The Media titlebar menu has an Import media command. It opens a file picker and
currently accepts image files by probing dimensions with the `image` crate.

Unsupported files are skipped and reported through the Info dock status field.
The command is media-oriented so video and folder import can be added without
changing the left dock structure.

## Left Docks

The left panel is switched by bottom-bar dock buttons.

It contains separate docks for:

- Project: datasets available to work on
- Media: media rows with nested annotation rows
- Labels: label list and label customization

Media rows show their asset type and annotation count. Each media row can expand
to show its annotations, including label, review state, and geometry type. Media
rows are selectable and update `UiState::selected_asset`.

Label rows show their color swatch and annotation count. They are selectable and
update `UiState::selected_label`.

Long filenames are shortened with a middle truncation helper before rendering.
This keeps repeated screenshot-style filenames readable in a narrow panel.

## Bottom Bar

The bottom bar is the persistent dock switcher:

- Project
- Media
- Labels
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
state live in the GPUI shell, while completed mutations go through `UiState`.
The initial tools are:

- Draw Box maps window coordinates to image pixel coordinates, draws a preview
  while dragging, clamps positions to image bounds, and creates
  `AnnotationGeometry::Bbox` through `UiState`.
- Select hit-tests bounding boxes in image space, selects the topmost matching
  annotation, and keeps selection changes out of dataset dirty state.
- Pan/Zoom drags the shared image viewport and zooms the viewport with the
  scroll wheel.

Future segmentation tools should add new handlers to the canvas tool layer and
commit completed polygons through the canonical core schema.

## Right Docks

The right panel is split into:

- Annotations
- Info

The Annotations dock lists annotations for the selected media. The Info dock
shows dataset, selection, status, count, review queue, and selected-media
metadata.

Label customization lives in the Labels left dock.

## Layout

The main workspace uses `gpui-component` resizable panels:

- left dock panel
- center canvas
- right dock panel
- bottom dock switcher

The panel group stores its state by element ID, so resizing is handled by the
component library.
