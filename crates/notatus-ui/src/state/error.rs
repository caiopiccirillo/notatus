//! Why a change to the UI state was refused.

use notatus_core::{
    AnnotationId, AssetId, ClassificationId, GeometryError, LabelId, ValidationError,
};

#[derive(Debug, thiserror::Error)]
pub enum UiMutationError {
    #[error("{0}")]
    Geometry(#[from] GeometryError),
    #[error("{0}")]
    Validation(#[from] ValidationError),
    #[error("missing asset {asset_id}")]
    MissingAsset { asset_id: AssetId },
    #[error("missing annotation {annotation_id}")]
    MissingAnnotation { annotation_id: AnnotationId },
    #[error("missing classification {classification_id}")]
    MissingClassification { classification_id: ClassificationId },
    #[error("missing label {label_id}")]
    MissingLabel { label_id: LabelId },
    #[error("project needs a name")]
    EmptyProjectName,
    #[error("label {label_id} needs a name")]
    EmptyLabelName { label_id: LabelId },
}
