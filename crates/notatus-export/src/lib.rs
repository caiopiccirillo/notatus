//! Import and export adapters for training and interoperability formats.
//!
//! The canonical Notatus schema remains the source of truth. Format modules
//! only translate to and from that schema.

mod error;
mod filter;

pub mod coco;
pub mod yolo;

pub use error::ExportError;
pub use filter::AnnotationFilter;

#[cfg(test)]
mod tests;
