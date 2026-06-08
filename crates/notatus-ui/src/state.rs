use notatus_core::{
    AnnotationGeometry, AnnotationId, AnnotationRecord, AssetId, AssetLocation, AssetRecord,
    BoundingBox, ClassificationId, ClassificationRecord, Dataset, GeometryError, LabelId, Point,
    Polygon, ValidationError,
};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnnotationTool {
    Select,
    DrawBox,
    DrawPolygon,
    Pan,
}

impl AnnotationTool {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::DrawBox => "Draw Box",
            Self::DrawPolygon => "Draw Polygon",
            Self::Pan => "Pan/Zoom",
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
    pub dirty: bool,
}

impl UiState {
    pub fn new_project(name: impl Into<String>) -> Self {
        Self {
            dataset: Dataset::new(name),
            active_tool: AnnotationTool::Select,
            selected_asset: None,
            selected_annotation: None,
            selected_label: None,
            dirty: false,
        }
    }

    pub fn from_dataset(dataset: Dataset) -> Result<Self, UiMutationError> {
        dataset.validate()?;
        Ok(Self {
            dataset,
            active_tool: AnnotationTool::Select,
            selected_asset: None,
            selected_annotation: None,
            selected_label: None,
            dirty: false,
        })
    }

    pub fn set_tool(&mut self, tool: AnnotationTool) {
        self.active_tool = tool;
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn rename_project(&mut self, name: impl Into<String>) -> Result<(), UiMutationError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(UiMutationError::EmptyProjectName);
        }

        self.dataset.rename_project(name);
        self.dirty = true;
        Ok(())
    }

    pub fn add_label(&mut self, name: impl Into<String>) -> LabelId {
        let label_id = self.dataset.add_label(name);
        self.selected_label = Some(label_id);
        self.dirty = true;
        label_id
    }

    pub fn select_label(&mut self, label_id: LabelId) -> Result<(), UiMutationError> {
        if self.dataset.label_by_id(label_id).is_none() {
            return Err(UiMutationError::MissingLabel { label_id });
        }

        self.selected_label = Some(label_id);
        self.selected_annotation = None;
        Ok(())
    }

    pub fn update_label_name(
        &mut self,
        label_id: LabelId,
        name: impl Into<String>,
    ) -> Result<(), UiMutationError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(UiMutationError::EmptyLabelName { label_id });
        }

        let label = self
            .dataset
            .labels
            .iter_mut()
            .find(|label| label.id == label_id)
            .ok_or(UiMutationError::MissingLabel { label_id })?;
        label.name = name;
        self.dirty = true;
        Ok(())
    }

    pub fn update_label_color(
        &mut self,
        label_id: LabelId,
        color: Option<String>,
    ) -> Result<(), UiMutationError> {
        let label = self
            .dataset
            .labels
            .iter_mut()
            .find(|label| label.id == label_id)
            .ok_or(UiMutationError::MissingLabel { label_id })?;
        label.color = color;
        self.dirty = true;
        Ok(())
    }

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

        self.dirty = true;
        Ok(RemovalSummary {
            annotations: original_annotation_count - self.dataset.annotations.len(),
            classifications: original_classification_count - self.dataset.classifications.len(),
        })
    }

    pub fn add_local_image_asset(
        &mut self,
        path: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<AssetId, UiMutationError> {
        let asset = AssetRecord::new_image(AssetLocation::local(path), width, height)?;
        let asset_id = self.dataset.add_asset(asset);
        self.selected_asset = Some(asset_id);
        self.dirty = true;
        Ok(asset_id)
    }

    pub fn select_asset(&mut self, asset_id: AssetId) -> Result<(), UiMutationError> {
        if self.dataset.asset_by_id(asset_id).is_none() {
            return Err(UiMutationError::MissingAsset { asset_id });
        }

        self.selected_asset = Some(asset_id);
        self.selected_annotation = None;
        Ok(())
    }

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

        self.dataset.assets.remove(asset_index);
        self.dataset
            .annotations
            .retain(|annotation| annotation.asset_id != asset_id);
        self.dataset
            .classifications
            .retain(|classification| classification.asset_id != asset_id);

        if self.selected_asset == Some(asset_id) {
            self.selected_asset = self.dataset.assets.first().map(|asset| asset.id);
        }
        if selected_annotation_removed {
            self.selected_annotation = None;
        }

        self.dirty = true;
        Ok(RemovalSummary {
            annotations: original_annotation_count - self.dataset.annotations.len(),
            classifications: original_classification_count - self.dataset.classifications.len(),
        })
    }

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

    pub fn update_annotation_label(
        &mut self,
        annotation_id: AnnotationId,
        label_id: LabelId,
    ) -> Result<(), UiMutationError> {
        if self.dataset.label_by_id(label_id).is_none() {
            return Err(UiMutationError::MissingLabel { label_id });
        }

        let annotation = self
            .dataset
            .annotations
            .iter_mut()
            .find(|annotation| annotation.id == annotation_id)
            .ok_or(UiMutationError::MissingAnnotation { annotation_id })?;

        annotation.label_id = label_id;
        self.selected_annotation = Some(annotation_id);
        self.selected_asset = Some(annotation.asset_id);
        self.selected_label = Some(label_id);
        self.dirty = true;
        Ok(())
    }

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

        self.dataset.annotations[annotation_index].geometry = AnnotationGeometry::Bbox(bbox);
        self.selected_annotation = Some(annotation_id);
        self.selected_asset = Some(asset_id);
        self.selected_label = Some(label_id);
        self.dirty = true;
        Ok(())
    }

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
        self.dataset.add_annotation(annotation);
        self.selected_annotation = None;
        self.selected_label = Some(label_id);
        self.dirty = true;
        Ok(annotation_id)
    }

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
        self.dataset.add_annotation(annotation);
        self.selected_annotation = None;
        self.selected_label = Some(label_id);
        self.dirty = true;
        Ok(annotation_id)
    }

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
            self.dataset.classifications.remove(index);
            self.selected_label = Some(label_id);
            self.dirty = true;
            return Ok(None);
        }

        let classification = ClassificationRecord::new_human(asset_id, label_id, None);
        let classification_id = classification.id;
        self.dataset.add_classification(classification);
        self.selected_asset = Some(asset_id);
        self.selected_label = Some(label_id);
        self.dirty = true;
        Ok(Some(classification_id))
    }

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

        let classification = self.dataset.classifications.remove(classification_index);
        self.selected_asset = Some(classification.asset_id);
        self.selected_label = Some(classification.label_id);
        self.dirty = true;
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
}
