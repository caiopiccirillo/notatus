# Desktop UI

The UI implementation lives in `crates/notatus-ui`.

The crate has two layers:

- `UiState`, which is renderer-independent and testable.
- `gpui_shell`, which is compiled by default through the `gpui-ui` feature.

## UI State

`UiState` wraps the canonical `Dataset` and tracks UI-specific state:

- active annotation tool
- selected asset
- selected annotation
- dirty flag

Implemented mutations:

- create a new project
- create UI state from an existing validated dataset
- set active tool
- mark saved
- add label
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
used by component controls such as the titlebar buttons.

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

## Command Bar

The command bar shows:

- project name
- image count
- selected image
- import status
- `+ Add images` command

Long selected-image names are truncated so the command bar remains usable on
narrow windows.

## Image Import

`+ Add images` opens the platform file picker through GPUI:

```rust
cx.prompt_for_paths(PathPromptOptions {
    files: true,
    directories: false,
    multiple: true,
    prompt: Some("Add images".into()),
})
```

For selected paths, the shell:

1. Reads image dimensions with `image::image_dimensions`.
2. Skips invalid images.
3. Skips duplicate local paths already in the dataset.
4. Adds valid images through `UiState::add_local_image_asset`.
5. Selects the most recently imported image.
6. Updates the import status text.

## Sidebar

The left sidebar uses `gpui-component` sidebar/menu primitives.

It contains:

- Images section
- imported image rows
- Labels section

Image rows are selectable and update `UiState::selected_asset`.

Long filenames are shortened with a middle truncation helper before rendering.
This keeps repeated screenshot-style filenames readable in a narrow panel.

## Canvas Area

The center panel previews the selected image with GPUI's `img` element:

- local images are loaded from their selected path
- object fit is `Contain`
- loading and fallback messages are provided
- S3 object previews currently show a not-implemented message

The canvas does not yet implement interactive drawing. The next major UI step is
to add coordinate mapping and bounding-box creation on top of this preview.

## Inspector

The right inspector shows:

- active tool
- selected image
- selected image dimensions
- annotation count
- draft review queue count

This is intentionally simple while the annotation workflow is still being built.

## Layout

The main workspace uses `gpui-component` resizable panels:

- left asset/navigation panel
- center canvas
- right inspector

The panel group stores its state by element ID, so resizing is handled by the
component library.
