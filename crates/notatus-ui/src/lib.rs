//! Desktop application state for the Notatus GPUI client.
//!
//! The GPUI windowing code is intentionally thin. The mutable annotation state
//! lives here so the same behavior can be tested without a renderer.

mod state;

pub use state::{AnnotationTool, UiMutationError, UiState};

#[cfg(feature = "gpui-ui")]
mod app;

#[cfg(feature = "gpui-ui")]
pub fn launch_gpui() {
    app::launch_gpui();
}
