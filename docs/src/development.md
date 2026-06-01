# Development

## Requirements

- Rust `1.96.0` or newer
- Rust edition `2024`
- Native GPUI dependencies for the desktop shell

## Common Commands

Run all non-GPUI tests:

```sh
cargo test --workspace
```

Run the UI crate without GPUI:

```sh
cargo run -p notatus-ui
```

Run the GPUI desktop shell:

```sh
cargo run -p notatus-ui --features gpui-ui
```

Check the GPUI desktop shell:

```sh
cargo check -p notatus-ui --features gpui-ui
```

Run Clippy for the default workspace:

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

Run Clippy for the GPUI shell:

```sh
cargo clippy -p notatus-ui --features gpui-ui -- -D warnings
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

The current tests focus on the stable domain and adapter layers:

- geometry validation
- dataset validation
- local project storage roundtrip
- YOLO import/export
- COCO export
- inference protocol roundtrip
- UI state mutation

The GPUI shell is currently verified by compiling the `gpui-ui` feature. Future
interactive canvas work should add more focused tests around coordinate mapping
and annotation creation.
