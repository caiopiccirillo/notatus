use crate::{AnnotationFilter, ExportError};
use notatus_core::{AssetId, Dataset, LabelId, Metadata};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

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
            image_id: *image_ids
                .get(&annotation.asset_id)
                .ok_or(ExportError::UnknownAsset {
                    asset_id: annotation.asset_id,
                })?,
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
