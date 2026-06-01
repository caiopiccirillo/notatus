# Storage

The storage implementation lives in `crates/notatus-storage`.

## Store Trait

Storage backends implement:

```rust
pub trait ProjectStore {
    fn load_dataset(&self) -> Result<Dataset, StorageError>;
    fn save_dataset(&self, dataset: &Dataset) -> Result<(), StorageError>;
}
```

This keeps the UI and future remote backends independent from the persistence
mechanism.

## Local Project Store

`LocalProjectStore` stores a project in a directory.

Project layout:

```text
project-root/
  notatus.project.json
  labels.json
  assets.jsonl
  annotations.jsonl
```

## File Responsibilities

### `notatus.project.json`

Stores the project manifest:

- schema version
- project metadata

### `labels.json`

Stores all labels as a pretty-printed JSON array.

### `assets.jsonl`

Stores one asset record per line.

JSONL is used because asset lists can grow large and line-oriented files are
friendly to incremental tooling.

### `annotations.jsonl`

Stores one annotation record per line.

This is the file expected to grow the fastest in real annotation projects.

## Save Flow

`LocalProjectStore::save_dataset()`:

1. Validates the dataset.
2. Creates the project directory if needed.
3. Writes the manifest.
4. Writes labels.
5. Writes assets.
6. Writes annotations.

## Load Flow

`LocalProjectStore::load_dataset()`:

1. Reads the manifest.
2. Reads labels, defaulting to an empty list if the file does not exist.
3. Reads assets, defaulting to an empty list if the file does not exist.
4. Reads annotations, defaulting to an empty list if the file does not exist.
5. Validates the reconstructed dataset.

## Error Handling

`StorageError` preserves path context for I/O and JSON failures. JSONL parse
errors include the line number when available.
