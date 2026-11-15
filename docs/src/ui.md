# Desktop UI

The UI implementation lives in `crates/notatus-ui`.

The crate has two layers:

- `state`: `UiState`, which is renderer-independent and testable.
- `app`: the GPUI application, compiled by default through the `gpui-ui`
  feature.

## UI State

`UiState` wraps the canonical `Dataset` and tracks UI-specific state:

- active annotation tool (Select, Draw Box, Draw Polygon)
- selected media, annotation, and label (the label new shapes get)
- undo and redo history
- the revision last saved, from which `is_dirty()` follows

Its fields are private and read through accessors such as `dataset()` and
`selected_asset()`. Every change is a method that validates it, records an
undo step, and moves to a new revision, so autosave and the close guard see
every edit. `batch` groups several changes into one undo step.

The methods are grouped by file under `src/state/`:

- `labels.rs`: create, rename, recolor, and remove labels
- `media.rs`: import, select, remove, and finish media, and step between media
- `annotations.rs`: draw boxes and polygons, select, relabel, move, and remove
  annotations, and step between them
- `review.rs`: review states, the review queue and its filter, and applying
  model suggestions
- `classifications.rs`: tag whole images with labels
- `history.rs`: the undo stack and edit coalescing for drags and typing

`UiState` always mutates the canonical dataset. It does not define a separate
annotation schema.

## GPUI App

The native desktop app runs by default:

```sh
cargo run -p notatus-ui
```

The `gpui-ui` feature enables `gpui`, `gpui_platform`, `gpui-component`,
`gpui-kit-assets` (Lucide icons), and `image` (reading image sizes on import).

`NotatusWindow` (`app/window.rs`) is the one view; its behavior is split into
modules under `src/app/` by feature:

- `actions.rs`: actions, key bindings, and their handlers
- `commands.rs`, `palette.rs`, `shortcuts.rs`: shared commands, the command
  palette, and the shortcut sheet
- `autosave.rs`, `project_commands.rs`, `project_session.rs`: saving, opening,
  and closing projects
- `media_import.rs`: importing files and folders
- `model_run.rs`, `runner_settings.rs`: running models and remembering each
  project's runner
- `review_queue.rs`: stepping through drafts across media, and its filter
- `export_commands/`: the export dialog and writing exports
- `canvas/`: drawing the image and overlays, hit testing, and mouse gestures
- `tools/`: tool definitions, in-progress drawing state, and the toolbar
- `menus.rs`: the application menus
- `dock.rs`, `left_dock/`, `right_dock/`, `status_bar.rs`, `titlebar.rs`,
  `label_picker.rs`, `view_options.rs`: the panels around the canvas
- `assets.rs`: icons beyond the default set
- `tests/`: tests that open the real window in GPUI's test platform

## Window Chrome

The shell uses `gpui-component::TitleBar`. It holds the application menus
(File, Edit, View, Annotate, Review, Model, Help) on the left, and the project
name and save state on the right, the one place the save state is shown. The
menus are gpui `Menu`s of actions: macOS shows them in its own menu bar, and
Linux and Windows draw them with `gpui-component::menu::AppMenuBar`. Each item
shows the key bound to its action. On Linux the window may draw its own
title bar; its close button goes through the same check as a compositor close
request: pending changes are saved first, and a project never saved to a
folder asks before it is discarded.

## Workflow Navigation

Each dock has a tab bar naming its views, with the current view's buttons at
its end: the left dock switches between Media and Labels, and the right dock
between Annotations and Details. Nothing has to happen in a fixed order:
media can be imported before labels exist, and drawing without a label creates
one.

Project persistence uses `notatus-storage::LocalProjectStore` through the
autosave pipeline described in [Storage](storage.md). A project can be unsaved
in memory or backed by a local folder.

## Status Bar

The status bar keeps the media navigation centered under the canvas: previous
and next, progress through the project, and marking the media done. Its left
side shows the last command's feedback; its right side shows a running model's
progress, with a stop button, and the zoom level, which fits the image when
clicked.

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

- Media, the default: the media list, with import buttons in the tab bar
- Labels: the label list, with an add button in the tab bar

The media list is gpui-component's virtualized `List`, so only the rows on
screen are built, however large the project. A search field above it matches
names and folders, and the filter button narrows it to media not done, done,
or with drafts to review. Each row starts with a green check when the media is
done or a hollow circle when it is not, then its name, the number of drafts
still to review, and its annotation count. The list marks the current media
and scrolls to it when it changes. Removing media, which also removes its
annotations, is in the row's right-click menu.

Label rows show their color and annotation count; renaming, recoloring, and
removing are in the row's options and right-click menus.

## Canvas Area

The center panel shows the selected image with GPUI's `img` element, fitted to
the canvas, which fills the whole panel; S3 media is not previewed yet. A
`gpui-component::toolbar::Toolbar` floats over its corner: the tools as one
group, the active label, display options, the model, and fit. Tooltips show
the keys bound to each command's action. Tool metadata and interaction state
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

## Model Suggestions

Ctrl+R runs the project's model runner on the current image and Ctrl+Shift+R
on every unfinished image; the sparkle button in the canvas toolbar does the
former. Suggestions arrive as drafts whose tags show the model's confidence,
ready for the Y/X review keys. See [Pre-Annotation](inference.md).

## Review Queue

The review queue is every draft in the project: media in list order, and each
media's annotations in the order they were made. N goes to the next draft and
Shift+N to the previous one, moving to its media, wrapping around at the end.
With no annotation selected, the queue starts at the current media.

Y and X review the selected annotation and move to the next draft on the same
media. When none is left there, the app stays on the image and the status bar
says so, so moving on is a choice: N goes on to the next media with drafts.

The "To review" section of the Annotations dock shows how many drafts are left,
with previous and next buttons and two filters: one label, and model
suggestions below a confidence (90%, 75%, 50%, or 25%), which leaves out shapes
a person drew. The filter also limits which draft Y and X move to. It lasts for
the session and resets when another project opens. The status bar shows the
count too.

## Right Docks

- Annotations, the default: the review queue, then the selected media's image
  labels, shown as tags in their label's color that can be removed, and its
  objects. Each object row shows its label, which opens a menu to change it,
  its shape and review state, and accept and reject toggles. Y accepts, X
  rejects, and U resets the selected annotation; accepting or rejecting moves
  to the next draft on the same media.
- Details: the project's name, location, and creation date, the dataset's
  totals, and the selected media's metadata.

## Layout

The main workspace uses `gpui-component` resizable panels:

- left dock panel
- center canvas
- right dock panel
- status bar

The panel group stores its state by element ID, so resizing is handled by the
component library.
