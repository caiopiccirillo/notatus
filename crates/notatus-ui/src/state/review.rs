//! Reviewing annotations and applying model suggestions.

use super::{UiMutationError, UiState};
use notatus_core::{AnnotationId, AnnotationRecord, AnnotationSource, AssetId, ReviewState};
use std::collections::BTreeSet;

impl UiState {
    /// Sets an annotation's review state, selecting it. No-op when unchanged.
    pub fn set_annotation_review_state(
        &mut self,
        annotation_id: AnnotationId,
        review_state: ReviewState,
    ) -> Result<(), UiMutationError> {
        let index = self.annotation_index(annotation_id)?;
        self.selected_annotation = Some(annotation_id);
        if self.dataset.annotations[index].review_state == review_state {
            return Ok(());
        }
        let description = match review_state {
            ReviewState::Draft => "Reset review",
            ReviewState::Reviewed => "Mark reviewed",
            ReviewState::Accepted => "Accept annotation",
            ReviewState::Rejected => "Reject annotation",
        };
        self.begin_change(description, None);
        self.dataset.annotations[index].set_review_state(review_state);
        Ok(())
    }

    /// The first draft annotation after `from` on the same media, wrapping
    /// around. `None` once every annotation there has been reviewed.
    pub fn next_draft_annotation(&self, from: AnnotationId) -> Option<AnnotationId> {
        let asset_id = self
            .dataset
            .annotations
            .iter()
            .find(|annotation| annotation.id == from)?
            .asset_id;
        let on_asset: Vec<_> = self
            .dataset
            .annotations
            .iter()
            .filter(|annotation| annotation.asset_id == asset_id)
            .collect();
        let start = on_asset
            .iter()
            .position(|annotation| annotation.id == from)?;
        (1..on_asset.len())
            .map(|offset| on_asset[(start + offset) % on_asset.len()])
            .find(|annotation| annotation.review_state == ReviewState::Draft)
            .map(|annotation| annotation.id)
    }

    /// Replaces the model's earlier suggestions on `asset_id` with
    /// `predictions`, as one undo step.
    ///
    /// Earlier model drafts nobody has touched are removed first, so running
    /// a model again does not stack duplicates; drafts a person edited or
    /// reviewed stay. Predictions whose label was removed while the model
    /// ran, or that do not fit the image, are skipped.
    pub fn apply_model_predictions(
        &mut self,
        asset_id: AssetId,
        predictions: Vec<AnnotationRecord>,
    ) -> Result<ModelPredictionSummary, UiMutationError> {
        let dimensions = self
            .dataset
            .asset_by_id(asset_id)
            .ok_or(UiMutationError::MissingAsset { asset_id })?
            .dimensions;
        let label_ids: BTreeSet<_> = self.dataset.labels.iter().map(|label| label.id).collect();
        let (usable, unusable): (Vec<_>, Vec<_>) =
            predictions.into_iter().partition(|annotation| {
                annotation.asset_id == asset_id
                    && label_ids.contains(&annotation.label_id)
                    && annotation
                        .geometry
                        .validate_within_image(dimensions)
                        .is_ok()
            });
        let is_untouched_suggestion = |annotation: &AnnotationRecord| {
            annotation.asset_id == asset_id
                && matches!(annotation.source, AnnotationSource::Model(_))
                && annotation.review_state == ReviewState::Draft
                && !annotation.human_edited
        };
        let replaced = self
            .dataset
            .annotations
            .iter()
            .filter(|annotation| is_untouched_suggestion(annotation))
            .count();
        let summary = ModelPredictionSummary {
            added: usable.len(),
            replaced,
            skipped: unusable.len(),
        };
        if replaced == 0 && usable.is_empty() {
            return Ok(summary);
        }

        self.begin_change("Add model suggestions", None);
        self.dataset
            .annotations
            .retain(|annotation| !is_untouched_suggestion(annotation));
        if self
            .selected_annotation
            .is_some_and(|id| self.annotation_index(id).is_err())
        {
            self.selected_annotation = None;
        }
        self.dataset.annotations.extend(usable);
        Ok(summary)
    }
}

/// What applying a model's predictions to one media item did.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ModelPredictionSummary {
    pub added: usize,
    /// Earlier untouched model drafts that were removed.
    pub replaced: usize,
    /// Predictions that no longer fit the project.
    pub skipped: usize,
}
