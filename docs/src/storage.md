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
  classifications.jsonl
  .notatus-recovery/      # only after a project needed repair on load
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

Media inside the project folder is stored with a path relative to it, using
`/` between folders, so the folder can be moved or shared together with its
images. Media elsewhere keeps an absolute path. Loading resolves relative
paths against wherever the project folder is now, so in memory every local
path is absolute; projects saved before this keep loading, and media inside
the folder becomes relative on their next save.

JSONL is used because asset lists can grow large and line-oriented files are
friendly to incremental tooling.

### `annotations.jsonl`

Stores one annotation record per line.

This is the file expected to grow the fastest in real annotation projects.

### `classifications.jsonl`

Stores one image classification record per line. Projects saved before
classifications existed have no such file and load with none.

## Save Flow

`LocalProjectStore::save_dataset()` never writes over a project file in place:

1. Validates the dataset.
2. Creates the project directory if needed.
3. Writes each file (classifications, annotations, assets, labels, manifest)
   to a hidden temporary sibling such as `.annotations.jsonl.<pid>.<n>.tmp`,
   flushes it, and syncs it to disk. Errors from the final flush are reported,
   so a full disk fails the save instead of truncating a file.
4. Only when every file is written, renames each temporary file over its
   original, then syncs the folder so the renames survive a crash.

If anything fails before the renames, the temporary files are deleted and the
previous save is left untouched. A crash between two renames can leave files
from two consecutive saves side by side; the repairing load below handles the
mismatched records that can cause.

In the desktop app, every save goes through one pipeline with at most one save
in flight, so Ctrl+S never writes at the same time as an autosave. Closing the
window waits for pending changes to reach disk, and asks before discarding a
project that was never saved to a folder. A failed save or autosave raises an
error notification that stays until dismissed, as well as the status message,
and is retried after the next edit.

## Load Flow

`LocalProjectStore::load_dataset()` is strict:

1. Reads the manifest.
2. Reads labels, defaulting to an empty list if the file does not exist.
3. Reads assets, annotations, and classifications, defaulting to empty lists
   if the files do not exist.
4. Validates the reconstructed dataset, failing on the first problem.

`LocalProjectStore::load_dataset_with_report()` is what the app uses to open
projects. A single bad record no longer makes a project impossible to open:

1. Reads the manifest and labels, which must parse, and checks the schema
   version is supported.
2. Reads the JSONL files line by line, skipping lines that are not valid JSON.
3. Skips duplicate ids, media with an invalid size, annotations and
   classifications that reference missing media or labels, and geometry
   outside its image. Invalid confidences are cleared and empty names replaced
   rather than dropping the record.
4. When anything was skipped or changed, copies the project files unchanged to
   `.notatus-recovery/<UTC timestamp>/` before returning, so the next save
   cannot lose them.
5. Returns the repaired dataset with a `LoadReport` listing every issue. The
   app shows the first few in a notification.

## Error Handling

`StorageError` preserves path context for I/O and JSON failures. JSONL parse
errors include the line number when available.
