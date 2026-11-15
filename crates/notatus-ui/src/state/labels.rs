//! Creating, renaming, coloring, and removing labels.

use super::history::CoalesceKey;
use super::{RemovalSummary, UiMutationError, UiState};
use notatus_core::LabelId;

impl UiState {
    #[tracing::instrument(level = "debug", skip_all, fields(name = tracing::field::Empty))]
    pub fn add_label(&mut self, name: impl Into<String>) -> LabelId {
        let name = name.into();
        self.begin_change("Create label", None);
        let label_id = self.dataset.add_label(name);
        self.selected_label = Some(label_id);
        tracing::debug!(label_id = %label_id, "label added via UI");
        label_id
    }

    #[tracing::instrument(level = "debug", skip_all, fields(label_id = %label_id))]
    pub fn select_label(&mut self, label_id: LabelId) -> Result<(), UiMutationError> {
        if self.dataset.label_by_id(label_id).is_none() {
            tracing::warn!(%label_id, "selected missing label");
            return Err(UiMutationError::MissingLabel { label_id });
        }

        self.selected_label = Some(label_id);
        self.selected_annotation = None;
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all, fields(label_id = %label_id, name = tracing::field::Empty))]
    pub fn update_label_name(
        &mut self,
        label_id: LabelId,
        name: impl Into<String>,
    ) -> Result<(), UiMutationError> {
        let name = name.into();
        if name.trim().is_empty() {
            tracing::warn!(%label_id, "attempted to set empty label name");
            return Err(UiMutationError::EmptyLabelName { label_id });
        }

        let index = self.label_index(label_id)?;
        self.begin_change("Rename label", Some(CoalesceKey::LabelName(label_id)));
        self.dataset.labels[index].name = name;
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all, fields(label_id = %label_id))]
    pub fn update_label_color(
        &mut self,
        label_id: LabelId,
        color: Option<String>,
    ) -> Result<(), UiMutationError> {
        let index = self.label_index(label_id)?;
        if self.dataset.labels[index].color == color {
            return Ok(());
        }
        self.begin_change(
            "Change label color",
            Some(CoalesceKey::LabelColor(label_id)),
        );
        self.dataset.labels[index].color = color;
        Ok(())
    }

    pub(super) fn label_index(&self, label_id: LabelId) -> Result<usize, UiMutationError> {
        self.dataset
            .labels
            .iter()
            .position(|label| label.id == label_id)
            .ok_or(UiMutationError::MissingLabel { label_id })
    }

    #[tracing::instrument(level = "debug", skip_all, fields(label_id = %label_id))]
    pub fn remove_label(&mut self, label_id: LabelId) -> Result<RemovalSummary, UiMutationError> {
        let label_index = self
            .dataset
            .labels
            .iter()
            .position(|label| label.id == label_id)
            .ok_or(UiMutationError::MissingLabel { label_id })?;

        let selected_annotation_removed = self.selected_annotation.is_some_and(|annotation_id| {
            self.dataset
                .annotations
                .iter()
                .any(|annotation| annotation.id == annotation_id && annotation.label_id == label_id)
        });
        let original_annotation_count = self.dataset.annotations.len();
        let original_classification_count = self.dataset.classifications.len();

        self.begin_change("Delete label", None);
        self.dataset.labels.remove(label_index);
        self.dataset
            .annotations
            .retain(|annotation| annotation.label_id != label_id);
        self.dataset
            .classifications
            .retain(|classification| classification.label_id != label_id);

        if self.selected_label == Some(label_id) {
            self.selected_label = self.dataset.labels.first().map(|label| label.id);
        }
        if selected_annotation_removed {
            self.selected_annotation = None;
        }

        let summary = RemovalSummary {
            annotations: original_annotation_count - self.dataset.annotations.len(),
            classifications: original_classification_count - self.dataset.classifications.len(),
        };
        tracing::info!(label_id = %label_id, removed_annotations = summary.annotations, removed_classifications = summary.classifications, "label removed");
        Ok(summary)
    }
}
