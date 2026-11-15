//! Reviewing annotations and applying model suggestions.

use super::{Direction, UiMutationError, UiState};
use notatus_core::{
    AnnotationId, AnnotationRecord, AnnotationSource, AssetId, LabelId, ReviewState,
};
use std::collections::{BTreeSet, HashMap};

/// Which drafts the review queue visits. The default visits every draft.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReviewFilter {
    /// Only drafts with this label.
    pub label: Option<LabelId>,
    /// Only model suggestions less confident than this, from 0 to 1. Shapes
    /// a person drew have no confidence, so this leaves them out.
    pub below_confidence: Option<f32>,
}

impl ReviewFilter {
    pub fn is_active(&self) -> bool {
        self.label.is_some() || self.below_confidence.is_some()
    }

    /// Whether the queue visits `annotation`: a draft that matches.
    pub fn accepts(&self, annotation: &AnnotationRecord) -> bool {
        annotation.review_state == ReviewState::Draft
            && self.label.is_none_or(|label| annotation.label_id == label)
            && self.below_confidence.is_none_or(|threshold| {
                annotation
                    .confidence
                    .is_some_and(|confidence| confidence < threshold)
            })
    }
}

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

    pub fn review_filter(&self) -> ReviewFilter {
        self.review_filter
    }

    pub fn set_review_filter(&mut self, filter: ReviewFilter) {
        self.review_filter = filter;
    }

    /// How many drafts the review queue still has to visit.
    pub fn review_queue_len(&self) -> usize {
        self.dataset
            .annotations
            .iter()
            .filter(|annotation| self.review_filter.accepts(annotation))
            .count()
    }

    /// The first draft in the review queue after `from` on the same media,
    /// wrapping around. `None` once the queue has nothing left there.
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
            .find(|annotation| self.review_filter.accepts(annotation))
            .map(|annotation| annotation.id)
    }

    /// The draft before or after the current position in the review queue,
    /// across all media, wrapping around.
    ///
    /// The queue visits media in list order and each media's annotations in
    /// the order they were made. The position is the selected annotation;
    /// with none selected, it is the start of the selected media going
    /// forward, or its end going back, so that media's drafts come first.
    /// The selected annotation itself is never returned.
    pub fn adjacent_review_draft(&self, direction: Direction) -> Option<AnnotationId> {
        let asset_order: HashMap<AssetId, usize> = self
            .dataset
            .assets
            .iter()
            .enumerate()
            .map(|(index, asset)| (asset.id, index))
            .collect();
        // Annotations sit at odd positions within their media, leaving the
        // even ones as gaps to start from.
        let position = |index: usize, annotation: &AnnotationRecord| {
            asset_order
                .get(&annotation.asset_id)
                .map(|asset| (*asset, 2 * index + 1))
        };
        let selected_position = self.selected_annotation.and_then(|id| {
            self.dataset
                .annotations
                .iter()
                .enumerate()
                .find(|(_, annotation)| annotation.id == id)
                .and_then(|(index, annotation)| position(index, annotation))
        });
        let media_position = self
            .selected_asset
            .and_then(|id| asset_order.get(&id).copied());
        let anchor = match (selected_position, media_position, direction) {
            (Some(position), _, _) => position,
            (None, Some(asset), Direction::Next) => (asset, 0),
            (None, Some(asset), Direction::Previous) => (asset, usize::MAX),
            (None, None, Direction::Next) => (0, 0),
            (None, None, Direction::Previous) => (usize::MAX, usize::MAX),
        };

        let mut queue: Vec<_> = self
            .dataset
            .annotations
            .iter()
            .enumerate()
            .filter(|(_, annotation)| {
                self.review_filter.accepts(annotation)
                    && Some(annotation.id) != self.selected_annotation
            })
            .filter_map(|(index, annotation)| Some((position(index, annotation)?, annotation.id)))
            .collect();
        queue.sort_unstable_by_key(|(position, _)| *position);

        let found = match direction {
            Direction::Next => queue
                .iter()
                .find(|(position, _)| *position > anchor)
                .or(queue.first()),
            Direction::Previous => queue
                .iter()
                .rev()
                .find(|(position, _)| *position < anchor)
                .or(queue.last()),
        };
        found.map(|(_, id)| *id)
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
