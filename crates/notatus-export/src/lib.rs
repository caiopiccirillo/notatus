//! Import and export adapters for training and interoperability formats.
//!
//! The canonical Notatus schema remains the source of truth. Format modules
//! only translate to and from that schema.

use notatus_core::{
    AnnotationGeometry, AnnotationRecord, AssetId, BoundingBox, Dataset, LabelId, Metadata,
    ReviewState,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct AnnotationFilter {
    review_states: BTreeSet<ReviewState>,
}

impl AnnotationFilter {
    pub fn accepted_and_reviewed() -> Self {
        Self {
            review_states: BTreeSet::from([ReviewState::Accepted, ReviewState::Reviewed]),
        }
    }

    pub fn all_non_rejected() -> Self {
        Self {
            review_states: BTreeSet::from([
                ReviewState::Draft,
                ReviewState::Reviewed,
                ReviewState::Accepted,
            ]),
        }
    }

    pub fn accepts(&self, annotation: &AnnotationRecord) -> bool {
        self.review_states.contains(&annotation.review_state)
    }
}

impl Default for AnnotationFilter {
    fn default() -> Self {
        Self::accepted_and_reviewed()
    }
}

pub mod yolo {
    use super::*;

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
                let class_index = class_map.get(&annotation.label_id).copied().ok_or(
                    ExportError::UnknownLabel {
                        label_id: annotation.label_id,
                    },
                )?;
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
                let label = dataset.labels.get(class_index).ok_or_else(|| {
                    ExportError::InvalidYoloLine {
                        asset_id: file.asset_id,
                        line: line_index + 1,
                        message: format!("unknown class index {class_index}"),
                    }
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
}

pub mod coco {
    use super::*;

    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    pub struct CocoDataset {
        pub info: CocoInfo,
        pub images: Vec<CocoImage>,
        pub annotations: Vec<CocoAnnotation>,
        pub categories: Vec<CocoCategory>,
    }

    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    pub struct CocoInfo {
        pub description: String,
        pub version: String,
    }

    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    pub struct CocoImage {
        pub id: u64,
        pub file_name: String,
        pub width: u32,
        pub height: u32,
    }

    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    pub struct CocoAnnotation {
        pub id: u64,
        pub image_id: u64,
        pub category_id: u64,
        pub bbox: [f64; 4],
        pub area: f64,
        pub iscrowd: u8,
        #[serde(skip_serializing_if = "Metadata::is_empty", default)]
        pub attributes: Metadata,
    }

    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    pub struct CocoCategory {
        pub id: u64,
        pub name: String,
        pub supercategory: String,
    }

    pub fn export_detection(
        dataset: &Dataset,
        filter: &AnnotationFilter,
    ) -> Result<CocoDataset, ExportError> {
        dataset.validate()?;

        let category_ids: BTreeMap<LabelId, u64> = dataset
            .labels
            .iter()
            .enumerate()
            .map(|(index, label)| (label.id, index as u64 + 1))
            .collect();
        let image_ids: BTreeMap<AssetId, u64> = dataset
            .assets
            .iter()
            .enumerate()
            .map(|(index, asset)| (asset.id, index as u64 + 1))
            .collect();

        let images = dataset
            .assets
            .iter()
            .map(|asset| CocoImage {
                id: *image_ids
                    .get(&asset.id)
                    .expect("image id generated from same asset list"),
                file_name: asset.location.display_path().into_owned(),
                width: asset.dimensions.width,
                height: asset.dimensions.height,
            })
            .collect();

        let categories = dataset
            .labels
            .iter()
            .map(|label| CocoCategory {
                id: *category_ids
                    .get(&label.id)
                    .expect("category id generated from same label list"),
                name: label.name.clone(),
                supercategory: "object".to_string(),
            })
            .collect();

        let mut annotations = Vec::new();
        for annotation in dataset
            .annotations
            .iter()
            .filter(|annotation| filter.accepts(annotation))
        {
            let Some(bbox) = annotation.geometry.as_bbox() else {
                continue;
            };

            let mut attributes = annotation.metadata.clone();
            attributes.insert(
                "notatus_annotation_id".to_string(),
                Value::String(annotation.id.to_string()),
            );
            attributes.insert(
                "review_state".to_string(),
                Value::String(format!("{:?}", annotation.review_state).to_lowercase()),
            );
            if let Some(confidence) = annotation.confidence {
                attributes.insert("confidence".to_string(), Value::from(confidence));
            }

            annotations.push(CocoAnnotation {
                id: annotations.len() as u64 + 1,
                image_id: *image_ids.get(&annotation.asset_id).ok_or(
                    ExportError::UnknownAsset {
                        asset_id: annotation.asset_id,
                    },
                )?,
                category_id: *category_ids.get(&annotation.label_id).ok_or(
                    ExportError::UnknownLabel {
                        label_id: annotation.label_id,
                    },
                )?,
                bbox: [bbox.x, bbox.y, bbox.width, bbox.height],
                area: bbox.area(),
                iscrowd: 0,
                attributes,
            });
        }

        Ok(CocoDataset {
            info: CocoInfo {
                description: dataset.manifest.project.name.clone(),
                version: dataset.manifest.schema_version.to_string(),
            },
            images,
            annotations,
            categories,
        })
    }
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error(transparent)]
    Validation(#[from] notatus_core::ValidationError),
    #[error(transparent)]
    Geometry(#[from] notatus_core::GeometryError),
    #[error("unknown asset {asset_id}")]
    UnknownAsset { asset_id: AssetId },
    #[error("unknown label {label_id}")]
    UnknownLabel { label_id: LabelId },
    #[error("invalid YOLO line for asset {asset_id} at line {line}: {message}")]
    InvalidYoloLine {
        asset_id: AssetId,
        line: usize,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use notatus_core::{AnnotationRecord, AssetLocation, AssetRecord, BoundingBox};

    fn sample_dataset() -> Dataset {
        let mut dataset = Dataset::new("demo");
        let label_id = dataset.add_label("car");
        let asset = AssetRecord::new_image(AssetLocation::local("images/a.jpg"), 200, 100).unwrap();
        let asset_id = dataset.add_asset(asset);
        let mut annotation = AnnotationRecord::new_human(
            asset_id,
            label_id,
            AnnotationGeometry::Bbox(BoundingBox::from_xywh(75.0, 30.0, 50.0, 40.0).unwrap()),
            None,
        );
        annotation.accept();
        dataset.add_annotation(annotation);
        dataset
    }

    #[test]
    fn exports_yolo_detection_files() {
        let files =
            yolo::export_detection(&sample_dataset(), &AnnotationFilter::default()).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].contents, "0 0.500000 0.500000 0.250000 0.400000");
    }

    #[test]
    fn imports_yolo_detection_files() {
        let dataset = sample_dataset();
        let imported = yolo::import_detection(
            &dataset,
            &[yolo::YoloImportFile {
                asset_id: dataset.assets[0].id,
                contents: "0 0.500000 0.500000 0.250000 0.400000".to_string(),
            }],
        )
        .unwrap();

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].label_id, dataset.labels[0].id);
    }

    #[test]
    fn exports_coco_detection_dataset() {
        let exported =
            coco::export_detection(&sample_dataset(), &AnnotationFilter::default()).unwrap();

        assert_eq!(exported.images.len(), 1);
        assert_eq!(exported.categories[0].name, "car");
        assert_eq!(exported.annotations[0].bbox, [75.0, 30.0, 50.0, 40.0]);
    }
}
