# Development

## Requirements

- Rust `1.96.0` or newer
- Rust edition `2024`
- On Linux, the X11 keyboard development packages, which the desktop app
  links against:

  ```sh
  sudo apt install libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev
  ```

  Fontconfig is loaded at runtime (see `.cargo/config.toml`), so it needs no
  development package.

## Common Commands

Run all tests with default features:

```sh
cargo test --workspace
```

Run the UI crate without GPUI:

```sh
cargo run -p notatus-ui --no-default-features
```

Run the GPUI desktop shell:

```sh
cargo run -p notatus-ui
```

Check the GPUI desktop shell:

```sh
cargo check -p notatus-ui
```

Run Clippy for the default workspace:

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

Run Clippy for just the UI package:

```sh
cargo clippy -p notatus-ui -- -D warnings
```

## Formatting

Use:

```sh
cargo fmt --all
```

## Documentation

This documentation is an mdBook.

Install mdBook if needed:

```sh
cargo install mdbook
```

Serve the book locally:

```sh
mdbook serve docs
```

Build the static book:

```sh
mdbook build docs
```

## Testing Strategy

- `notatus-core`: geometry and dataset validation, serialization.
- `notatus-storage`: save and load roundtrips, atomic saves, lenient loading
  and recovery backups.
- `notatus-export`: YOLO import and export, COCO and classification export,
  filters, and finished-media selection.
- `notatus-infer`: the protocol, prediction clipping, and the runner process
  manager driven against `notatus-fake-runner` (crashes, bad output,
  timeouts), plus the Ultralytics example runner against a stub of the
  library.
- `notatus-ui` state: every `UiState` change, undo, and model suggestions.
- `notatus-ui` app: the real window in GPUI's test platform, driven by
  keyboard shortcuts, the command palette, mouse gestures, saving, closing,
  and running a model.

Screenshots of the window can be rendered offscreen for visual review. They
need a GPU adapter, so they are ignored by default:

```sh
NOTATUS_SCREENSHOT_DIR=/tmp/shots NOTATUS_SAMPLE_IMAGE=/path/to/image.png \
    cargo test -p notatus-ui screenshots -- --ignored --nocapture
```

To try model suggestions without a model, build the fake runner and choose
`target/debug/notatus-fake-runner` with "Choose model runner…" in the app:

```sh
cargo build -p notatus-infer --bin notatus-fake-runner
```
