//! Why a change to the UI state was refused.

use notatus_core::{
    AnnotationId, AssetId, ClassificationId, GeometryError, LabelId, ValidationError,
};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum UiMutationError {
    Geometry(GeometryError),
    Validation(ValidationError),
    MissingAsset { asset_id: AssetId },
    MissingAnnotation { annotation_id: AnnotationId },
    MissingClassification { classification_id: ClassificationId },
    MissingLabel { label_id: LabelId },
    EmptyProjectName,
    EmptyLabelName { label_id: LabelId },
}

impl fmt::Display for UiMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geometry(source) => write!(f, "{source}"),
            Self::Validation(source) => write!(f, "{source}"),
            Self::MissingAsset { asset_id } => write!(f, "missing asset {asset_id}"),
            Self::MissingAnnotation { annotation_id } => {
                write!(f, "missing annotation {annotation_id}")
            }
            Self::MissingClassification { classification_id } => {
                write!(f, "missing classification {classification_id}")
            }
            Self::MissingLabel { label_id } => write!(f, "missing label {label_id}"),
            Self::EmptyProjectName => write!(f, "project needs a name"),
            Self::EmptyLabelName { label_id } => write!(f, "label {label_id} needs a name"),
        }
    }
}

impl Error for UiMutationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Geometry(source) => Some(source),
            Self::Validation(source) => Some(source),
            Self::MissingAsset { .. }
            | Self::MissingAnnotation { .. }
            | Self::MissingClassification { .. }
            | Self::MissingLabel { .. }
            | Self::EmptyProjectName
            | Self::EmptyLabelName { .. } => None,
        }
    }
}

impl From<GeometryError> for UiMutationError {
    fn from(source: GeometryError) -> Self {
        Self::Geometry(source)
    }
}

impl From<ValidationError> for UiMutationError {
    fn from(source: ValidationError) -> Self {
        Self::Validation(source)
    }
}
