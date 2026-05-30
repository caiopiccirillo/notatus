//! Domain model for Notatus datasets.
//!
//! The core crate owns the canonical schema. UI, storage, import/export, and
//! inference integrations should translate to and from these types instead of
//! inventing their own annotation shapes.

mod dataset;
mod geometry;
mod ids;

pub use dataset::{
    AnnotationRecord, AnnotationSource, AssetKind, AssetLocation, AssetRecord,
    CURRENT_SCHEMA_VERSION, Dataset, DatasetSplit, ImportProvenance, Label, Metadata,
    ModelProvenance, ProjectManifest, ProjectMetadata, ReviewState, ValidationError,
};
pub use geometry::{
    AnnotationGeometry, BoundingBox, GeometryError, ImageDimensions, Point, Polygon,
};
pub use ids::{AnnotationId, AssetId, LabelId, ProjectId};

pub type Result<T> = std::result::Result<T, ValidationError>;
