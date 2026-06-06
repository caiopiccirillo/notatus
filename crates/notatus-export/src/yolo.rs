use crate::{AnnotationFilter, ExportError};
use notatus_core::{AnnotationGeometry, AnnotationRecord, AssetId, BoundingBox, Dataset, LabelId};
use std::collections::BTreeMap;

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

    Ok(files)
}

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

            let center_x = parse_f64(parts[1], file.asset_id, line_index + 1)?;
            let center_y = parse_f64(parts[2], file.asset_id, line_index + 1)?;
            let width = parse_f64(parts[3], file.asset_id, line_index + 1)?;
            let height = parse_f64(parts[4], file.asset_id, line_index + 1)?;
            let bbox = BoundingBox::from_yolo_normalized(
                center_x,
                center_y,
                width,
                height,
                asset.dimensions,
            )?;

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

    Ok(annotations)
}

fn yolo_class_map(dataset: &Dataset) -> BTreeMap<LabelId, usize> {
    dataset
        .labels
        .iter()
        .enumerate()
        .map(|(index, label)| (label.id, index))
        .collect()
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
