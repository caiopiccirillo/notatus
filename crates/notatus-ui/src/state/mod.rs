//! The renderer-independent state of an open project: the dataset, what
//! is selected, undo history, and whether there are unsaved changes.
//!
//! Every change to the dataset goes through a `UiState` method, which
//! validates it, records an undo step, and marks the project dirty.

mod annotations;
mod classifications;
mod error;
mod history;
mod labels;
mod media;
mod review;
#[cfg(test)]
mod tests;

pub use error::UiMutationError;
pub use history::CoalesceKey;
pub use review::ModelPredictionSummary;

use history::{History, Selection, Snapshot};
use notatus_core::{AnnotationId, AssetId, Dataset, LabelId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnnotationTool {
    Select,
    DrawBox,
    DrawPolygon,
}

impl AnnotationTool {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::DrawBox => "Draw Box",
            Self::DrawPolygon => "Draw Polygon",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RemovalSummary {
    pub annotations: usize,
    pub classifications: usize,
}

#[derive(Clone, Debug)]
pub struct UiState {
    pub dataset: Dataset,
    pub active_tool: AnnotationTool,
    pub selected_asset: Option<AssetId>,
    pub selected_annotation: Option<AnnotationId>,
    pub selected_label: Option<LabelId>,
    /// True when the dataset differs from the last saved revision.
    pub dirty: bool,
    revision: u64,
    saved_revision: u64,
    next_revision: u64,
    history: History,
    batch_depth: usize,
}

/// Which way to step through media or annotations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Previous,
    Next,
}

impl UiState {
    fn with_dataset(dataset: Dataset) -> Self {
        Self {
            dataset,
            active_tool: AnnotationTool::Select,
            selected_asset: None,
            selected_annotation: None,
            selected_label: None,
            dirty: false,
            revision: 0,
            saved_revision: 0,
            next_revision: 1,
            history: History::default(),
            batch_depth: 0,
        }
    }

    /// Monotonic identifier of the current dataset contents. Undo restores
    /// earlier revisions, so equal revisions mean equal datasets.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Records the current state for undo and moves to a fresh revision.
    /// Call after validation succeeds and right before mutating the dataset.
    fn begin_change(&mut self, description: &'static str, key: Option<CoalesceKey>) {
        if self.batch_depth == 0 && !self.history.coalesces(key) {
            let snapshot = self.snapshot();
            self.history.record(snapshot, description, key);
        }
        self.revision = self.next_revision;
        self.next_revision += 1;
        self.dirty = true;
    }

    /// Runs several mutations as one undo step.
    pub fn batch<R>(&mut self, description: &'static str, f: impl FnOnce(&mut Self) -> R) -> R {
        let before = self.revision;
        let snapshot = self.snapshot();
        self.batch_depth += 1;
        let result = f(self);
        self.batch_depth -= 1;
        if self.batch_depth == 0 && self.revision != before {
            self.history.record(snapshot, description, None);
        }
        result
    }

    /// Ends the current run of coalesced edits, such as one drag gesture.
    pub fn break_history_coalescing(&mut self) {
        self.history.break_coalescing();
    }

    pub fn undo_description(&self) -> Option<&'static str> {
        self.history.undo_description()
    }

    pub fn redo_description(&self) -> Option<&'static str> {
        self.history.redo_description()
    }

    /// Restores the state before the last change. Returns its description.
    pub fn undo(&mut self) -> Option<&'static str> {
        let current = self.snapshot();
        let (snapshot, description) = self.history.undo(current)?;
        self.restore(snapshot);
        Some(description)
    }

    /// Reapplies the last undone change. Returns its description.
    pub fn redo(&mut self) -> Option<&'static str> {
        let current = self.snapshot();
        let (snapshot, description) = self.history.redo(current)?;
        self.restore(snapshot);
        Some(description)
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            dataset: self.dataset.clone(),
            selection: Selection {
                asset: self.selected_asset,
                annotation: self.selected_annotation,
                label: self.selected_label,
            },
            revision: self.revision,
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.dataset = snapshot.dataset;
        self.selected_asset = snapshot.selection.asset;
        self.selected_annotation = snapshot.selection.annotation;
        self.selected_label = snapshot.selection.label;
        self.revision = snapshot.revision;
        self.dirty = self.revision != self.saved_revision;
    }

    #[tracing::instrument(level = "debug", skip_all, fields(name = tracing::field::Empty))]
    pub fn new_project(name: impl Into<String>) -> Self {
        let name = name.into();
        tracing::debug!(name, "creating new project");
        Self::with_dataset(Dataset::new(name))
    }

    #[tracing::instrument(level = "debug", skip_all, fields(
        project_name = %dataset.manifest.project.name,
        labels = dataset.labels.len(),
        assets = dataset.assets.len(),
        annotations = dataset.annotations.len(),
    ))]
    pub fn from_dataset(dataset: Dataset) -> Result<Self, UiMutationError> {
        dataset.validate()?;
        tracing::debug!("loading dataset into UI state");
        let mut state = Self::with_dataset(dataset);
        state.selected_asset = state.dataset.assets.first().map(|asset| asset.id);
        state.selected_label = state.dataset.labels.first().map(|label| label.id);
        Ok(state)
    }

    #[tracing::instrument(level = "trace", skip_all, fields(tool = ?tool))]
    pub fn set_tool(&mut self, tool: AnnotationTool) {
        tracing::trace!(tool = %tool.display_name(), "tool changed");
        self.active_tool = tool;
    }

    pub fn mark_saved(&mut self) {
        tracing::trace!("marking dataset as saved");
        self.saved_revision = self.revision;
        self.dirty = false;
    }

    /// Marks a specific revision as saved, for saves that finish after more
    /// edits were made.
    pub fn mark_revision_saved(&mut self, revision: u64) {
        self.saved_revision = revision;
        self.dirty = self.revision != self.saved_revision;
    }

    #[tracing::instrument(level = "debug", skip_all, fields(name = tracing::field::Empty))]
    pub fn rename_project(&mut self, name: impl Into<String>) -> Result<(), UiMutationError> {
        let name = name.into();
        if name.trim().is_empty() {
            tracing::warn!("attempted to rename project to empty name");
            return Err(UiMutationError::EmptyProjectName);
        }

        tracing::debug!(name, "renaming project");
        self.begin_change("Rename project", Some(CoalesceKey::ProjectName));
        self.dataset.rename_project(name);
        Ok(())
    }
}
