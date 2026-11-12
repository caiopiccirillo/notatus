//! Project storage backends for Notatus.
//!
//! The first backend is a transparent local folder layout. It keeps the
//! canonical dataset in deterministic JSON/JSONL files so projects can be
//! versioned with Git, DVC, lakeFS, object storage manifests, or similar tools.
//!
//! Saves never write over a project file in place. Every file is written to a
//! temporary sibling, flushed and synced, and only then renamed over the
//! original, so a crash or a full disk leaves the previous save readable.

use notatus_core::{
    AnnotationRecord, AssetRecord, ClassificationRecord, Dataset, Label, ProjectManifest,
};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

mod recovery;

pub use recovery::{LoadIssue, LoadReport, RECOVERY_DIR};

pub const MANIFEST_FILE: &str = "notatus.project.json";
pub const LABELS_FILE: &str = "labels.json";
pub const ASSETS_FILE: &str = "assets.jsonl";
pub const ANNOTATIONS_FILE: &str = "annotations.jsonl";
pub const CLASSIFICATIONS_FILE: &str = "classifications.jsonl";

/// Every file a project folder owns, in the order saves replace them.
pub const PROJECT_FILES: [&str; 5] = [
    CLASSIFICATIONS_FILE,
    ANNOTATIONS_FILE,
    ASSETS_FILE,
    LABELS_FILE,
    MANIFEST_FILE,
];

pub trait ProjectStore {
    fn load_dataset(&self) -> Result<Dataset, StorageError>;
    fn save_dataset(&self, dataset: &Dataset) -> Result<(), StorageError>;
}

#[derive(Clone, Debug)]
pub struct LocalProjectStore {
    root: PathBuf,
}

impl LocalProjectStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        tracing::debug!(root = %root.display(), "creating local project store");
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    #[tracing::instrument(skip_all, fields(name = tracing::field::Empty))]
    pub fn initialize(&self, name: impl Into<String>) -> Result<Dataset, StorageError> {
        let name = name.into();
        tracing::debug!(name, "initializing project");
        let dataset = Dataset::new(name);
        self.save_dataset(&dataset)?;
        Ok(dataset)
    }

    /// Loads a project, setting aside records that cannot be used instead of
    /// refusing to open it.
    ///
    /// Lines that are not valid JSON, duplicate ids, references to missing
    /// labels or media, and geometry outside the image are left out of the
    /// returned dataset and listed in the report. When anything was left out,
    /// the project files are first copied, unchanged, into a folder under
    /// [`RECOVERY_DIR`], so the next save cannot lose them.
    ///
    /// The manifest and the label file must still parse, and the schema
    /// version must be supported, because nothing else can be read without
    /// them.
    #[tracing::instrument(level = "debug", skip_all, fields(root = %self.root.display()))]
    pub fn load_dataset_with_report(&self) -> Result<(Dataset, LoadReport), StorageError> {
        tracing::info!(root = %self.root.display(), "loading dataset with repair");

        let manifest: ProjectManifest = read_json(&self.manifest_path())?;
        let labels: Vec<Label> = read_json_or_default(&self.labels_path())?;
        let mut issues = Vec::new();
        let assets = read_jsonl_lenient(&self.assets_path(), ASSETS_FILE, &mut issues)?;
        let annotations =
            read_jsonl_lenient(&self.annotations_path(), ANNOTATIONS_FILE, &mut issues)?;
        let classifications = read_jsonl_lenient(
            &self.classifications_path(),
            CLASSIFICATIONS_FILE,
            &mut issues,
        )?;

        let mut dataset = Dataset {
            manifest,
            labels,
            assets,
            annotations,
            classifications,
        };
        recovery::repair(&mut dataset, &mut issues)?;
        dataset.validate()?;

        let recovery_dir = if issues.is_empty() {
            None
        } else {
            tracing::warn!(issues = issues.len(), "project loaded with skipped records");
            Some(recovery::back_up_project_files(&self.root)?)
        };

        tracing::info!(
            labels = dataset.labels.len(),
            assets = dataset.assets.len(),
            annotations = dataset.annotations.len(),
            classifications = dataset.classifications.len(),
            issues = issues.len(),
            "dataset loaded"
        );
        Ok((
            dataset,
            LoadReport {
                issues,
                recovery_dir,
            },
        ))
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }

    fn labels_path(&self) -> PathBuf {
        self.root.join(LABELS_FILE)
    }

    fn assets_path(&self) -> PathBuf {
        self.root.join(ASSETS_FILE)
    }

    fn annotations_path(&self) -> PathBuf {
        self.root.join(ANNOTATIONS_FILE)
    }

    fn classifications_path(&self) -> PathBuf {
        self.root.join(CLASSIFICATIONS_FILE)
    }
}

impl ProjectStore for LocalProjectStore {
    #[tracing::instrument(level = "debug", skip_all, fields(root = %self.root.display()))]
    fn load_dataset(&self) -> Result<Dataset, StorageError> {
        tracing::info!(root = %self.root.display(), "loading dataset from disk");

        let manifest: ProjectManifest = read_json(&self.manifest_path())?;
        let labels: Vec<Label> = read_json_or_default(&self.labels_path())?;
        let assets: Vec<AssetRecord> = read_jsonl_or_default(&self.assets_path())?;
        let annotations: Vec<AnnotationRecord> = read_jsonl_or_default(&self.annotations_path())?;
        let classifications: Vec<ClassificationRecord> =
            read_jsonl_or_default(&self.classifications_path())?;

        let dataset = Dataset {
            manifest,
            labels,
            assets,
            annotations,
            classifications,
        };
        dataset.validate()?;

        tracing::info!(
            labels = dataset.labels.len(),
            assets = dataset.assets.len(),
            annotations = dataset.annotations.len(),
            classifications = dataset.classifications.len(),
            "dataset loaded successfully"
        );
        Ok(dataset)
    }

    /// Writes every project file to a synced temporary sibling first, and
    /// renames them over the originals only once all of them are written.
    /// A failure before the renames leaves the previous save untouched.
    #[tracing::instrument(level = "debug", skip_all, fields(
        root = %self.root.display(),
        labels = dataset.labels.len(),
        assets = dataset.assets.len(),
        annotations = dataset.annotations.len(),
        classifications = dataset.classifications.len(),
    ))]
    fn save_dataset(&self, dataset: &Dataset) -> Result<(), StorageError> {
        tracing::info!(root = %self.root.display(), "saving dataset to disk");

        dataset.validate()?;
        fs::create_dir_all(&self.root).map_err(|source| StorageError::Io {
            path: self.root.clone(),
            source,
        })?;

        let mut staged = StagedFiles::default();
        staged.stage(&self.classifications_path(), |writer| {
            write_jsonl(writer, &dataset.classifications)
        })?;
        staged.stage(&self.annotations_path(), |writer| {
            write_jsonl(writer, &dataset.annotations)
        })?;
        staged.stage(&self.assets_path(), |writer| {
            write_jsonl(writer, &dataset.assets)
        })?;
        staged.stage(&self.labels_path(), |writer| {
            write_json_pretty(writer, &dataset.labels)
        })?;
        staged.stage(&self.manifest_path(), |writer| {
            write_json_pretty(writer, &dataset.manifest)
        })?;
        staged.commit(&self.root)?;

        tracing::info!("dataset saved successfully");
        Ok(())
    }
}

/// Temporary files written for one save, renamed into place by [`commit`].
/// Dropping it without committing deletes them.
///
/// [`commit`]: StagedFiles::commit
#[derive(Default)]
struct StagedFiles {
    files: Vec<(PathBuf, PathBuf)>,
}

impl StagedFiles {
    /// Writes `path`'s new contents to a synced temporary sibling.
    fn stage(
        &mut self,
        path: &Path,
        write: impl FnOnce(&mut BufWriter<File>) -> Result<(), WriteError>,
    ) -> Result<(), StorageError> {
        let temp = temp_path_for(path);
        let result = write_synced(&temp, write);
        if let Err(error) = result {
            let _ = fs::remove_file(&temp);
            return Err(error.at(path));
        }
        self.files.push((temp, path.to_path_buf()));
        Ok(())
    }

    /// Renames every staged file over its target, then syncs the folder so
    /// the renames themselves survive a crash.
    fn commit(mut self, root: &Path) -> Result<(), StorageError> {
        let files = std::mem::take(&mut self.files);
        let mut pending = files.iter();
        for (temp, target) in pending.by_ref() {
            if let Err(source) = fs::rename(temp, target) {
                for (rest, _) in pending {
                    let _ = fs::remove_file(rest);
                }
                let _ = fs::remove_file(temp);
                return Err(StorageError::Io {
                    path: target.clone(),
                    source,
                });
            }
        }
        sync_dir(root)
    }
}

impl Drop for StagedFiles {
    fn drop(&mut self) {
        for (temp, _) in &self.files {
            let _ = fs::remove_file(temp);
        }
    }
}

/// A hidden sibling of `path` that no other save in this process uses.
fn temp_path_for(path: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!(".{name}.{}.{unique}.tmp", std::process::id()))
}

/// Creates `path`, writes it, and makes sure every byte reached the disk.
/// Unlike dropping a `BufWriter`, this reports errors from the final flush.
fn write_synced(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> Result<(), WriteError>,
) -> Result<(), WriteError> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write(&mut writer)?;
    let file = writer.into_inner().map_err(|error| error.into_error())?;
    file.sync_all()?;
    Ok(())
}

#[cfg(unix)]
fn sync_dir(dir: &Path) -> Result<(), StorageError> {
    File::open(dir)
        .and_then(|dir| dir.sync_all())
        .map_err(|source| StorageError::Io {
            path: dir.to_path_buf(),
            source,
        })
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> Result<(), StorageError> {
    // Folders cannot be opened for syncing on Windows; renames are already
    // durable there once the file contents are flushed.
    Ok(())
}

/// A write failure before the file path is known to the caller.
#[derive(Debug)]
enum WriteError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl WriteError {
    fn at(self, path: &Path) -> StorageError {
        let path = path.to_path_buf();
        match self {
            Self::Io(source) => StorageError::Io { path, source },
            Self::Json(source) => StorageError::Json {
                path,
                line: None,
                source,
            },
        }
    }
}

impl From<std::io::Error> for WriteError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl From<serde_json::Error> for WriteError {
    fn from(source: serde_json::Error) -> Self {
        Self::Json(source)
    }
}

#[tracing::instrument(level = "debug", skip_all, fields(path = %path.display()))]
fn read_json<T>(path: &Path) -> Result<T, StorageError>
where
    T: serde::de::DeserializeOwned,
{
    tracing::debug!(path = %path.display(), "reading JSON file");

    let file = File::open(path).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    serde_json::from_reader(BufReader::new(file)).map_err(|source| StorageError::Json {
        path: path.to_path_buf(),
        line: None,
        source,
    })
}

#[tracing::instrument(level = "debug", skip_all, fields(path = %path.display()))]
fn read_json_or_default<T>(path: &Path) -> Result<T, StorageError>
where
    T: serde::de::DeserializeOwned + Default,
{
    if !path.exists() {
        tracing::debug!(path = %path.display(), "file does not exist, returning default");
        return Ok(T::default());
    }

    read_json(path)
}

#[tracing::instrument(level = "debug", skip_all, fields(path = %path.display()))]
fn read_jsonl_or_default<T>(path: &Path) -> Result<Vec<T>, StorageError>
where
    T: serde::de::DeserializeOwned,
{
    let mut values = Vec::new();
    for_each_jsonl_line(path, |line_number, line| {
        let value = serde_json::from_str(line).map_err(|source| StorageError::Json {
            path: path.to_path_buf(),
            line: Some(line_number),
            source,
        })?;
        values.push(value);
        Ok(())
    })?;
    tracing::debug!(count = values.len(), "read JSONL records");
    Ok(values)
}

/// Reads a JSONL file, skipping lines that do not parse and listing them in
/// `issues` instead of failing.
#[tracing::instrument(level = "debug", skip_all, fields(path = %path.display()))]
fn read_jsonl_lenient<T>(
    path: &Path,
    file: &'static str,
    issues: &mut Vec<LoadIssue>,
) -> Result<Vec<T>, StorageError>
where
    T: serde::de::DeserializeOwned,
{
    let mut values = Vec::new();
    for_each_jsonl_line(path, |line_number, line| {
        match serde_json::from_str(line) {
            Ok(value) => values.push(value),
            Err(error) => issues.push(LoadIssue {
                file,
                line: Some(line_number),
                message: format!("unreadable record: {error}"),
            }),
        }
        Ok(())
    })?;
    Ok(values)
}

/// Calls `visit` with the 1-based number and text of every non-blank line.
/// A missing file has no lines.
fn for_each_jsonl_line(
    path: &Path,
    mut visit: impl FnMut(usize, &str) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    if !path.exists() {
        tracing::debug!(path = %path.display(), "file does not exist, returning empty");
        return Ok(());
    }

    tracing::debug!(path = %path.display(), "reading JSONL file");
    let file = File::open(path).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|source| StorageError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if !line.trim().is_empty() {
            visit(line_index + 1, &line)?;
        }
    }
    Ok(())
}

fn write_json_pretty<T>(writer: &mut BufWriter<File>, value: &T) -> Result<(), WriteError>
where
    T: serde::Serialize,
{
    serde_json::to_writer_pretty(&mut *writer, value)?;
    Ok(())
}

fn write_jsonl<T>(writer: &mut BufWriter<File>, values: &[T]) -> Result<(), WriteError>
where
    T: serde::Serialize,
{
    for value in values {
        serde_json::to_writer(&mut *writer, value)?;
        writeln!(writer)?;
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("JSON error at {path}{line_text}: {source}", line_text = line.map(|line| format!(":{line}")).unwrap_or_default())]
    Json {
        path: PathBuf,
        line: Option<usize>,
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    Validation(#[from] notatus_core::ValidationError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use notatus_core::{
        AnnotationGeometry, AnnotationRecord, AssetLocation, AssetRecord, BoundingBox,
    };

    fn sample_dataset(store: &LocalProjectStore) -> Dataset {
        let mut dataset = store.initialize("demo").unwrap();
        let label_id = dataset.add_label("vehicle");
        let asset =
            AssetRecord::new_image(AssetLocation::local("images/frame.jpg"), 320, 240).unwrap();
        let asset_id = dataset.add_asset(asset);
        let mut annotation = AnnotationRecord::new_human(
            asset_id,
            label_id,
            AnnotationGeometry::Bbox(BoundingBox::from_xywh(20.0, 30.0, 50.0, 60.0).unwrap()),
            None,
        );
        annotation.accept();
        dataset.add_annotation(annotation);
        dataset
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn roundtrips_local_project_folder() {
        let temp = tempfile::tempdir().unwrap();
        let store = LocalProjectStore::new(temp.path());
        let dataset = sample_dataset(&store);

        store.save_dataset(&dataset).unwrap();
        let loaded = store.load_dataset().unwrap();

        assert_eq!(loaded, dataset);
        assert!(temp.path().join(MANIFEST_FILE).exists());
        assert!(temp.path().join(ASSETS_FILE).exists());
    }

    #[test]
    fn save_leaves_no_temporary_files() {
        let temp = tempfile::tempdir().unwrap();
        let store = LocalProjectStore::new(temp.path());
        let dataset = sample_dataset(&store);

        store.save_dataset(&dataset).unwrap();
        store.save_dataset(&dataset).unwrap();

        let mut expected: Vec<_> = PROJECT_FILES.iter().map(|name| name.to_string()).collect();
        expected.sort();
        assert_eq!(file_names(temp.path()), expected);
    }

    #[test]
    fn failed_save_keeps_previous_files() {
        let temp = tempfile::tempdir().unwrap();
        let store = LocalProjectStore::new(temp.path());
        let dataset = sample_dataset(&store);
        store.save_dataset(&dataset).unwrap();
        let before: Vec<_> = PROJECT_FILES
            .iter()
            .map(|name| fs::read(temp.path().join(name)).unwrap())
            .collect();

        // Writing the second file fails halfway through, after the first
        // file was already staged.
        let mut staged = StagedFiles::default();
        staged
            .stage(&temp.path().join(CLASSIFICATIONS_FILE), |writer| {
                writer.write_all(b"[]")?;
                Ok(())
            })
            .unwrap();
        let error = staged
            .stage(&temp.path().join(ANNOTATIONS_FILE), |writer| {
                writer.write_all(b"{\"partial\":")?;
                Err(std::io::Error::other("disk full").into())
            })
            .unwrap_err();
        drop(staged);

        assert!(error.to_string().contains("disk full"));
        let after: Vec<_> = PROJECT_FILES
            .iter()
            .map(|name| fs::read(temp.path().join(name)).unwrap())
            .collect();
        assert_eq!(after, before);
        assert_eq!(file_names(temp.path()).len(), PROJECT_FILES.len());
        assert_eq!(store.load_dataset().unwrap(), dataset);
    }

    #[test]
    fn strict_load_rejects_unreadable_lines() {
        let temp = tempfile::tempdir().unwrap();
        let store = LocalProjectStore::new(temp.path());
        let dataset = sample_dataset(&store);
        store.save_dataset(&dataset).unwrap();
        let path = temp.path().join(ANNOTATIONS_FILE);
        let mut contents = fs::read_to_string(&path).unwrap();
        contents.push_str("{\"truncated\n");
        fs::write(&path, contents).unwrap();

        assert!(matches!(
            store.load_dataset(),
            Err(StorageError::Json { line: Some(2), .. })
        ));
    }
}
