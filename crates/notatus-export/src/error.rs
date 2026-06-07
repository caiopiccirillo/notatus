use notatus_core::{AssetId, LabelId};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error(transparent)]
    Validation(#[from] notatus_core::ValidationError),
    #[error(transparent)]
    Geometry(#[from] notatus_core::GeometryError),
    #[error("failed to write {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize {}: {source}", path.display())]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
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
