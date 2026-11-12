//! Opening projects that contain records the current schema rejects.
//!
//! A single bad record used to make a whole project impossible to open, and
//! every later save fail. Loading now sets such records aside, reports them,
//! and keeps an untouched copy of the files they came from.

use crate::{
    ANNOTATIONS_FILE, ASSETS_FILE, CLASSIFICATIONS_FILE, LABELS_FILE, MANIFEST_FILE, PROJECT_FILES,
    StorageError,
};
use notatus_core::{CURRENT_SCHEMA_VERSION, Dataset, ImageDimensions, ValidationError};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

/// Folder inside a project that keeps copies of files that needed repair.
pub const RECOVERY_DIR: &str = ".notatus-recovery";

const UNTITLED_PROJECT: &str = "Untitled project";
const UNNAMED_LABEL: &str = "Unnamed label";

/// What loading a project left out or changed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadReport {
    pub issues: Vec<LoadIssue>,
    /// Where the original files were copied, when there were issues.
    pub recovery_dir: Option<PathBuf>,
}

impl LoadReport {
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }
}

/// One record that was skipped or changed while loading.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadIssue {
    /// Project file the record came from.
    pub file: &'static str,
    /// 1-based line, for records that could not be read at all.
    pub line: Option<usize>,
    pub message: String,
}

impl fmt::Display for LoadIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{line}: {}", self.file, self.message),
            None => write!(f, "{}: {}", self.file, self.message),
        }
    }
}

/// Removes or fixes every record that would fail [`Dataset::validate`],
/// noting each change in `issues`. Records are dropped only when no safe fix
/// exists; an invalid confidence is cleared, an empty name replaced.
pub(crate) fn repair(
    dataset: &mut Dataset,
    issues: &mut Vec<LoadIssue>,
) -> Result<(), StorageError> {
    if dataset.manifest.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(ValidationError::UnsupportedSchemaVersion {
            found: dataset.manifest.schema_version,
            supported: CURRENT_SCHEMA_VERSION,
        }
        .into());
    }

    let mut issue = |file: &'static str, message: String| {
        issues.push(LoadIssue {
            file,
            line: None,
            message,
        });
    };

    if dataset.manifest.project.name.trim().is_empty() {
        issue(
            MANIFEST_FILE,
            format!("project had no name; renamed to \"{UNTITLED_PROJECT}\""),
        );
        dataset.manifest.project.name = UNTITLED_PROJECT.to_string();
    }

    let mut label_ids = BTreeSet::new();
    dataset.labels.retain_mut(|label| {
        if !label_ids.insert(label.id) {
            issue(LABELS_FILE, format!("skipped duplicate label {}", label.id));
            return false;
        }
        if label.name.trim().is_empty() {
            issue(
                LABELS_FILE,
                format!(
                    "label {} had no name; renamed to \"{UNNAMED_LABEL}\"",
                    label.id
                ),
            );
            label.name = UNNAMED_LABEL.to_string();
        }
        true
    });

    let mut asset_dimensions: BTreeMap<_, ImageDimensions> = BTreeMap::new();
    dataset.assets.retain(|asset| {
        if asset_dimensions.contains_key(&asset.id) {
            issue(ASSETS_FILE, format!("skipped duplicate media {}", asset.id));
            return false;
        }
        if asset.dimensions.width == 0 || asset.dimensions.height == 0 {
            issue(
                ASSETS_FILE,
                format!(
                    "skipped media {} with invalid size {}x{}",
                    asset.id, asset.dimensions.width, asset.dimensions.height
                ),
            );
            return false;
        }
        asset_dimensions.insert(asset.id, asset.dimensions);
        true
    });

    let mut annotation_ids = BTreeSet::new();
    dataset.annotations.retain_mut(|annotation| {
        let id = annotation.id;
        if !annotation_ids.insert(id) {
            issue(
                ANNOTATIONS_FILE,
                format!("skipped duplicate annotation {id}"),
            );
            return false;
        }
        let Some(dimensions) = asset_dimensions.get(&annotation.asset_id) else {
            issue(
                ANNOTATIONS_FILE,
                format!(
                    "skipped annotation {id}: media {} is missing",
                    annotation.asset_id
                ),
            );
            return false;
        };
        if !label_ids.contains(&annotation.label_id) {
            issue(
                ANNOTATIONS_FILE,
                format!(
                    "skipped annotation {id}: label {} is missing",
                    annotation.label_id
                ),
            );
            return false;
        }
        if let Err(error) = annotation.geometry.validate_within_image(*dimensions) {
            issue(
                ANNOTATIONS_FILE,
                format!("skipped annotation {id}: {error}"),
            );
            return false;
        }
        if annotation
            .confidence
            .is_some_and(|value| !valid_confidence(value))
        {
            issue(
                ANNOTATIONS_FILE,
                format!("cleared invalid confidence on annotation {id}"),
            );
            annotation.confidence = None;
        }
        true
    });

    let mut classification_ids = BTreeSet::new();
    dataset.classifications.retain_mut(|classification| {
        let id = classification.id;
        if !classification_ids.insert(id) {
            issue(
                CLASSIFICATIONS_FILE,
                format!("skipped duplicate classification {id}"),
            );
            return false;
        }
        if !asset_dimensions.contains_key(&classification.asset_id) {
            issue(
                CLASSIFICATIONS_FILE,
                format!(
                    "skipped classification {id}: media {} is missing",
                    classification.asset_id
                ),
            );
            return false;
        }
        if !label_ids.contains(&classification.label_id) {
            issue(
                CLASSIFICATIONS_FILE,
                format!(
                    "skipped classification {id}: label {} is missing",
                    classification.label_id
                ),
            );
            return false;
        }
        if classification
            .confidence
            .is_some_and(|value| !valid_confidence(value))
        {
            issue(
                CLASSIFICATIONS_FILE,
                format!("cleared invalid confidence on classification {id}"),
            );
            classification.confidence = None;
        }
        true
    });

    Ok(())
}

fn valid_confidence(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

/// Copies the project files as they are on disk into a new, timestamped
/// folder under [`RECOVERY_DIR`] and returns that folder.
pub(crate) fn back_up_project_files(root: &Path) -> Result<PathBuf, StorageError> {
    let parent = root.join(RECOVERY_DIR);
    fs::create_dir_all(&parent).map_err(|source| StorageError::Io {
        path: parent.clone(),
        source,
    })?;

    let now = OffsetDateTime::now_utc();
    let stamp = format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );
    let dir = unique_dir(&parent, &stamp)?;

    for name in PROJECT_FILES {
        let source_path = root.join(name);
        if !source_path.exists() {
            continue;
        }
        let target = dir.join(name);
        fs::copy(&source_path, &target).map_err(|source| StorageError::Io {
            path: target.clone(),
            source,
        })?;
    }
    tracing::warn!(dir = %dir.display(), "copied project files before repair");
    Ok(dir)
}

/// Creates `parent/stem`, or `parent/stem-2` and so on when it exists.
fn unique_dir(parent: &Path, stem: &str) -> Result<PathBuf, StorageError> {
    for attempt in 1.. {
        let name = if attempt == 1 {
            stem.to_string()
        } else {
            format!("{stem}-{attempt}")
        };
        let dir = parent.join(name);
        match fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(StorageError::Io { path: dir, source }),
        }
    }
    unreachable!("an unbounded range always yields another attempt")
}

#[cfg(test)]
mod tests {
    use crate::{
        ANNOTATIONS_FILE, ASSETS_FILE, LoadIssue, LocalProjectStore, ProjectStore, RECOVERY_DIR,
    };
    use notatus_core::{
        AnnotationGeometry, AnnotationRecord, AssetLocation, AssetRecord, BoundingBox, Dataset,
        LabelId,
    };
    use std::fs;

    fn saved_project() -> (tempfile::TempDir, LocalProjectStore, Dataset) {
        let temp = tempfile::tempdir().unwrap();
        let store = LocalProjectStore::new(temp.path());
        let mut dataset = store.initialize("demo").unwrap();
        let label_id = dataset.add_label("car");
        let asset = AssetRecord::new_image(AssetLocation::local("a.jpg"), 100, 100).unwrap();
        let asset_id = dataset.add_asset(asset);
        dataset.add_annotation(AnnotationRecord::new_human(
            asset_id,
            label_id,
            AnnotationGeometry::Bbox(BoundingBox::from_xywh(10.0, 10.0, 20.0, 20.0).unwrap()),
            None,
        ));
        store.save_dataset(&dataset).unwrap();
        (temp, store, dataset)
    }

    fn append_line(path: &std::path::Path, line: &str) {
        let mut contents = fs::read_to_string(path).unwrap();
        contents.push_str(line);
        contents.push('\n');
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn clean_project_loads_without_backup() {
        let (temp, store, dataset) = saved_project();

        let (loaded, report) = store.load_dataset_with_report().unwrap();

        assert_eq!(loaded, dataset);
        assert!(report.is_clean());
        assert_eq!(report.recovery_dir, None);
        assert!(!temp.path().join(RECOVERY_DIR).exists());
    }

    #[test]
    fn skips_bad_records_and_keeps_a_copy() {
        let (temp, store, dataset) = saved_project();
        let annotations_path = temp.path().join(ANNOTATIONS_FILE);
        let asset_id = dataset.assets[0].id;

        // A truncated line, an annotation for a missing label, and one that
        // lies outside its image.
        append_line(&annotations_path, "{\"id\":");
        let mut orphan = dataset.annotations[0].clone();
        orphan.id = notatus_core::AnnotationId::new();
        orphan.label_id = LabelId::new();
        append_line(&annotations_path, &serde_json::to_string(&orphan).unwrap());
        let mut outside = dataset.annotations[0].clone();
        outside.id = notatus_core::AnnotationId::new();
        outside.geometry =
            AnnotationGeometry::Bbox(BoundingBox::from_xywh(90.0, 90.0, 50.0, 50.0).unwrap());
        append_line(&annotations_path, &serde_json::to_string(&outside).unwrap());
        let damaged = fs::read(&annotations_path).unwrap();

        assert!(store.load_dataset().is_err());
        let (loaded, report) = store.load_dataset_with_report().unwrap();

        assert_eq!(loaded.annotations, dataset.annotations);
        assert_eq!(loaded.assets[0].id, asset_id);
        assert_eq!(report.issues.len(), 3);
        assert_eq!(
            report.issues[0],
            LoadIssue {
                file: ANNOTATIONS_FILE,
                line: Some(2),
                message: report.issues[0].message.clone(),
            }
        );
        assert!(report.issues[1].message.contains("label"));
        assert!(report.issues[2].message.contains("exceeds image bounds"));

        let backup = report.recovery_dir.expect("a backup folder");
        assert!(backup.starts_with(temp.path().join(RECOVERY_DIR)));
        assert_eq!(fs::read(backup.join(ANNOTATIONS_FILE)).unwrap(), damaged);

        // The repaired dataset saves, and loads cleanly afterwards.
        store.save_dataset(&loaded).unwrap();
        let (_, report) = store.load_dataset_with_report().unwrap();
        assert!(report.is_clean());
    }

    #[test]
    fn dropping_media_drops_its_annotations() {
        let (temp, store, dataset) = saved_project();
        let mut broken = dataset.assets[0].clone();
        broken.dimensions.width = 0;
        fs::write(
            temp.path().join(ASSETS_FILE),
            format!("{}\n", serde_json::to_string(&broken).unwrap()),
        )
        .unwrap();

        let (loaded, report) = store.load_dataset_with_report().unwrap();

        assert!(loaded.assets.is_empty());
        assert!(loaded.annotations.is_empty());
        assert_eq!(report.issues.len(), 2);
    }

    #[test]
    fn backups_never_overwrite_each_other() {
        let (temp, _, _) = saved_project();

        let first = super::back_up_project_files(temp.path()).unwrap();
        let second = super::back_up_project_files(temp.path()).unwrap();

        assert_ne!(first, second);
        assert!(first.join(crate::MANIFEST_FILE).exists());
        assert!(second.join(crate::MANIFEST_FILE).exists());
    }

    #[test]
    fn unsupported_schema_is_still_an_error() {
        let (temp, store, mut dataset) = saved_project();
        dataset.manifest.schema_version = 99;
        fs::write(
            temp.path().join(crate::MANIFEST_FILE),
            serde_json::to_string(&dataset.manifest).unwrap(),
        )
        .unwrap();

        assert!(store.load_dataset_with_report().is_err());
        assert!(!temp.path().join(RECOVERY_DIR).exists());
    }
}
