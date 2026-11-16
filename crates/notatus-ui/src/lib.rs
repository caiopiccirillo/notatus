//! The Notatus desktop app.
//!
//! `state` holds the project being edited, its selection, and undo history,
//! independent of any renderer so it can be tested on its own. `app` is the
//! GPUI window around it, compiled through the default `gpui-ui` feature.

mod state;

pub use state::{
    AnnotationTool, CoalesceKey, DatasetIndex, Direction, ModelPredictionSummary, ReviewFilter,
    UiMutationError, UiState,
};

#[cfg(feature = "gpui-ui")]
mod app;

#[cfg(feature = "gpui-ui")]
pub fn launch_gpui() {
    app::launch_gpui();
}
