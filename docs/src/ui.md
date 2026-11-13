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
- remove label
- add local image asset
- select asset
- remove media asset
- select annotation
- update annotation label
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

The shell uses `gpui-component::TitleBar`. It shows the application identity,
the project name, and the save state. On Linux the window may draw its own
title bar; its close button goes through the same check as a compositor close
request: pending changes are saved first, and a project never saved to a
folder asks before it is discarded.

## Workflow Navigation

The bottom bar switches the left dock between Dataset and Media, and the right
dock between Annotations and Info. Nothing has to happen in a fixed order:
media can be imported before labels exist, and drawing without a label creates
one.

Project persistence uses `notatus-storage::LocalProjectStore` through the
autosave pipeline described in [Storage](storage.md). A project can be unsaved
in memory or backed by a local folder.

## Keyboard

The annotation loop is keyboard-driven. `?` opens a sheet of every shortcut,
grouped under Draw, Media, Review, View, and Project; each row shows the key
bound to its action, so the sheet follows the bindings in `actions.rs`. Ctrl+K
opens the command palette, which lists the same commands with their keys.

Commands that open pickers or dialogs (new, open, save, import, export) are
deferred out of their action listener with `run_outside_update`, because they
update the window through a weak handle and gpui panics on a nested update.

## Media Import

Import images (Ctrl+I) picks image files. Import folder (Ctrl+Shift+I) searches
the chosen folders recursively for JPEG, PNG, BMP, GIF, WebP, and TIFF files,
skipping hidden files and folders and not entering symlinked folders. Both read
image sizes off the UI thread, recording the size after EXIF orientation so
rotated phone photos match what the canvas shows. Files already in the project
are skipped and reported in the status message.

## Left Docks

- Dataset: project identity, persistence and export buttons, and labels
- Media: import and the media list

Each media row starts with a green check when the media is done or a hollow
circle when it is not, then its name, then its annotation count. Removing
media, which also removes its annotations, is in the row's right-click menu.

Label rows show their color and annotation count; renaming, recoloring, and
removing are in the row's options and right-click menus.

## Canvas Area

The center panel shows the selected image with GPUI's `img` element, fitted to
the canvas; S3 media is not previewed yet. Tool metadata and interaction state
live in the app layer, and completed changes go through `UiState`.

- Box (B) draws on empty space and moves or resizes existing boxes; Alt always
  draws. Drags shorter than four screen pixels are treated as clicks.
- Polygon (P) adds a point per click and closes the shape on double-click.
- Select (V) selects, moves, and resizes without drawing, and pans on empty
  space.
- The scroll wheel zooms around the pointer, up to 32 times the fit-to-view
  size; F or a double-click fits the image again.

Every drag also ends when the button is released outside the canvas, so a box
dragged past the edge is created at the edge and a move stays one undo step.

## Right Docks

- Annotations, the default: the selected media's image labels and objects. Each
  object row shows its review state, accept and reject toggles, and a label
  menu. Y accepts, X rejects, and U resets the selected annotation; accepting
  or rejecting moves to the next draft on the same media.
- Info: active tool, zoom, counts, media still to review, and media metadata.

## Layout

The main workspace uses `gpui-component` resizable panels:

- left dock panel
- center canvas
- right dock panel
- bottom dock switcher

The panel group stores its state by element ID, so resizing is handled by the
component library.
