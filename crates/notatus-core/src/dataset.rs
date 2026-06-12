use crate::geometry::{AnnotationGeometry, GeometryError, ImageDimensions};
use crate::ids::{AnnotationId, AssetId, ClassificationId, LabelId, ProjectId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use time::OffsetDateTime;
use tracing;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

pub type Metadata = BTreeMap<String, Value>;

pub fn now_utc() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub project: ProjectMetadata,
}

impl ProjectManifest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            project: ProjectMetadata::new(name),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProjectMetadata {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    pub metadata: Metadata,
}

impl ProjectMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        let now = now_utc();

        Self {
            id: ProjectId::new(),
            name: name.into(),
            description: None,
            created_at: now,
            updated_at: now,
            metadata: Metadata::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Dataset {
    pub manifest: ProjectManifest,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub assets: Vec<AssetRecord>,
    #[serde(default)]
    pub annotations: Vec<AnnotationRecord>,
    #[serde(default)]
    pub classifications: Vec<ClassificationRecord>,
}

impl Dataset {
    #[tracing::instrument(level = "debug", skip_all, fields(name = tracing::field::Empty))]
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        tracing::Span::current().record("name", &name);
        tracing::debug!(name, "creating dataset");

        Self {
            manifest: ProjectManifest::new(name),
            labels: Vec::new(),
            assets: Vec::new(),
            annotations: Vec::new(),
            classifications: Vec::new(),
        }
    }

    #[tracing::instrument(level = "debug", skip_all, fields(name = tracing::field::Empty))]
    pub fn rename_project(&mut self, name: impl Into<String>) {
        let name = name.into();
        tracing::Span::current().record("name", &name);
        tracing::debug!(name, "renaming project");

        self.manifest.project.name = name;
        self.manifest.project.updated_at = now_utc();
    }

    #[tracing::instrument(level = "debug", skip_all, fields(name = tracing::field::Empty))]
    pub fn add_label(&mut self, name: impl Into<String>) -> LabelId {
        let name = name.into();
        tracing::Span::current().record("name", &name);

        let label = Label::new(name);
        let id = label.id;
        tracing::debug!(%id, name = label.name, "adding label");
        self.labels.push(label);
        id
    }

    #[tracing::instrument(level = "debug", skip_all, fields(asset_id = %asset.id, asset_kind = ?asset.kind))]
    pub fn add_asset(&mut self, asset: AssetRecord) -> AssetId {
        let id = asset.id;
        tracing::debug!(%id, "adding asset");
        self.assets.push(asset);
        id
    }

    #[tracing::instrument(level = "debug", skip_all, fields(annotation_id = %annotation.id, asset_id = %annotation.asset_id, label_id = %annotation.label_id))]
    pub fn add_annotation(&mut self, annotation: AnnotationRecord) -> AnnotationId {
        let id = annotation.id;
        tracing::debug!(%id, "adding annotation");
        self.annotations.push(annotation);
        id
    }

    #[tracing::instrument(level = "debug", skip_all, fields(classification_id = %classification.id, asset_id = %classification.asset_id, label_id = %classification.label_id))]
    pub fn add_classification(&mut self, classification: ClassificationRecord) -> ClassificationId {
        let id = classification.id;
        tracing::debug!(%id, "adding classification");
        self.classifications.push(classification);
        id
    }

    pub fn asset_by_id(&self, id: AssetId) -> Option<&AssetRecord> {
        let found = self.assets.iter().find(|asset| asset.id == id);
        if found.is_none() {
            tracing::debug!(%id, "asset not found by id");
        }
        found
    }

    pub fn label_by_id(&self, id: LabelId) -> Option<&Label> {
        let found = self.labels.iter().find(|label| label.id == id);
        if found.is_none() {
            tracing::debug!(%id, "label not found by id");
        }
        found
    }

    #[tracing::instrument(level = "debug", skip_all, fields(
        labels = self.labels.len(),
        assets = self.assets.len(),
        annotations = self.annotations.len(),
        classifications = self.classifications.len(),
    ))]
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.manifest.schema_version != CURRENT_SCHEMA_VERSION {
            tracing::warn!(found = self.manifest.schema_version, supported = CURRENT_SCHEMA_VERSION, "unsupported schema version");
            return Err(ValidationError::UnsupportedSchemaVersion {
                found: self.manifest.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }

        if self.manifest.project.name.trim().is_empty() {
            tracing::warn!(project_id = %self.manifest.project.id, "empty project name");
            return Err(ValidationError::EmptyProjectName {
                project_id: self.manifest.project.id,
            });
        }

        let mut label_ids = BTreeSet::new();
        for label in &self.labels {
            if label.name.trim().is_empty() {
                tracing::warn!(label_id = %label.id, "empty label name");
                return Err(ValidationError::EmptyLabelName { label_id: label.id });
            }
            if !label_ids.insert(label.id) {
                tracing::warn!(label_id = %label.id, "duplicate label id");
                return Err(ValidationError::DuplicateLabel { label_id: label.id });
            }
        }

        let mut asset_ids = BTreeSet::new();
        for asset in &self.assets {
            if !asset_ids.insert(asset.id) {
                tracing::warn!(asset_id = %asset.id, "duplicate asset id");
                return Err(ValidationError::DuplicateAsset { asset_id: asset.id });
            }
            if asset.dimensions.width == 0 || asset.dimensions.height == 0 {
                tracing::warn!(asset_id = %asset.id, width = asset.dimensions.width, height = asset.dimensions.height, "invalid asset dimensions");
                return Err(ValidationError::InvalidAssetDimensions {
                    asset_id: asset.id,
                    width: asset.dimensions.width,
                    height: asset.dimensions.height,
                });
            }
        }

        let mut annotation_ids = BTreeSet::new();
        for annotation in &self.annotations {
            if !annotation_ids.insert(annotation.id) {
                tracing::warn!(annotation_id = %annotation.id, "duplicate annotation id");
                return Err(ValidationError::DuplicateAnnotation {
                    annotation_id: annotation.id,
                });
            }
            if !asset_ids.contains(&annotation.asset_id) {
                tracing::warn!(annotation_id = %annotation.id, asset_id = %annotation.asset_id, "unknown asset reference");
                return Err(ValidationError::UnknownAsset {
                    annotation_id: annotation.id,
                    asset_id: annotation.asset_id,
                });
            }
            if !label_ids.contains(&annotation.label_id) {
                tracing::warn!(annotation_id = %annotation.id, label_id = %annotation.label_id, "unknown label reference");
                return Err(ValidationError::UnknownLabel {
                    annotation_id: annotation.id,
                    label_id: annotation.label_id,
                });
            }
            if let Some(confidence) = annotation.confidence
                && (!confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
            {
                tracing::warn!(annotation_id = %annotation.id, confidence, "invalid confidence value");
                return Err(ValidationError::InvalidConfidence {
                    annotation_id: annotation.id,
                    confidence,
                });
            }

            let asset = self
                .asset_by_id(annotation.asset_id)
                .expect("asset id already validated");
            annotation
                .geometry
                .validate_within_image(asset.dimensions)
                .map_err(|source| ValidationError::InvalidGeometry {
                    annotation_id: annotation.id,
                    source,
                })?;
        }

        let mut classification_ids = BTreeSet::new();
        for classification in &self.classifications {
            if !classification_ids.insert(classification.id) {
                tracing::warn!(classification_id = %classification.id, "duplicate classification id");
                return Err(ValidationError::DuplicateClassification {
                    classification_id: classification.id,
                });
            }
            if !asset_ids.contains(&classification.asset_id) {
                tracing::warn!(classification_id = %classification.id, asset_id = %classification.asset_id, "unknown classification asset");
                return Err(ValidationError::UnknownClassificationAsset {
                    classification_id: classification.id,
                    asset_id: classification.asset_id,
                });
            }
            if !label_ids.contains(&classification.label_id) {
                tracing::warn!(classification_id = %classification.id, label_id = %classification.label_id, "unknown classification label");
                return Err(ValidationError::UnknownClassificationLabel {
                    classification_id: classification.id,
                    label_id: classification.label_id,
                });
            }
            if let Some(confidence) = classification.confidence
                && (!confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
            {
                tracing::warn!(classification_id = %classification.id, confidence, "invalid classification confidence");
                return Err(ValidationError::InvalidClassificationConfidence {
                    classification_id: classification.id,
                    confidence,
                });
            }
        }

        tracing::debug!("dataset validation passed");
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Label {
    pub id: LabelId,
    pub name: String,
    pub color: Option<String>,
    #[serde(default)]
    pub metadata: Metadata,
}

impl Label {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: LabelId::new(),
            name: name.into(),
            color: None,
            metadata: Metadata::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Image,
    Video,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssetLocation {
    LocalPath {
        path: String,
    },
    S3Object {
        endpoint: Option<String>,
        bucket: String,
        key: String,
        version_id: Option<String>,
    },
}

impl AssetLocation {
    pub fn local(path: impl Into<String>) -> Self {
        Self::LocalPath { path: path.into() }
    }

    pub fn display_path(&self) -> Cow<'_, str> {
        match self {
            Self::LocalPath { path } => Cow::Borrowed(path),
            Self::S3Object { bucket, key, .. } => Cow::Owned(format!("s3://{bucket}/{key}")),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetSplit {
    Train,
    Validation,
    Test,
    Unassigned,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AssetRecord {
    pub id: AssetId,
    pub kind: AssetKind,
    pub location: AssetLocation,
    pub dimensions: ImageDimensions,
    pub content_hash: Option<String>,
    pub split: DatasetSplit,
    #[serde(default)]
    pub metadata: Metadata,
}

impl AssetRecord {
    #[tracing::instrument(level = "debug", skip_all, fields(width = width, height = height))]
    pub fn new_image(
        location: AssetLocation,
        width: u32,
        height: u32,
    ) -> Result<Self, GeometryError> {
        let record = Self {
            id: AssetId::new(),
            kind: AssetKind::Image,
            location,
            dimensions: ImageDimensions::new(width, height)?,
            content_hash: None,
            split: DatasetSplit::Unassigned,
            metadata: Metadata::new(),
        };
        tracing::debug!(asset_id = %record.id, "created image asset");
        Ok(record)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    Draft,
    Reviewed,
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ModelProvenance {
    pub name: String,
    pub version: Option<String>,
    pub invocation_id: Option<String>,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ImportProvenance {
    pub format: String,
    pub source_id: Option<String>,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum AnnotationSource {
    Human { user_id: Option<String> },
    Model(ModelProvenance),
    Imported(ImportProvenance),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnnotationRecord {
    pub id: AnnotationId,
    pub asset_id: AssetId,
    pub label_id: LabelId,
    pub geometry: AnnotationGeometry,
    pub source: AnnotationSource,
    pub confidence: Option<f32>,
    pub review_state: ReviewState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    pub metadata: Metadata,
}

impl AnnotationRecord {
    #[tracing::instrument(level = "debug", skip_all, fields(
        asset_id = %asset_id,
        label_id = %label_id,
    ))]
    pub fn new_human(
        asset_id: AssetId,
        label_id: LabelId,
        geometry: AnnotationGeometry,
        user_id: Option<String>,
    ) -> Self {
        let record = Self::new(
            asset_id,
            label_id,
            geometry,
            AnnotationSource::Human { user_id },
            None,
            ReviewState::Draft,
        );
        tracing::debug!(annotation_id = %record.id, "created human annotation");
        record
    }

    #[tracing::instrument(level = "debug", skip_all, fields(
        asset_id = %asset_id,
        label_id = %label_id,
        model_name = %model.name,
    ))]
    pub fn new_model(
        asset_id: AssetId,
        label_id: LabelId,
        geometry: AnnotationGeometry,
        model: ModelProvenance,
        confidence: Option<f32>,
    ) -> Self {
        let record = Self::new(
            asset_id,
            label_id,
            geometry,
            AnnotationSource::Model(model),
            confidence,
            ReviewState::Draft,
        );
        tracing::debug!(annotation_id = %record.id, confidence, "created model annotation");
        record
    }

    #[tracing::instrument(level = "debug", skip_all, fields(
        asset_id = %asset_id,
        label_id = %label_id,
        format = tracing::field::Empty,
    ))]
    pub fn new_imported(
        asset_id: AssetId,
        label_id: LabelId,
        geometry: AnnotationGeometry,
        format: impl Into<String>,
        source_id: Option<String>,
    ) -> Self {
        let format = format.into();
        let record = Self::new(
            asset_id,
            label_id,
            geometry,
            AnnotationSource::Imported(ImportProvenance {
                format: format.clone(),
                source_id,
                metadata: Metadata::new(),
            }),
            None,
            ReviewState::Draft,
        );
        tracing::debug!(annotation_id = %record.id, %format, "created imported annotation");
        record
    }

    #[tracing::instrument(level = "debug", skip_all, fields(annotation_id = %self.id))]
    pub fn accept(&mut self) {
        tracing::debug!(annotation_id = %self.id, "accepting annotation");
        self.review_state = ReviewState::Accepted;
        self.updated_at = now_utc();
    }

    fn new(
        asset_id: AssetId,
        label_id: LabelId,
        geometry: AnnotationGeometry,
        source: AnnotationSource,
        confidence: Option<f32>,
        review_state: ReviewState,
    ) -> Self {
        let now = now_utc();

        Self {
            id: AnnotationId::new(),
            asset_id,
            label_id,
            geometry,
            source,
            confidence,
            review_state,
            created_at: now,
            updated_at: now,
            metadata: Metadata::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ClassificationRecord {
    pub id: ClassificationId,
    pub asset_id: AssetId,
    pub label_id: LabelId,
    pub source: AnnotationSource,
    pub confidence: Option<f32>,
    pub review_state: ReviewState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    pub metadata: Metadata,
}

impl ClassificationRecord {
    #[tracing::instrument(level = "debug", skip_all, fields(
        asset_id = %asset_id,
        label_id = %label_id,
    ))]
    pub fn new_human(asset_id: AssetId, label_id: LabelId, user_id: Option<String>) -> Self {
        let record = Self::new(
            asset_id,
            label_id,
            AnnotationSource::Human { user_id },
            None,
            ReviewState::Draft,
        );
        tracing::debug!(classification_id = %record.id, "created human classification");
        record
    }

    #[tracing::instrument(level = "debug", skip_all, fields(
        asset_id = %asset_id,
        label_id = %label_id,
        model_name = %model.name,
    ))]
    pub fn new_model(
        asset_id: AssetId,
        label_id: LabelId,
        model: ModelProvenance,
        confidence: Option<f32>,
    ) -> Self {
        let record = Self::new(
            asset_id,
            label_id,
            AnnotationSource::Model(model),
            confidence,
            ReviewState::Draft,
        );
        tracing::debug!(classification_id = %record.id, confidence, "created model classification");
        record
    }

    #[tracing::instrument(level = "debug", skip_all, fields(
        asset_id = %asset_id,
        label_id = %label_id,
        format = tracing::field::Empty,
    ))]
    pub fn new_imported(
        asset_id: AssetId,
        label_id: LabelId,
        format: impl Into<String>,
        source_id: Option<String>,
    ) -> Self {
        let format = format.into();
        let record = Self::new(
            asset_id,
            label_id,
            AnnotationSource::Imported(ImportProvenance {
                format: format.clone(),
                source_id,
                metadata: Metadata::new(),
            }),
            None,
            ReviewState::Draft,
        );
        tracing::debug!(classification_id = %record.id, %format, "created imported classification");
        record
    }

    fn new(
        asset_id: AssetId,
        label_id: LabelId,
        source: AnnotationSource,
        confidence: Option<f32>,
        review_state: ReviewState,
    ) -> Self {
        let now = now_utc();

        Self {
            id: ClassificationId::new(),
            asset_id,
            label_id,
            source,
            confidence,
            review_state,
            created_at: now,
            updated_at: now,
            metadata: Metadata::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("unsupported schema version {found}, supported version is {supported}")]
    UnsupportedSchemaVersion { found: u32, supported: u32 },
    #[error("project {project_id} has an empty name")]
    EmptyProjectName { project_id: ProjectId },
    #[error("duplicate label id {label_id}")]
    DuplicateLabel { label_id: LabelId },
    #[error("duplicate asset id {asset_id}")]
    DuplicateAsset { asset_id: AssetId },
    #[error("duplicate annotation id {annotation_id}")]
    DuplicateAnnotation { annotation_id: AnnotationId },
    #[error("duplicate classification id {classification_id}")]
    DuplicateClassification { classification_id: ClassificationId },
    #[error("label {label_id} has an empty name")]
    EmptyLabelName { label_id: LabelId },
    #[error("asset {asset_id} has invalid dimensions {width}x{height}")]
    InvalidAssetDimensions {
        asset_id: AssetId,
        width: u32,
        height: u32,
    },
    #[error("annotation {annotation_id} references unknown asset {asset_id}")]
    UnknownAsset {
        annotation_id: AnnotationId,
        asset_id: AssetId,
    },
    #[error("annotation {annotation_id} references unknown label {label_id}")]
    UnknownLabel {
        annotation_id: AnnotationId,
        label_id: LabelId,
    },
    #[error("classification {classification_id} references unknown asset {asset_id}")]
    UnknownClassificationAsset {
        classification_id: ClassificationId,
        asset_id: AssetId,
    },
    #[error("classification {classification_id} references unknown label {label_id}")]
    UnknownClassificationLabel {
        classification_id: ClassificationId,
        label_id: LabelId,
    },
    #[error("annotation {annotation_id} has invalid confidence {confidence}")]
    InvalidConfidence {
        annotation_id: AnnotationId,
        confidence: f32,
    },
    #[error("classification {classification_id} has invalid confidence {confidence}")]
    InvalidClassificationConfidence {
        classification_id: ClassificationId,
        confidence: f32,
    },
    #[error("annotation {annotation_id} has invalid geometry: {source}")]
    InvalidGeometry {
        annotation_id: AnnotationId,
        source: GeometryError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnnotationGeometry, BoundingBox};

    #[test]
    fn validates_dataset_references_and_geometry() {
        let mut dataset = Dataset::new("demo");
        let label_id = dataset.add_label("car");
        let asset = AssetRecord::new_image(AssetLocation::local("images/a.jpg"), 640, 480).unwrap();
        let asset_id = dataset.add_asset(asset);
        let mut annotation = AnnotationRecord::new_human(
            asset_id,
            label_id,
            AnnotationGeometry::Bbox(BoundingBox::from_xywh(10.0, 20.0, 100.0, 80.0).unwrap()),
            Some("reviewer".to_string()),
        );
        annotation.accept();
        dataset.add_annotation(annotation);

        assert!(dataset.validate().is_ok());
    }

    #[test]
    fn validates_classification_references() {
        let mut dataset = Dataset::new("demo");
        let label_id = dataset.add_label("outdoor");
        let asset = AssetRecord::new_image(AssetLocation::local("images/a.jpg"), 640, 480).unwrap();
        let asset_id = dataset.add_asset(asset);
        dataset.add_classification(ClassificationRecord::new_human(asset_id, label_id, None));

        assert!(dataset.validate().is_ok());
    }

    #[test]
    fn rejects_unknown_label_reference() {
        let mut dataset = Dataset::new("demo");
        let asset = AssetRecord::new_image(AssetLocation::local("images/a.jpg"), 640, 480).unwrap();
        let asset_id = dataset.add_asset(asset);
        let annotation = AnnotationRecord::new_human(
            asset_id,
            LabelId::new(),
            AnnotationGeometry::Bbox(BoundingBox::from_xywh(1.0, 1.0, 10.0, 10.0).unwrap()),
            None,
        );
        dataset.add_annotation(annotation);

        assert!(matches!(
            dataset.validate(),
            Err(ValidationError::UnknownLabel { .. })
        ));
    }

    #[test]
    fn rejects_unknown_classification_label_reference() {
        let mut dataset = Dataset::new("demo");
        let asset = AssetRecord::new_image(AssetLocation::local("images/a.jpg"), 640, 480).unwrap();
        let asset_id = dataset.add_asset(asset);
        dataset.add_classification(ClassificationRecord::new_human(
            asset_id,
            LabelId::new(),
            None,
        ));

        assert!(matches!(
            dataset.validate(),
            Err(ValidationError::UnknownClassificationLabel { .. })
        ));
    }

    #[test]
    fn rejects_invalid_classification_confidence() {
        let mut dataset = Dataset::new("demo");
        let label_id = dataset.add_label("outdoor");
        let asset = AssetRecord::new_image(AssetLocation::local("images/a.jpg"), 640, 480).unwrap();
        let asset_id = dataset.add_asset(asset);
        let mut classification = ClassificationRecord::new_human(asset_id, label_id, None);
        classification.confidence = Some(1.5);
        dataset.add_classification(classification);

        assert!(matches!(
            dataset.validate(),
            Err(ValidationError::InvalidClassificationConfidence { .. })
        ));
    }

    #[test]
    fn rejects_empty_project_name() {
        let mut dataset = Dataset::new("demo");

        dataset.rename_project(" ");

        assert!(matches!(
            dataset.validate(),
            Err(ValidationError::EmptyProjectName { .. })
        ));
    }
}
