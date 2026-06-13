use crate::ExportError;
use notatus_core::{ClassificationId, Dataset, ReviewState};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ClassificationExportRow {
    pub classification_id: ClassificationId,
    pub asset_id: notatus_core::AssetId,
    pub image_path: String,
    pub label_id: notatus_core::LabelId,
    pub label_name: String,
    pub review_state: String,
    pub confidence: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassificationExportSummary {
    pub classification_count: usize,
}

#[tracing::instrument(level = "debug", skip_all, fields(
    classifications = dataset.classifications.len(),
))]
pub fn export_classifications(
    dataset: &Dataset,
) -> Result<Vec<ClassificationExportRow>, ExportError> {
    dataset.validate()?;

    let mut rows = Vec::new();
    for classification in dataset
        .classifications
        .iter()
        .filter(|classification| classification.review_state != ReviewState::Rejected)
    {
        let asset =
            dataset
                .asset_by_id(classification.asset_id)
                .ok_or(ExportError::UnknownAsset {
                    asset_id: classification.asset_id,
                })?;
        let label =
            dataset
                .label_by_id(classification.label_id)
                .ok_or(ExportError::UnknownLabel {
                    label_id: classification.label_id,
                })?;

        rows.push(ClassificationExportRow {
            classification_id: classification.id,
            asset_id: classification.asset_id,
            image_path: asset.location.display_path().into_owned(),
            label_id: classification.label_id,
            label_name: label.name.clone(),
            review_state: format!("{:?}", classification.review_state).to_lowercase(),
            confidence: classification.confidence,
        });
    }

    tracing::info!(count = rows.len(), "classification export complete");
    Ok(rows)
}

#[tracing::instrument(level = "info", skip_all, fields(output_dir = %output_dir.as_ref().display()))]
pub fn write_classification_export(
    dataset: &Dataset,
    output_dir: impl AsRef<Path>,
) -> Result<ClassificationExportSummary, ExportError> {
    let output_dir = output_dir.as_ref();
    create_dir_all(output_dir)?;

    let rows = export_classifications(dataset)?;
    let json_path = output_dir.join("classifications.json");
    let json = serde_json::to_string_pretty(&rows).map_err(|source| ExportError::Json {
        path: json_path.clone(),
        source,
    })?;
    write_file(&json_path, &json)?;

    let csv_path = output_dir.join("classifications.csv");
    write_file(&csv_path, &classification_csv(&rows))?;

    let summary = ClassificationExportSummary {
        classification_count: rows.len(),
    };

    tracing::info!(count = summary.classification_count, "classification export written");
    Ok(summary)
}

fn classification_csv(rows: &[ClassificationExportRow]) -> String {
    let mut contents =
        "classification_id,asset_id,image_path,label_id,label_name,review_state,confidence\n"
            .to_string();
    for row in rows {
        contents.push_str(&csv_line(&[
            row.classification_id.to_string(),
            row.asset_id.to_string(),
            row.image_path.clone(),
            row.label_id.to_string(),
            row.label_name.clone(),
            row.review_state.clone(),
            row.confidence
                .map(|confidence| confidence.to_string())
                .unwrap_or_default(),
        ]));
        contents.push('\n');
    }
    contents
}

fn csv_line(fields: &[String]) -> String {
    fields
        .iter()
        .map(|field| {
            if field.contains(',') || field.contains('"') || field.contains('\n') {
                format!("\"{}\"", field.replace('"', "\"\""))
            } else {
                field.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
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
