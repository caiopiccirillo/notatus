use crate::history::{CoalesceKey, History, Selection, Snapshot};
use notatus_core::{
    AnnotationGeometry, AnnotationId, AnnotationRecord, AnnotationSource, AssetId, AssetLocation,
    AssetRecord, AssetStatus, BoundingBox, ClassificationId, ClassificationRecord, Dataset,
    GeometryError, LabelId, Point, Polygon, ReviewState, ValidationError,
};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

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

    fn label_index(&self, label_id: LabelId) -> Result<usize, UiMutationError> {
        self.dataset
            .labels
            .iter()
            .position(|label| label.id == label_id)
            .ok_or(UiMutationError::MissingLabel { label_id })
    }

    fn annotation_index(&self, annotation_id: AnnotationId) -> Result<usize, UiMutationError> {
        self.dataset
            .annotations
            .iter()
            .position(|annotation| annotation.id == annotation_id)
            .ok_or(UiMutationError::MissingAnnotation { annotation_id })
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

    #[tracing::instrument(level = "debug", skip_all, fields(width = width, height = height))]
    pub fn add_local_image_asset(
        &mut self,
        path: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<AssetId, UiMutationError> {
        let path = path.into();
        let asset = AssetRecord::new_image(AssetLocation::local(path), width, height)?;
        self.begin_change("Import media", None);
        let asset_id = self.dataset.add_asset(asset);
        self.selected_asset = Some(asset_id);
        tracing::debug!(asset_id = %asset_id, "image asset added via UI");
        Ok(asset_id)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(asset_id = %asset_id))]
    pub fn select_asset(&mut self, asset_id: AssetId) -> Result<(), UiMutationError> {
        if self.dataset.asset_by_id(asset_id).is_none() {
            tracing::warn!(%asset_id, "selected missing asset");
            return Err(UiMutationError::MissingAsset { asset_id });
        }

        self.selected_asset = Some(asset_id);
        self.selected_annotation = None;
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all, fields(asset_id = %asset_id))]
    pub fn remove_asset(&mut self, asset_id: AssetId) -> Result<RemovalSummary, UiMutationError> {
        let asset_index = self
            .dataset
            .assets
            .iter()
            .position(|asset| asset.id == asset_id)
            .ok_or(UiMutationError::MissingAsset { asset_id })?;

        let selected_annotation_removed = self.selected_annotation.is_some_and(|annotation_id| {
            self.dataset
                .annotations
                .iter()
                .any(|annotation| annotation.id == annotation_id && annotation.asset_id == asset_id)
        });
        let original_annotation_count = self.dataset.annotations.len();
        let original_classification_count = self.dataset.classifications.len();

        self.begin_change("Remove media", None);
        self.dataset.assets.remove(asset_index);
        self.dataset
            .annotations
            .retain(|annotation| annotation.asset_id != asset_id);
        self.dataset
            .classifications
            .retain(|classification| classification.asset_id != asset_id);

        if self.selected_asset == Some(asset_id) {
            // Keep the user's place in the list: the next item, else the previous.
            let neighbor = asset_index.min(self.dataset.assets.len().saturating_sub(1));
            self.selected_asset = self.dataset.assets.get(neighbor).map(|asset| asset.id);
        }
        if selected_annotation_removed {
            self.selected_annotation = None;
        }

        let summary = RemovalSummary {
            annotations: original_annotation_count - self.dataset.annotations.len(),
            classifications: original_classification_count - self.dataset.classifications.len(),
        };
        tracing::info!(asset_id = %asset_id, removed_annotations = summary.annotations, removed_classifications = summary.classifications, "asset removed");
        Ok(summary)
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub fn select_annotation(
        &mut self,
        annotation_id: Option<AnnotationId>,
    ) -> Result<(), UiMutationError> {
        let Some(annotation_id) = annotation_id else {
            self.selected_annotation = None;
            return Ok(());
        };

        let annotation = self
            .dataset
            .annotations
            .iter()
            .find(|annotation| annotation.id == annotation_id)
            .ok_or(UiMutationError::MissingAnnotation { annotation_id })?;

        self.selected_asset = Some(annotation.asset_id);
        self.selected_annotation = Some(annotation_id);
        self.selected_label = Some(annotation.label_id);
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all, fields(annotation_id = %annotation_id, label_id = %label_id))]
    pub fn update_annotation_label(
        &mut self,
        annotation_id: AnnotationId,
        label_id: LabelId,
    ) -> Result<(), UiMutationError> {
        if self.dataset.label_by_id(label_id).is_none() {
            tracing::warn!(%label_id, "annotation update: missing label");
            return Err(UiMutationError::MissingLabel { label_id });
        }

        let index = self.annotation_index(annotation_id)?;
        if self.dataset.annotations[index].label_id != label_id {
            self.begin_change("Change annotation label", None);
            let annotation = &mut self.dataset.annotations[index];
            annotation.label_id = label_id;
            annotation.record_human_edit();
        }
        self.selected_annotation = Some(annotation_id);
        self.selected_asset = Some(self.dataset.annotations[index].asset_id);
        self.selected_label = Some(label_id);
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all, fields(annotation_id = %annotation_id))]
    pub fn update_annotation_bbox(
        &mut self,
        annotation_id: AnnotationId,
        bbox: BoundingBox,
    ) -> Result<(), UiMutationError> {
        let annotation_index = self
            .dataset
            .annotations
            .iter()
            .position(|annotation| annotation.id == annotation_id)
            .ok_or(UiMutationError::MissingAnnotation { annotation_id })?;
        let asset_id = self.dataset.annotations[annotation_index].asset_id;
        let label_id = self.dataset.annotations[annotation_index].label_id;
        let asset = self
            .dataset
            .asset_by_id(asset_id)
            .ok_or(UiMutationError::MissingAsset { asset_id })?;

        bbox.validate_within_image(asset.dimensions)?;

        let geometry = AnnotationGeometry::Bbox(bbox);
        if self.dataset.annotations[annotation_index].geometry != geometry {
            self.begin_change(
                "Edit annotation",
                Some(CoalesceKey::AnnotationGeometry(annotation_id)),
            );
            let annotation = &mut self.dataset.annotations[annotation_index];
            annotation.geometry = geometry;
            annotation.record_human_edit();
        }
        self.selected_annotation = Some(annotation_id);
        self.selected_asset = Some(asset_id);
        self.selected_label = Some(label_id);
        Ok(())
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

    #[tracing::instrument(level = "debug", skip_all, fields(annotation_id = %annotation_id))]
    pub fn remove_annotation(
        &mut self,
        annotation_id: AnnotationId,
    ) -> Result<(), UiMutationError> {
        let index = self.annotation_index(annotation_id)?;
        self.begin_change("Delete annotation", None);
        let annotation = self.dataset.annotations.remove(index);
        if self.selected_annotation == Some(annotation_id) {
            self.selected_annotation = None;
        }
        self.selected_asset = Some(annotation.asset_id);
        tracing::info!(%annotation_id, "annotation removed");
        Ok(())
    }

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

    #[tracing::instrument(level = "debug", skip_all, fields(asset_id = %asset_id))]
    pub fn set_asset_status(
        &mut self,
        asset_id: AssetId,
        status: AssetStatus,
    ) -> Result<(), UiMutationError> {
        let index = self
            .dataset
            .assets
            .iter()
            .position(|asset| asset.id == asset_id)
            .ok_or(UiMutationError::MissingAsset { asset_id })?;
        if self.dataset.assets[index].status == status {
            return Ok(());
        }
        let description = match status {
            AssetStatus::Done => "Mark media done",
            AssetStatus::Pending => "Mark media pending",
        };
        self.begin_change(description, None);
        self.dataset.assets[index].status = status;
        Ok(())
    }

    /// The asset before or after `from` in dataset order. With `pending_only`,
    /// skips media already marked done.
    pub fn adjacent_asset(
        &self,
        from: Option<AssetId>,
        direction: Direction,
        pending_only: bool,
    ) -> Option<AssetId> {
        let assets = &self.dataset.assets;
        let start = from.and_then(|id| assets.iter().position(|asset| asset.id == id));
        let eligible = |asset: &&AssetRecord| !pending_only || asset.status == AssetStatus::Pending;
        match (direction, start) {
            (Direction::Next, Some(index)) => assets[index + 1..].iter().find(eligible),
            (Direction::Previous, Some(index)) => assets[..index].iter().rev().find(eligible),
            (Direction::Next, None) => assets.iter().find(eligible),
            (Direction::Previous, None) => assets.iter().rev().find(eligible),
        }
        .map(|asset| asset.id)
    }

    /// The annotation before or after the selected one on the current asset,
    /// wrapping around.
    pub fn adjacent_annotation(&self, direction: Direction) -> Option<AnnotationId> {
        let asset_id = self.selected_asset?;
        let ids: Vec<_> = self
            .dataset
            .annotations
            .iter()
            .filter(|annotation| annotation.asset_id == asset_id)
            .map(|annotation| annotation.id)
            .collect();
        if ids.is_empty() {
            return None;
        }
        let current = self
            .selected_annotation
            .and_then(|id| ids.iter().position(|candidate| *candidate == id));
        let index = match (direction, current) {
            (Direction::Next, Some(index)) => (index + 1) % ids.len(),
            (Direction::Previous, Some(index)) => (index + ids.len() - 1) % ids.len(),
            (Direction::Next, None) => 0,
            (Direction::Previous, None) => ids.len() - 1,
        };
        Some(ids[index])
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

    #[tracing::instrument(level = "debug", skip_all, fields(asset_id = %asset_id, label_id = %label_id))]
    pub fn add_human_bbox(
        &mut self,
        asset_id: AssetId,
        label_id: LabelId,
        bbox: BoundingBox,
        user_id: Option<String>,
    ) -> Result<AnnotationId, UiMutationError> {
        let asset = self
            .dataset
            .asset_by_id(asset_id)
            .ok_or(UiMutationError::MissingAsset { asset_id })?;
        if self.dataset.label_by_id(label_id).is_none() {
            return Err(UiMutationError::MissingLabel { label_id });
        }
        bbox.validate_within_image(asset.dimensions)?;

        let annotation = AnnotationRecord::new_human(
            asset_id,
            label_id,
            AnnotationGeometry::Bbox(bbox),
            user_id,
        );
        let annotation_id = annotation.id;
        self.begin_change("Draw box", None);
        self.dataset.add_annotation(annotation);
        self.selected_annotation = None;
        self.selected_label = Some(label_id);
        tracing::info!(%annotation_id, asset_id = %asset_id, label_id = %label_id, "human bbox annotation created");
        Ok(annotation_id)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(asset_id = %asset_id, label_id = %label_id))]
    pub fn add_human_polygon(
        &mut self,
        asset_id: AssetId,
        label_id: LabelId,
        polygon: Polygon,
        user_id: Option<String>,
    ) -> Result<AnnotationId, UiMutationError> {
        let asset = self
            .dataset
            .asset_by_id(asset_id)
            .ok_or(UiMutationError::MissingAsset { asset_id })?;
        if self.dataset.label_by_id(label_id).is_none() {
            return Err(UiMutationError::MissingLabel { label_id });
        }
        polygon.validate_within_image(asset.dimensions)?;

        let annotation = AnnotationRecord::new_human(
            asset_id,
            label_id,
            AnnotationGeometry::Polygon(polygon),
            user_id,
        );
        let annotation_id = annotation.id;
        self.begin_change("Draw polygon", None);
        self.dataset.add_annotation(annotation);
        self.selected_annotation = None;
        self.selected_label = Some(label_id);
        tracing::info!(%annotation_id, asset_id = %asset_id, label_id = %label_id, "human polygon annotation created");
        Ok(annotation_id)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(asset_id = %asset_id, label_id = %label_id))]
    pub fn toggle_image_classification(
        &mut self,
        asset_id: AssetId,
        label_id: LabelId,
    ) -> Result<Option<ClassificationId>, UiMutationError> {
        if self.dataset.asset_by_id(asset_id).is_none() {
            return Err(UiMutationError::MissingAsset { asset_id });
        }
        if self.dataset.label_by_id(label_id).is_none() {
            return Err(UiMutationError::MissingLabel { label_id });
        }

        if let Some(index) = self
            .dataset
            .classifications
            .iter()
            .position(|classification| {
                classification.asset_id == asset_id && classification.label_id == label_id
            })
        {
            self.begin_change("Remove classification", None);
            self.dataset.classifications.remove(index);
            self.selected_label = Some(label_id);
            tracing::debug!(asset_id = %asset_id, label_id = %label_id, "classification toggled off");
            return Ok(None);
        }

        let classification = ClassificationRecord::new_human(asset_id, label_id, None);
        let classification_id = classification.id;
        self.begin_change("Add classification", None);
        self.dataset.add_classification(classification);
        self.selected_asset = Some(asset_id);
        self.selected_label = Some(label_id);
        tracing::debug!(%classification_id, asset_id = %asset_id, label_id = %label_id, "classification toggled on");
        Ok(Some(classification_id))
    }

    #[tracing::instrument(level = "debug", skip_all, fields(classification_id = %classification_id))]
    pub fn remove_classification(
        &mut self,
        classification_id: ClassificationId,
    ) -> Result<(), UiMutationError> {
        let classification_index = self
            .dataset
            .classifications
            .iter()
            .position(|classification| classification.id == classification_id)
            .ok_or(UiMutationError::MissingClassification { classification_id })?;

        self.begin_change("Remove classification", None);
        let classification = self.dataset.classifications.remove(classification_index);
        self.selected_asset = Some(classification.asset_id);
        self.selected_label = Some(classification.label_id);
        tracing::debug!(%classification_id, "classification removed");
        Ok(())
    }

    pub fn polygon_from_points(points: &[(f64, f64)]) -> Result<Polygon, UiMutationError> {
        let points = points
            .iter()
            .map(|(x, y)| Point::new(*x, *y))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Polygon::new(points)?)
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

#[derive(Debug)]
pub enum UiMutationError {
    Geometry(GeometryError),
    Validation(ValidationError),
    MissingAsset { asset_id: AssetId },
    MissingAnnotation { annotation_id: AnnotationId },
    MissingClassification { classification_id: ClassificationId },
    MissingLabel { label_id: LabelId },
    EmptyProjectName,
    EmptyLabelName { label_id: LabelId },
}

impl fmt::Display for UiMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geometry(source) => write!(f, "{source}"),
            Self::Validation(source) => write!(f, "{source}"),
            Self::MissingAsset { asset_id } => write!(f, "missing asset {asset_id}"),
            Self::MissingAnnotation { annotation_id } => {
                write!(f, "missing annotation {annotation_id}")
            }
            Self::MissingClassification { classification_id } => {
                write!(f, "missing classification {classification_id}")
            }
            Self::MissingLabel { label_id } => write!(f, "missing label {label_id}"),
            Self::EmptyProjectName => write!(f, "project needs a name"),
            Self::EmptyLabelName { label_id } => write!(f, "label {label_id} needs a name"),
        }
    }
}

impl Error for UiMutationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Geometry(source) => Some(source),
            Self::Validation(source) => Some(source),
            Self::MissingAsset { .. }
            | Self::MissingAnnotation { .. }
            | Self::MissingClassification { .. }
            | Self::MissingLabel { .. }
            | Self::EmptyProjectName
            | Self::EmptyLabelName { .. } => None,
        }
    }
}

impl From<GeometryError> for UiMutationError {
    fn from(source: GeometryError) -> Self {
        Self::Geometry(source)
    }
}

impl From<ValidationError> for UiMutationError {
    fn from(source: ValidationError) -> Self {
        Self::Validation(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_bbox_through_ui_state() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("car");
        let asset_id = state
            .add_local_image_asset("images/a.jpg", 640, 480)
            .unwrap();
        let annotation_id = state
            .add_human_bbox(
                asset_id,
                label_id,
                BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();

        assert!(
            state
                .dataset
                .annotations
                .iter()
                .any(|annotation| annotation.id == annotation_id)
        );
        assert_eq!(state.selected_annotation, None);
        assert_eq!(state.selected_label, Some(label_id));
        assert!(state.dirty);
        assert!(state.dataset.validate().is_ok());
    }

    #[test]
    fn adds_polygon_through_ui_state() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("car");
        let asset_id = state
            .add_local_image_asset("images/a.jpg", 640, 480)
            .unwrap();
        let polygon = Polygon::new(vec![
            Point::new(10.0, 20.0).unwrap(),
            Point::new(60.0, 20.0).unwrap(),
            Point::new(60.0, 70.0).unwrap(),
            Point::new(10.0, 70.0).unwrap(),
        ])
        .unwrap();

        let annotation_id = state
            .add_human_polygon(asset_id, label_id, polygon.clone(), None)
            .unwrap();

        let annotation = state
            .dataset
            .annotations
            .iter()
            .find(|annotation| annotation.id == annotation_id)
            .unwrap();
        assert_eq!(annotation.geometry, AnnotationGeometry::Polygon(polygon));
        assert_eq!(state.selected_annotation, None);
        assert_eq!(state.selected_label, Some(label_id));
        assert!(state.dirty);
        assert!(state.dataset.validate().is_ok());
    }

    #[test]
    fn toggles_image_classification() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("outdoor");
        let asset_id = state
            .add_local_image_asset("images/a.jpg", 640, 480)
            .unwrap();

        let classification_id = state
            .toggle_image_classification(asset_id, label_id)
            .unwrap()
            .unwrap();

        assert!(
            state
                .dataset
                .classifications
                .iter()
                .any(|classification| classification.id == classification_id)
        );
        assert_eq!(state.selected_asset, Some(asset_id));
        assert_eq!(state.selected_label, Some(label_id));

        assert_eq!(
            state
                .toggle_image_classification(asset_id, label_id)
                .unwrap(),
            None
        );
        assert!(state.dataset.classifications.is_empty());
        assert!(state.dataset.validate().is_ok());
    }

    #[test]
    fn setting_tool_does_not_dirty_dataset() {
        let mut state = UiState::new_project("demo");

        state.set_tool(AnnotationTool::DrawBox);

        assert_eq!(state.active_tool, AnnotationTool::DrawBox);
        assert!(!state.dirty);
    }

    #[test]
    fn renames_project_and_marks_dirty() {
        let mut state = UiState::new_project("demo");
        state.mark_saved();
        let old_updated_at = state.dataset.manifest.project.updated_at;

        state.rename_project("renamed").unwrap();

        assert_eq!(state.dataset.manifest.project.name, "renamed");
        assert!(state.dataset.manifest.project.updated_at >= old_updated_at);
        assert!(state.dirty);
    }

    #[test]
    fn rejects_empty_project_name() {
        let mut state = UiState::new_project("demo");

        let error = state.rename_project(" ").unwrap_err();

        assert!(matches!(error, UiMutationError::EmptyProjectName));
        assert_eq!(state.dataset.manifest.project.name, "demo");
    }

    #[test]
    fn selects_annotation_without_dirtying_dataset() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("car");
        let asset_id = state
            .add_local_image_asset("images/a.jpg", 640, 480)
            .unwrap();
        let annotation_id = state
            .add_human_bbox(
                asset_id,
                label_id,
                BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();
        state.mark_saved();

        state.select_annotation(Some(annotation_id)).unwrap();

        assert_eq!(state.selected_asset, Some(asset_id));
        assert_eq!(state.selected_label, Some(label_id));
        assert_eq!(state.selected_annotation, Some(annotation_id));
        assert!(!state.dirty);
    }

    #[test]
    fn selecting_asset_preserves_active_label() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("vehicle");
        let asset_id = state
            .add_local_image_asset("/tmp/image.png", 640, 480)
            .unwrap();
        state.select_label(label_id).unwrap();

        state.select_asset(asset_id).unwrap();

        assert_eq!(state.selected_label, Some(label_id));
        assert_eq!(state.selected_asset, Some(asset_id));
    }

    #[test]
    fn customizes_selected_label() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("vehicle");

        state.select_label(label_id).unwrap();
        state.update_label_name(label_id, "car").unwrap();
        state
            .update_label_color(label_id, Some("#dc2626".to_string()))
            .unwrap();

        let label = state.dataset.label_by_id(label_id).unwrap();
        assert_eq!(state.selected_label, Some(label_id));
        assert_eq!(label.name, "car");
        assert_eq!(label.color.as_deref(), Some("#dc2626"));
        assert!(state.dirty);
    }

    #[test]
    fn updates_annotation_label_and_marks_dirty() {
        let mut state = UiState::new_project("demo");
        let old_label = state.add_label("vehicle");
        let new_label = state.add_label("pedestrian");
        let asset_id = state
            .add_local_image_asset("images/a.jpg", 640, 480)
            .unwrap();
        let annotation_id = state
            .add_human_bbox(
                asset_id,
                old_label,
                BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();
        state.mark_saved();

        state
            .update_annotation_label(annotation_id, new_label)
            .unwrap();

        let annotation = state
            .dataset
            .annotations
            .iter()
            .find(|annotation| annotation.id == annotation_id)
            .unwrap();
        assert_eq!(annotation.label_id, new_label);
        assert_eq!(state.selected_asset, Some(asset_id));
        assert_eq!(state.selected_annotation, Some(annotation_id));
        assert_eq!(state.selected_label, Some(new_label));
        assert!(state.dirty);
    }

    #[test]
    fn updates_annotation_bbox_and_marks_dirty() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("vehicle");
        let asset_id = state
            .add_local_image_asset("images/a.jpg", 640, 480)
            .unwrap();
        let annotation_id = state
            .add_human_bbox(
                asset_id,
                label_id,
                BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();
        state.mark_saved();

        let bbox = BoundingBox::from_xywh(50.0, 60.0, 70.0, 80.0).unwrap();
        state.update_annotation_bbox(annotation_id, bbox).unwrap();

        let annotation = state
            .dataset
            .annotations
            .iter()
            .find(|annotation| annotation.id == annotation_id)
            .unwrap();
        assert_eq!(annotation.geometry, AnnotationGeometry::Bbox(bbox));
        assert_eq!(state.selected_asset, Some(asset_id));
        assert_eq!(state.selected_annotation, Some(annotation_id));
        assert_eq!(state.selected_label, Some(label_id));
        assert!(state.dirty);
    }

    #[test]
    fn rejects_annotation_bbox_outside_image_bounds() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("vehicle");
        let asset_id = state
            .add_local_image_asset("images/a.jpg", 100, 100)
            .unwrap();
        let annotation_id = state
            .add_human_bbox(
                asset_id,
                label_id,
                BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();
        let original_geometry = state.dataset.annotations[0].geometry.clone();

        let error = state
            .update_annotation_bbox(
                annotation_id,
                BoundingBox::from_xywh(90.0, 90.0, 20.0, 20.0).unwrap(),
            )
            .unwrap_err();

        assert!(matches!(error, UiMutationError::Geometry(_)));
        assert_eq!(state.dataset.annotations[0].geometry, original_geometry);
    }

    #[test]
    fn rejects_annotation_label_update_with_missing_label() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("vehicle");
        let asset_id = state
            .add_local_image_asset("images/a.jpg", 640, 480)
            .unwrap();
        let annotation_id = state
            .add_human_bbox(
                asset_id,
                label_id,
                BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();

        let error = state
            .update_annotation_label(annotation_id, LabelId::new())
            .unwrap_err();

        assert!(matches!(error, UiMutationError::MissingLabel { .. }));
        assert_eq!(
            state
                .dataset
                .annotations
                .iter()
                .find(|annotation| annotation.id == annotation_id)
                .unwrap()
                .label_id,
            label_id
        );
    }

    #[test]
    fn rejects_empty_label_name() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("vehicle");

        let error = state.update_label_name(label_id, " ").unwrap_err();

        assert!(matches!(error, UiMutationError::EmptyLabelName { .. }));
        assert_eq!(state.dataset.label_by_id(label_id).unwrap().name, "vehicle");
    }

    #[test]
    fn removes_label_and_its_annotations() {
        let mut state = UiState::new_project("demo");
        let removed_label = state.add_label("vehicle");
        let remaining_label = state.add_label("person");
        let asset_id = state
            .add_local_image_asset("images/a.jpg", 640, 480)
            .unwrap();
        let removed_annotation = state
            .add_human_bbox(
                asset_id,
                removed_label,
                BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();
        let remaining_annotation = state
            .add_human_bbox(
                asset_id,
                remaining_label,
                BoundingBox::from_xywh(60.0, 70.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();
        state.select_annotation(Some(removed_annotation)).unwrap();
        state.mark_saved();

        state
            .toggle_image_classification(asset_id, removed_label)
            .unwrap();
        state
            .toggle_image_classification(asset_id, remaining_label)
            .unwrap();

        let removed = state.remove_label(removed_label).unwrap();

        assert_eq!(removed.annotations, 1);
        assert_eq!(removed.classifications, 1);
        assert!(state.dataset.label_by_id(removed_label).is_none());
        assert_eq!(state.selected_label, Some(remaining_label));
        assert_eq!(state.selected_annotation, None);
        assert!(
            state
                .dataset
                .annotations
                .iter()
                .all(|annotation| annotation.label_id != removed_label)
        );
        assert!(
            state
                .dataset
                .annotations
                .iter()
                .any(|annotation| annotation.id == remaining_annotation)
        );
        assert!(
            state
                .dataset
                .classifications
                .iter()
                .all(|classification| classification.label_id != removed_label)
        );
        assert!(
            state
                .dataset
                .classifications
                .iter()
                .any(|classification| classification.label_id == remaining_label)
        );
        assert!(state.dirty);
        assert!(state.dataset.validate().is_ok());
    }

    #[test]
    fn removes_asset_and_its_annotations() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("vehicle");
        let removed_asset = state
            .add_local_image_asset("images/a.jpg", 640, 480)
            .unwrap();
        let remaining_asset = state
            .add_local_image_asset("images/b.jpg", 640, 480)
            .unwrap();
        let removed_annotation = state
            .add_human_bbox(
                removed_asset,
                label_id,
                BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();
        let remaining_annotation = state
            .add_human_bbox(
                remaining_asset,
                label_id,
                BoundingBox::from_xywh(60.0, 70.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();
        state.select_annotation(Some(removed_annotation)).unwrap();
        state.mark_saved();

        state
            .toggle_image_classification(removed_asset, label_id)
            .unwrap();
        state
            .toggle_image_classification(remaining_asset, label_id)
            .unwrap();

        let removed = state.remove_asset(removed_asset).unwrap();

        assert_eq!(removed.annotations, 1);
        assert_eq!(removed.classifications, 1);
        assert!(state.dataset.asset_by_id(removed_asset).is_none());
        assert_eq!(state.selected_asset, Some(remaining_asset));
        assert_eq!(state.selected_annotation, None);
        assert!(
            state
                .dataset
                .annotations
                .iter()
                .all(|annotation| annotation.asset_id != removed_asset)
        );
        assert!(
            state
                .dataset
                .annotations
                .iter()
                .any(|annotation| annotation.id == remaining_annotation)
        );
        assert!(
            state
                .dataset
                .classifications
                .iter()
                .all(|classification| classification.asset_id != removed_asset)
        );
        assert!(
            state
                .dataset
                .classifications
                .iter()
                .any(|classification| classification.asset_id == remaining_asset)
        );
        assert!(state.dirty);
        assert!(state.dataset.validate().is_ok());
    }

    fn demo_with_box() -> (UiState, LabelId, AssetId, AnnotationId) {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("car");
        let asset_id = state
            .add_local_image_asset("images/a.jpg", 640, 480)
            .unwrap();
        let annotation_id = state
            .add_human_bbox(
                asset_id,
                label_id,
                BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();
        (state, label_id, asset_id, annotation_id)
    }

    #[test]
    fn undo_and_redo_restore_dataset_and_selection() {
        let (mut state, _, asset_id, annotation_id) = demo_with_box();
        state.select_annotation(Some(annotation_id)).unwrap();

        state.remove_annotation(annotation_id).unwrap();
        assert!(state.dataset.annotations.is_empty());
        assert_eq!(state.undo_description(), Some("Delete annotation"));

        assert_eq!(state.undo(), Some("Delete annotation"));
        assert_eq!(state.dataset.annotations.len(), 1);
        assert_eq!(state.selected_annotation, Some(annotation_id));
        assert_eq!(state.selected_asset, Some(asset_id));

        assert_eq!(state.redo(), Some("Delete annotation"));
        assert!(state.dataset.annotations.is_empty());
        assert_eq!(state.selected_annotation, None);
    }

    #[test]
    fn undo_back_to_saved_revision_clears_dirty() {
        let (mut state, _, _, annotation_id) = demo_with_box();
        state.mark_saved();

        state.remove_annotation(annotation_id).unwrap();
        assert!(state.dirty);

        state.undo();
        assert!(!state.dirty);

        state.redo();
        assert!(state.dirty);
    }

    #[test]
    fn drag_edits_coalesce_into_one_undo_step() {
        let (mut state, _, _, annotation_id) = demo_with_box();
        let original = state.dataset.annotations[0].geometry.clone();

        for offset in 1..=5 {
            let bbox = BoundingBox::from_xywh(10.0 + offset as f64, 20.0, 30.0, 40.0).unwrap();
            state.update_annotation_bbox(annotation_id, bbox).unwrap();
        }
        state.break_history_coalescing();

        assert_eq!(state.undo(), Some("Edit annotation"));
        assert_eq!(state.dataset.annotations[0].geometry, original);
        assert_eq!(state.undo(), Some("Draw box"));
    }

    #[test]
    fn unchanged_edits_do_not_dirty_or_record_history() {
        let (mut state, label_id, _, annotation_id) = demo_with_box();
        state.mark_saved();
        let bbox = BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap();

        state.update_annotation_bbox(annotation_id, bbox).unwrap();
        state
            .update_annotation_label(annotation_id, label_id)
            .unwrap();

        assert!(!state.dirty);
        assert_eq!(state.undo_description(), Some("Draw box"));
    }

    #[test]
    fn batch_groups_changes_into_one_step() {
        let mut state = UiState::new_project("demo");
        state.batch("Import media", |state| {
            state.add_local_image_asset("a.jpg", 10, 10).unwrap();
            state.add_local_image_asset("b.jpg", 10, 10).unwrap();
        });

        assert_eq!(state.undo(), Some("Import media"));
        assert!(state.dataset.assets.is_empty());
        assert_eq!(state.undo(), None);
    }

    #[test]
    fn marks_asset_done_and_navigates_pending() {
        let mut state = UiState::new_project("demo");
        let a = state.add_local_image_asset("a.jpg", 10, 10).unwrap();
        let b = state.add_local_image_asset("b.jpg", 10, 10).unwrap();
        let c = state.add_local_image_asset("c.jpg", 10, 10).unwrap();

        state.set_asset_status(b, AssetStatus::Done).unwrap();

        assert_eq!(
            state.adjacent_asset(Some(a), Direction::Next, false),
            Some(b)
        );
        assert_eq!(
            state.adjacent_asset(Some(a), Direction::Next, true),
            Some(c)
        );
        assert_eq!(
            state.adjacent_asset(Some(c), Direction::Previous, true),
            Some(a)
        );
        assert_eq!(state.adjacent_asset(Some(c), Direction::Next, false), None);
        assert_eq!(state.adjacent_asset(None, Direction::Next, true), Some(a));
        assert_eq!(state.undo(), Some("Mark media done"));
    }

    #[test]
    fn cycles_annotations_on_current_asset() {
        let (mut state, label_id, asset_id, first) = demo_with_box();
        let second = state
            .add_human_bbox(
                asset_id,
                label_id,
                BoundingBox::from_xywh(100.0, 100.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();

        assert_eq!(state.adjacent_annotation(Direction::Next), Some(first));
        state.select_annotation(Some(first)).unwrap();
        assert_eq!(state.adjacent_annotation(Direction::Next), Some(second));
        assert_eq!(state.adjacent_annotation(Direction::Previous), Some(second));
    }

    #[test]
    fn removing_selected_asset_selects_its_neighbor() {
        let mut state = UiState::new_project("demo");
        let a = state.add_local_image_asset("a.jpg", 10, 10).unwrap();
        let b = state.add_local_image_asset("b.jpg", 10, 10).unwrap();
        let c = state.add_local_image_asset("c.jpg", 10, 10).unwrap();

        state.select_asset(b).unwrap();
        state.remove_asset(b).unwrap();
        assert_eq!(state.selected_asset, Some(c));

        state.remove_asset(c).unwrap();
        assert_eq!(state.selected_asset, Some(a));
    }

    #[test]
    fn review_state_changes_undo_and_skip_reviewed_annotations() {
        let mut state = UiState::new_project("demo");
        let label = state.add_label("Person");
        let asset = state.add_local_image_asset("/tmp/a.png", 100, 100).unwrap();
        let ids: Vec<_> = (0..3)
            .map(|i| {
                let bbox = BoundingBox::from_xywh(i as f64 * 10.0, 0.0, 5.0, 5.0).unwrap();
                state.add_human_bbox(asset, label, bbox, None).unwrap()
            })
            .collect();

        state
            .set_annotation_review_state(ids[1], ReviewState::Accepted)
            .unwrap();
        assert_eq!(state.undo_description(), Some("Accept annotation"));
        // From the first, the next draft skips the accepted second one.
        assert_eq!(state.next_draft_annotation(ids[0]), Some(ids[2]));
        // Wraps around to the start.
        assert_eq!(state.next_draft_annotation(ids[2]), Some(ids[0]));

        let revision = state.revision();
        state
            .set_annotation_review_state(ids[1], ReviewState::Accepted)
            .unwrap();
        assert_eq!(state.revision(), revision, "unchanged state is a no-op");

        state.undo();
        assert_eq!(
            state.dataset.annotations[1].review_state,
            ReviewState::Draft
        );

        for id in &ids {
            state
                .set_annotation_review_state(*id, ReviewState::Rejected)
                .unwrap();
        }
        assert_eq!(state.next_draft_annotation(ids[0]), None);
    }

    #[test]
    fn editing_model_output_marks_it_human_edited_until_undone() {
        let mut state = UiState::new_project("demo");
        let car = state.add_label("Car");
        let truck = state.add_label("Truck");
        let asset = state.add_local_image_asset("/tmp/a.png", 100, 100).unwrap();
        let model = notatus_core::ModelProvenance {
            name: "detector".to_string(),
            version: None,
            invocation_id: None,
            metadata: Default::default(),
        };
        let predicted = AnnotationRecord::new_model(
            asset,
            car,
            AnnotationGeometry::Bbox(BoundingBox::from_xywh(10.0, 10.0, 20.0, 20.0).unwrap()),
            model,
            Some(0.8),
        );
        let id = predicted.id;
        state.dataset.add_annotation(predicted);

        state
            .update_annotation_bbox(id, BoundingBox::from_xywh(12.0, 10.0, 20.0, 20.0).unwrap())
            .unwrap();
        assert!(state.dataset.annotations[0].human_edited);

        state.undo();
        assert!(!state.dataset.annotations[0].human_edited);

        state.update_annotation_label(id, truck).unwrap();
        assert!(state.dataset.annotations[0].human_edited);
    }

    fn suggestion(asset: AssetId, label: LabelId, x: f64) -> AnnotationRecord {
        AnnotationRecord::new_model(
            asset,
            label,
            AnnotationGeometry::Bbox(BoundingBox::from_xywh(x, 10.0, 10.0, 10.0).unwrap()),
            notatus_core::ModelProvenance {
                name: "detector".to_string(),
                version: None,
                invocation_id: None,
                metadata: Default::default(),
            },
            Some(0.8),
        )
    }

    #[test]
    fn running_a_model_again_replaces_only_untouched_suggestions() {
        let mut state = UiState::new_project("demo");
        let car = state.add_label("Car");
        let asset = state.add_local_image_asset("/tmp/a.png", 100, 100).unwrap();
        let other = state.add_local_image_asset("/tmp/b.png", 100, 100).unwrap();
        state
            .apply_model_predictions(
                asset,
                vec![suggestion(asset, car, 0.0), suggestion(asset, car, 20.0)],
            )
            .unwrap();
        state
            .apply_model_predictions(other, vec![suggestion(other, car, 0.0)])
            .unwrap();
        // A person accepts one suggestion and moves the other.
        let first = state.dataset.annotations[0].id;
        let second = state.dataset.annotations[1].id;
        state
            .set_annotation_review_state(first, ReviewState::Accepted)
            .unwrap();
        state
            .update_annotation_bbox(
                second,
                BoundingBox::from_xywh(30.0, 10.0, 10.0, 10.0).unwrap(),
            )
            .unwrap();
        state
            .apply_model_predictions(asset, vec![suggestion(asset, car, 50.0)])
            .unwrap();

        let summary = state
            .apply_model_predictions(asset, vec![suggestion(asset, car, 60.0)])
            .unwrap();

        assert_eq!(
            summary,
            ModelPredictionSummary {
                added: 1,
                replaced: 1,
                skipped: 0
            }
        );
        let on_asset: Vec<_> = state
            .dataset
            .annotations
            .iter()
            .filter(|annotation| annotation.asset_id == asset)
            .collect();
        assert_eq!(
            on_asset.len(),
            3,
            "accepted, edited, and the new suggestion"
        );
        assert!(on_asset.iter().any(|annotation| annotation.id == first));
        assert!(on_asset.iter().any(|annotation| annotation.id == second));
        assert_eq!(
            state
                .dataset
                .annotations
                .iter()
                .filter(|a| a.asset_id == other)
                .count(),
            1,
            "other media keeps its suggestions"
        );
    }

    #[test]
    fn suggestions_are_one_undo_step_and_skip_removed_labels() {
        let mut state = UiState::new_project("demo");
        let car = state.add_label("Car");
        let gone = state.add_label("Gone");
        let asset = state.add_local_image_asset("/tmp/a.png", 100, 100).unwrap();
        state.remove_label(gone).unwrap();
        let before = state.dataset.annotations.len();

        let summary = state
            .apply_model_predictions(
                asset,
                vec![
                    suggestion(asset, car, 0.0),
                    suggestion(asset, car, 20.0),
                    suggestion(asset, gone, 40.0),
                ],
            )
            .unwrap();

        assert_eq!(
            summary,
            ModelPredictionSummary {
                added: 2,
                replaced: 0,
                skipped: 1
            }
        );
        assert_eq!(state.undo_description(), Some("Add model suggestions"));
        state.undo();
        assert_eq!(state.dataset.annotations.len(), before);
        assert!(state.dataset.validate().is_ok());
    }
}
