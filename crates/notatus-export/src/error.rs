use notatus_core::{AssetId, LabelId};
use thiserror::Error;

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
