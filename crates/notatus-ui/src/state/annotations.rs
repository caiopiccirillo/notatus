//! Drawing, selecting, editing, and removing annotations.

use super::history::CoalesceKey;
use super::{Direction, UiMutationError, UiState};
use notatus_core::{
    AnnotationGeometry, AnnotationId, AnnotationRecord, AssetId, BoundingBox, LabelId, Point,
    Polygon,
};

impl UiState {
    pub(super) fn annotation_index(
        &self,
        annotation_id: AnnotationId,
    ) -> Result<usize, UiMutationError> {
        self.dataset
            .annotations
            .iter()
            .position(|annotation| annotation.id == annotation_id)
            .ok_or(UiMutationError::MissingAnnotation { annotation_id })
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

    pub fn polygon_from_points(points: &[(f64, f64)]) -> Result<Polygon, UiMutationError> {
        let points = points
            .iter()
            .map(|(x, y)| Point::new(*x, *y))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Polygon::new(points)?)
    }
}
