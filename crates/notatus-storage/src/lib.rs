//! Project storage backends for Notatus.
//!
//! The first backend is a transparent local folder layout. It keeps the
//! canonical dataset in deterministic JSON/JSONL files so projects can be
//! versioned with Git, DVC, lakeFS, object storage manifests, or similar tools.

use notatus_core::{
    AnnotationRecord, AssetRecord, ClassificationRecord, Dataset, Label, ProjectManifest,
};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const MANIFEST_FILE: &str = "notatus.project.json";
pub const LABELS_FILE: &str = "labels.json";
pub const ASSETS_FILE: &str = "assets.jsonl";
pub const ANNOTATIONS_FILE: &str = "annotations.jsonl";
pub const CLASSIFICATIONS_FILE: &str = "classifications.jsonl";

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

        write_json_pretty(&self.manifest_path(), &dataset.manifest)?;
        write_json_pretty(&self.labels_path(), &dataset.labels)?;
        write_jsonl(&self.assets_path(), &dataset.assets)?;
        write_jsonl(&self.annotations_path(), &dataset.annotations)?;
        write_jsonl(&self.classifications_path(), &dataset.classifications)?;

        tracing::info!("dataset saved successfully");
        Ok(())
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

    serde_json::from_reader(file).map_err(|source| StorageError::Json {
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
    if !path.exists() {
        tracing::debug!(path = %path.display(), "file does not exist, returning empty");
        return Ok(Vec::new());
    }

    tracing::debug!(path = %path.display(), "reading JSONL file");

    let file = File::open(path).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let reader = BufReader::new(file);
    let mut values = Vec::new();

    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|source| StorageError::Io {
            path: path.to_path_buf(),
            source,
        })?;

        if line.trim().is_empty() {
            continue;
        }

        let value = serde_json::from_str(&line).map_err(|source| StorageError::Json {
            path: path.to_path_buf(),
            line: Some(line_index + 1),
            source,
        })?;
        values.push(value);
    }

    tracing::debug!(count = values.len(), "read JSONL records");
    Ok(values)
}

#[tracing::instrument(level = "debug", skip_all, fields(path = %path.display()))]
fn write_json_pretty<T>(path: &Path, value: &T) -> Result<(), StorageError>
where
    T: serde::Serialize,
{
    tracing::debug!(path = %path.display(), "writing JSON file");

    let file = File::create(path).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::to_writer_pretty(BufWriter::new(file), value).map_err(|source| StorageError::Json {
        path: path.to_path_buf(),
        line: None,
        source,
    })
}

#[tracing::instrument(level = "debug", skip_all, fields(path = %path.display(), count = values.len()))]
fn write_jsonl<T>(path: &Path, values: &[T]) -> Result<(), StorageError>
where
    T: serde::Serialize,
{
    tracing::debug!(path = %path.display(), count = values.len(), "writing JSONL file");

    let file = File::create(path).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut writer = BufWriter::new(file);

    for value in values {
        serde_json::to_writer(&mut writer, value).map_err(|source| StorageError::Json {
            path: path.to_path_buf(),
            line: None,
            source,
        })?;
        writeln!(writer).map_err(|source| StorageError::Io {
            path: path.to_path_buf(),
            source,
        })?;
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

    #[test]
    fn roundtrips_local_project_folder() {
        let temp = tempfile::tempdir().unwrap();
        let store = LocalProjectStore::new(temp.path());
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

        store.save_dataset(&dataset).unwrap();
        let loaded = store.load_dataset().unwrap();

        assert_eq!(loaded.labels.len(), 1);
        assert_eq!(loaded.assets.len(), 1);
        assert_eq!(loaded.annotations.len(), 1);
        assert!(temp.path().join(MANIFEST_FILE).exists());
        assert!(temp.path().join(ASSETS_FILE).exists());
    }
}
