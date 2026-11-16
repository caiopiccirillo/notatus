use crate::{AnnotationFilter, ExportError};
use notatus_core::{
    AnnotationGeometry, AnnotationRecord, AssetId, AssetLocation, AssetRecord, BoundingBox,
    Dataset, DatasetSplit, ImageDimensions, LabelId,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct YoloAnnotationFile {
    pub asset_id: AssetId,
    pub image_path: String,
    pub contents: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct YoloImportFile {
    pub asset_id: AssetId,
    pub contents: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct YoloExportSummary {
    pub class_count: usize,
    pub annotation_file_count: usize,
    pub annotation_count: usize,
    /// Polygons have no place in a detection export and are left out.
    pub skipped_polygons: usize,
}

#[tracing::instrument(level = "debug", skip_all, fields(
    assets = dataset.assets.len(),
    annotations = dataset.annotations.len(),
))]
pub fn export_detection(
    dataset: &Dataset,
    filter: &AnnotationFilter,
) -> Result<Vec<YoloAnnotationFile>, ExportError> {
    dataset.validate()?;
    let class_map = yolo_class_map(dataset);
    let mut files = Vec::with_capacity(dataset.assets.len());

    for asset in &dataset.assets {
        let mut lines = Vec::new();

        for annotation in dataset
            .annotations
            .iter()
            .filter(|annotation| annotation.asset_id == asset.id)
            .filter(|annotation| filter.accepts(annotation))
        {
            let Some(bbox) = annotation.geometry.as_bbox() else {
                continue;
            };
            let class_index =
                class_map
                    .get(&annotation.label_id)
                    .copied()
                    .ok_or(ExportError::UnknownLabel {
                        label_id: annotation.label_id,
                    })?;
            let [center_x, center_y, width, height] = bbox.to_yolo_normalized(asset.dimensions);
            lines.push(format!(
                "{class_index} {center_x:.6} {center_y:.6} {width:.6} {height:.6}"
            ));
        }

        files.push(YoloAnnotationFile {
            asset_id: asset.id,
            image_path: asset.location.display_path().into_owned(),
            contents: lines.join("\n"),
        });
    }

    tracing::info!(file_count = files.len(), "YOLO detection export complete");
    Ok(files)
}

/// Writes a dataset Ultralytics can train on directly:
///
/// ```text
/// output_dir/
///   data.yaml              class names and split folders
///   classes.txt            class names, one per line
///   images/<split>/...     copies of the images
///   labels/<split>/...     one .txt per image, named after it
/// ```
///
/// Media without a split goes to `train`. The export is built in a hidden
/// sibling folder and swapped in at the end, so a failure leaves any earlier
/// export intact and no labels from an earlier export are left behind. An
/// existing `output_dir` is only replaced when Notatus made it.
#[tracing::instrument(level = "info", skip_all, fields(output_dir = %output_dir.as_ref().display()))]
pub fn write_detection_export(
    dataset: &Dataset,
    filter: &AnnotationFilter,
    output_dir: impl AsRef<Path>,
) -> Result<YoloExportSummary, ExportError> {
    let output_dir = output_dir.as_ref();
    let files = export_detection(dataset, filter)?;
    let staging = StagingDir::create_beside(output_dir)?;

    write_file(&staging.path.join(EXPORT_MARKER), EXPORT_MARKER_CONTENTS)?;
    write_file(
        &staging.path.join("classes.txt"),
        &classes_file_contents(dataset),
    )?;

    let mut used_names: BTreeSet<(YoloSplit, String)> = BTreeSet::new();
    let mut splits_used = BTreeSet::new();
    let mut annotation_count = 0;
    for (asset, file) in dataset.assets.iter().zip(&files) {
        debug_assert_eq!(asset.id, file.asset_id);
        let split = YoloSplit::of(&asset.split);
        splits_used.insert(split);
        let source = local_image_path(asset)?;
        let image_name = unique_image_name(&source, asset.id, split, &mut used_names);
        let image_dir = staging.path.join("images").join(split.dir());
        let label_dir = staging.path.join("labels").join(split.dir());
        create_dir_all(&image_dir)?;
        create_dir_all(&label_dir)?;

        // A copy, not a hard link: tools that edit exported images in place
        // would otherwise change the originals too. Copies are cheap clones
        // on file systems that support them.
        ensure_image_file(&source)
            .and_then(|()| fs::copy(&source, image_dir.join(&image_name)))
            .map_err(|source_error| ExportError::Image {
                asset_id: asset.id,
                path: source.clone(),
                source: source_error,
            })?;
        let stem = Path::new(&image_name)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| asset.id.to_string());
        let mut contents = file.contents.clone();
        if !contents.is_empty() {
            contents.push('\n');
        }
        write_file(&label_dir.join(format!("{stem}.txt")), &contents)?;
        annotation_count += file.contents.lines().count();
    }

    let final_dir = absolute(output_dir);
    write_file(
        &staging.path.join("data.yaml"),
        &data_yaml(dataset, &final_dir, &splits_used),
    )?;
    staging.replace(output_dir)?;

    let skipped_polygons = dataset
        .annotations
        .iter()
        .filter(|annotation| filter.accepts(annotation))
        .filter(|annotation| annotation.geometry.as_bbox().is_none())
        .count();
    let summary = YoloExportSummary {
        class_count: dataset.labels.len(),
        annotation_file_count: files.len(),
        annotation_count,
        skipped_polygons,
    };

    tracing::info!(
        class_count = summary.class_count,
        file_count = summary.annotation_file_count,
        annotation_count = summary.annotation_count,
        skipped_polygons = summary.skipped_polygons,
        "YOLO detection export written"
    );
    Ok(summary)
}

/// Name of the file that marks a folder as a Notatus export, so it may be
/// replaced by the next export.
pub const EXPORT_MARKER: &str = ".notatus-export";
const EXPORT_MARKER_CONTENTS: &str =
    "This folder was written by Notatus and is replaced by the next export.\n";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum YoloSplit {
    Train,
    Val,
    Test,
}

impl YoloSplit {
    fn of(split: &DatasetSplit) -> Self {
        match split {
            DatasetSplit::Train | DatasetSplit::Unassigned => Self::Train,
            DatasetSplit::Validation => Self::Val,
            DatasetSplit::Test => Self::Test,
        }
    }

    fn dir(self) -> &'static str {
        match self {
            Self::Train => "train",
            Self::Val => "val",
            Self::Test => "test",
        }
    }
}

fn local_image_path(asset: &AssetRecord) -> Result<PathBuf, ExportError> {
    match &asset.location {
        AssetLocation::LocalPath { path } => Ok(PathBuf::from(path)),
        location => Err(ExportError::UnsupportedLocation {
            asset_id: asset.id,
            location: location.display_path().into_owned(),
        }),
    }
}

/// The image's own file name, or with a short id before the extension when
/// another image in the same split already uses it.
fn unique_image_name(
    source: &Path,
    asset_id: AssetId,
    split: YoloSplit,
    used: &mut BTreeSet<(YoloSplit, String)>,
) -> String {
    let file_name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| asset_id.to_string());
    // Labels are matched to images by stem, so stems must be unique too.
    let stem = Path::new(&file_name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    if used.insert((split, stem.clone())) {
        return file_name;
    }
    let id = asset_id.as_uuid().simple().to_string();
    let suffix = &id[id.len() - 12..];
    let unique_stem = format!("{stem}-{suffix}");
    used.insert((split, unique_stem.clone()));
    match Path::new(&file_name).extension() {
        Some(extension) => format!("{unique_stem}.{}", extension.to_string_lossy()),
        None => unique_stem,
    }
}

/// Checks that `path` is a regular file that starts like an image.
///
/// A project can come from someone else, and its media paths are only data:
/// without this check, a path pointing at a private file would copy that
/// file into an export folder the user may then share.
fn ensure_image_file(path: &Path) -> std::io::Result<()> {
    if !fs::metadata(path)?.is_file() {
        return Err(std::io::Error::other("not a regular file"));
    }
    let mut header = [0_u8; 12];
    let read = std::io::Read::read(&mut fs::File::open(path)?, &mut header)?;
    if has_image_signature(&header[..read]) {
        Ok(())
    } else {
        Err(std::io::Error::other("not a recognized image file"))
    }
}

/// Whether `header` starts with the signature of an image format the app
/// imports: JPEG, PNG, GIF, BMP, WebP, or TIFF.
fn has_image_signature(header: &[u8]) -> bool {
    header.starts_with(&[0xFF, 0xD8, 0xFF])
        || header.starts_with(b"\x89PNG\r\n\x1a\n")
        || header.starts_with(b"GIF87a")
        || header.starts_with(b"GIF89a")
        || header.starts_with(b"BM")
        || (header.starts_with(b"RIFF") && header.get(8..12) == Some(b"WEBP"))
        || header.starts_with(b"II*\0")
        || header.starts_with(b"MM\0*")
}

fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

fn data_yaml(dataset: &Dataset, root: &Path, splits: &BTreeSet<YoloSplit>) -> String {
    let mut yaml =
        String::from("# Written by Notatus. Train with: yolo detect train data=<this file>\n");
    // An absolute path, because Ultralytics resolves a relative one against
    // its own datasets folder rather than this file. Edit it after moving
    // the export.
    yaml.push_str(&format!("path: {}\n", yaml_string(&root.to_string_lossy())));
    yaml.push_str("train: images/train\n");
    if splits.contains(&YoloSplit::Val) {
        yaml.push_str("val: images/val\n");
    } else {
        yaml.push_str("# No media is in the validation split, so validation reuses train.\n");
        yaml.push_str("val: images/train\n");
    }
    if splits.contains(&YoloSplit::Test) {
        yaml.push_str("test: images/test\n");
    }
    yaml.push_str("names:\n");
    for (index, label) in dataset.labels.iter().enumerate() {
        yaml.push_str(&format!("  {index}: {}\n", yaml_string(&label.name)));
    }
    yaml
}

/// A double-quoted YAML scalar.
fn yaml_string(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for ch in value.chars() {
        match ch {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\t' => quoted.push_str("\\t"),
            ch if ch.is_control() => quoted.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

/// A hidden folder next to the export target, deleted unless swapped in.
struct StagingDir {
    path: PathBuf,
}

impl StagingDir {
    fn create_beside(target: &Path) -> Result<Self, ExportError> {
        let parent = target
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        create_dir_all(parent)?;
        let name = target
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "export".to_string());
        let path = parent.join(format!(".{name}.notatus-tmp-{}", std::process::id()));
        if path.exists() {
            fs::remove_dir_all(&path).map_err(|source| ExportError::Io {
                path: path.clone(),
                source,
            })?;
        }
        create_dir_all(&path)?;
        Ok(Self { path })
    }

    /// Moves the staged export to `target`, replacing an earlier export
    /// there. Refuses to replace a folder Notatus did not write.
    fn replace(self, target: &Path) -> Result<(), ExportError> {
        if let Ok(metadata) = fs::symlink_metadata(target) {
            let ours = metadata.is_dir() && target.join(EXPORT_MARKER).is_file();
            let empty = metadata.is_dir()
                && fs::read_dir(target).is_ok_and(|mut entries| entries.next().is_none());
            if !ours && !empty {
                return Err(ExportError::OutputExists {
                    path: target.to_path_buf(),
                });
            }
            fs::remove_dir_all(target).map_err(|source| ExportError::Io {
                path: target.to_path_buf(),
                source,
            })?;
        }
        fs::rename(&self.path, target).map_err(|source| ExportError::Io {
            path: target.to_path_buf(),
            source,
        })
        // `self` drops here; the staged folder is gone, so nothing to clean.
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[tracing::instrument(level = "debug", skip_all, fields(file_count = files.len()))]
pub fn import_detection(
    dataset: &Dataset,
    files: &[YoloImportFile],
) -> Result<Vec<AnnotationRecord>, ExportError> {
    let mut annotations = Vec::new();

    for file in files {
        let asset = dataset
            .asset_by_id(file.asset_id)
            .ok_or(ExportError::UnknownAsset {
                asset_id: file.asset_id,
            })?;

        for (line_index, line) in file.contents.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() != 5 {
                return Err(ExportError::InvalidYoloLine {
                    asset_id: file.asset_id,
                    line: line_index + 1,
                    message: "expected 5 whitespace-separated fields".to_string(),
                });
            }

            let class_index = parse_usize(parts[0], file.asset_id, line_index + 1)?;
            let label =
                dataset
                    .labels
                    .get(class_index)
                    .ok_or_else(|| ExportError::InvalidYoloLine {
                        asset_id: file.asset_id,
                        line: line_index + 1,
                        message: format!("unknown class index {class_index}"),
                    })?;

            let values = [
                parse_f64(parts[1], file.asset_id, line_index + 1)?,
                parse_f64(parts[2], file.asset_id, line_index + 1)?,
                parse_f64(parts[3], file.asset_id, line_index + 1)?,
                parse_f64(parts[4], file.asset_id, line_index + 1)?,
            ];
            let bbox = bbox_from_yolo(values, asset.dimensions).map_err(|message| {
                ExportError::InvalidYoloLine {
                    asset_id: file.asset_id,
                    line: line_index + 1,
                    message,
                }
            })?;

            annotations.push(AnnotationRecord::new_imported(
                file.asset_id,
                label.id,
                AnnotationGeometry::Bbox(bbox),
                "yolo",
                Some(format!(
                    "{}:{}",
                    asset.location.display_path(),
                    line_index + 1
                )),
            ));
        }
    }

    tracing::info!(count = annotations.len(), "YOLO detection import complete");
    Ok(annotations)
}

/// How far normalized values may stray outside `0..=1`. Tools round YOLO
/// coordinates, often to 6 decimals, so a box touching the image edge can
/// come back a hair outside it; such boxes are clipped instead of rejected.
const YOLO_EDGE_TOLERANCE: f64 = 1e-3;

/// Converts normalized YOLO `[center_x, center_y, width, height]` to a pixel
/// box, clipped to the image.
fn bbox_from_yolo(values: [f64; 4], dimensions: ImageDimensions) -> Result<BoundingBox, String> {
    for value in values {
        if !value.is_finite() {
            return Err(format!("invalid number {value}"));
        }
        if !(-YOLO_EDGE_TOLERANCE..=1.0 + YOLO_EDGE_TOLERANCE).contains(&value) {
            return Err(format!("{value} is outside the normalized range 0..=1"));
        }
    }
    let [center_x, center_y, width, height] = values;
    if width <= 0.0 || height <= 0.0 {
        return Err(format!("box size must be positive, got {width}x{height}"));
    }

    let image_width = f64::from(dimensions.width);
    let image_height = f64::from(dimensions.height);
    let min_x = ((center_x - width / 2.0) * image_width).max(0.0);
    let min_y = ((center_y - height / 2.0) * image_height).max(0.0);
    let max_x = ((center_x + width / 2.0) * image_width).min(image_width);
    let max_y = ((center_y + height / 2.0) * image_height).min(image_height);
    if max_x <= min_x || max_y <= min_y {
        return Err("box has no area inside the image".to_string());
    }
    BoundingBox::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y)
        .map_err(|error| error.to_string())
}

fn yolo_class_map(dataset: &Dataset) -> BTreeMap<LabelId, usize> {
    dataset
        .labels
        .iter()
        .enumerate()
        .map(|(index, label)| (label.id, index))
        .collect()
}

fn classes_file_contents(dataset: &Dataset) -> String {
    let mut contents = dataset
        .labels
        .iter()
        .map(|label| label.name.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if !contents.is_empty() {
        contents.push('\n');
    }
    contents
}

fn create_dir_all(path: &Path) -> Result<(), ExportError> {
    fs::create_dir_all(path).map_err(|source| ExportError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn write_file(path: &Path, contents: &str) -> Result<(), ExportError> {
    fs::write(path, contents).map_err(|source| ExportError::Io {
        path: PathBuf::from(path),
        source,
    })
}

fn parse_usize(value: &str, asset_id: AssetId, line: usize) -> Result<usize, ExportError> {
    value.parse().map_err(|_| ExportError::InvalidYoloLine {
        asset_id,
        line,
        message: format!("invalid integer {value:?}"),
    })
}

fn parse_f64(value: &str, asset_id: AssetId, line: usize) -> Result<f64, ExportError> {
    value.parse().map_err(|_| ExportError::InvalidYoloLine {
        asset_id,
        line,
        message: format!("invalid number {value:?}"),
    })
}
