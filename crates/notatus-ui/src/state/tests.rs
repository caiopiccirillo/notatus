use super::*;
use notatus_core::{
    AnnotationGeometry, AnnotationId, AnnotationRecord, AssetId, AssetStatus, BoundingBox, LabelId,
    Point, Polygon, ReviewState,
};

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
    assert!(state.is_dirty());
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
    assert!(state.is_dirty());
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
    assert!(!state.is_dirty());
}

#[test]
fn renames_project_and_marks_dirty() {
    let mut state = UiState::new_project("demo");
    state.mark_saved();
    let old_updated_at = state.dataset.manifest.project.updated_at;

    state.rename_project("renamed").unwrap();

    assert_eq!(state.dataset.manifest.project.name, "renamed");
    assert!(state.dataset.manifest.project.updated_at >= old_updated_at);
    assert!(state.is_dirty());
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
    assert!(!state.is_dirty());
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
    assert!(state.is_dirty());
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
    assert!(state.is_dirty());
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
    assert!(state.is_dirty());
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
    assert!(state.is_dirty());
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
    assert!(state.is_dirty());
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
    assert!(state.is_dirty());

    state.undo();
    assert!(!state.is_dirty());

    state.redo();
    assert!(state.is_dirty());
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

    assert!(!state.is_dirty());
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

fn draft_box(state: &mut UiState, asset: AssetId, label: LabelId, x: f64) -> AnnotationId {
    state
        .add_human_bbox(
            asset,
            label,
            BoundingBox::from_xywh(x, 10.0, 10.0, 10.0).unwrap(),
            None,
        )
        .unwrap()
}

/// Three media: the first has a draft, an accepted box, and a draft; the
/// second has nothing; the third has one draft.
fn review_project() -> (UiState, [AssetId; 3], [AnnotationId; 3]) {
    let mut state = UiState::new_project("demo");
    let label = state.add_label("car");
    let assets = [
        state.add_local_image_asset("/tmp/a.png", 100, 100).unwrap(),
        state.add_local_image_asset("/tmp/b.png", 100, 100).unwrap(),
        state.add_local_image_asset("/tmp/c.png", 100, 100).unwrap(),
    ];
    let first = draft_box(&mut state, assets[0], label, 0.0);
    let accepted = draft_box(&mut state, assets[0], label, 20.0);
    state
        .set_annotation_review_state(accepted, ReviewState::Accepted)
        .unwrap();
    let second = draft_box(&mut state, assets[0], label, 40.0);
    let third = draft_box(&mut state, assets[2], label, 0.0);
    state.select_asset(assets[0]).unwrap();
    (state, assets, [first, second, third])
}

#[test]
fn the_review_queue_visits_drafts_across_media_and_wraps() {
    let (mut state, _, [first, second, third]) = review_project();
    assert_eq!(state.review_queue_len(), 3);

    let mut visited = Vec::new();
    for _ in 0..4 {
        let next = state.adjacent_review_draft(Direction::Next).unwrap();
        state.select_annotation(Some(next)).unwrap();
        visited.push(next);
    }
    assert_eq!(visited, [first, second, third, first]);

    assert_eq!(
        state.adjacent_review_draft(Direction::Previous),
        Some(third)
    );
}

#[test]
fn without_a_selected_annotation_the_queue_starts_at_the_selected_media() {
    let (mut state, assets, [_, second, third]) = review_project();

    state.select_asset(assets[1]).unwrap();
    assert_eq!(state.adjacent_review_draft(Direction::Next), Some(third));
    assert_eq!(
        state.adjacent_review_draft(Direction::Previous),
        Some(second)
    );

    state.select_asset(assets[2]).unwrap();
    assert_eq!(state.adjacent_review_draft(Direction::Next), Some(third));
    assert_eq!(
        state.adjacent_review_draft(Direction::Previous),
        Some(third)
    );
}

#[test]
fn the_only_draft_left_has_nowhere_to_go() {
    let (mut state, _, [first, second, third]) = review_project();
    for id in [first, second] {
        state
            .set_annotation_review_state(id, ReviewState::Rejected)
            .unwrap();
    }
    state.select_annotation(Some(third)).unwrap();

    assert_eq!(state.review_queue_len(), 1);
    assert_eq!(state.adjacent_review_draft(Direction::Next), None);
}

#[test]
fn the_review_filter_narrows_the_queue_by_label_and_confidence() {
    let mut state = UiState::new_project("demo");
    let car = state.add_label("car");
    let person = state.add_label("person");
    let asset = state.add_local_image_asset("/tmp/a.png", 100, 100).unwrap();
    let mut unsure = suggestion(asset, car, 0.0);
    unsure.confidence = Some(0.3);
    let unsure_id = unsure.id;
    let sure = suggestion(asset, car, 20.0);
    let unsure_person = {
        let mut annotation = suggestion(asset, person, 40.0);
        annotation.confidence = Some(0.2);
        annotation
    };
    state
        .apply_model_predictions(asset, vec![unsure, sure, unsure_person])
        .unwrap();
    let drawn = draft_box(&mut state, asset, car, 60.0);
    state.select_annotation(None).unwrap();
    assert_eq!(state.review_queue_len(), 4);

    state.set_review_filter(ReviewFilter {
        label: Some(car),
        below_confidence: None,
    });
    assert_eq!(state.review_queue_len(), 3);

    state.set_review_filter(ReviewFilter {
        label: None,
        below_confidence: Some(0.5),
    });
    assert_eq!(state.review_queue_len(), 2);
    assert_eq!(
        state.adjacent_review_draft(Direction::Next),
        Some(unsure_id)
    );

    state.set_review_filter(ReviewFilter {
        label: Some(car),
        below_confidence: Some(0.5),
    });
    assert_eq!(state.review_queue_len(), 1);
    // Reviewing moves within the media only to drafts the filter keeps.
    assert_eq!(state.next_draft_annotation(drawn), Some(unsure_id));
    assert_eq!(state.next_draft_annotation(unsure_id), None);
}

#[test]
fn removing_the_filtered_label_clears_it_from_the_filter() {
    let mut state = UiState::new_project("demo");
    let car = state.add_label("car");
    state.set_review_filter(ReviewFilter {
        label: Some(car),
        below_confidence: Some(0.5),
    });

    state.remove_label(car).unwrap();

    assert_eq!(
        state.review_filter(),
        ReviewFilter {
            label: None,
            below_confidence: Some(0.5),
        }
    );
}

#[test]
fn separate_projects_never_share_a_revision() {
    // Views cache what they show by revision, so a project opened in place
    // of another must not look unchanged.
    let first = UiState::new_project("First");
    let second = UiState::new_project("Second");
    let reopened = UiState::from_dataset(first.dataset().clone()).unwrap();

    assert_ne!(first.revision(), second.revision());
    assert_ne!(first.revision(), reopened.revision());
    assert!(!second.is_dirty() && !reopened.is_dirty());
}

#[test]
fn the_index_follows_edits_undo_and_the_review_filter() {
    let mut state = UiState::new_project("demo");
    let car = state.add_label("car");
    let person = state.add_label("person");
    let first = state
        .add_local_image_asset("images/a.jpg", 640, 480)
        .unwrap();
    let second = state
        .add_local_image_asset("images/b.jpg", 640, 480)
        .unwrap();
    let bbox = || BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap();
    let a = state.add_human_bbox(first, car, bbox(), None).unwrap();
    let b = state.add_human_bbox(first, person, bbox(), None).unwrap();
    let c = state.add_human_bbox(second, car, bbox(), None).unwrap();
    state
        .set_annotation_review_state(c, ReviewState::Accepted)
        .unwrap();
    state.set_asset_status(second, AssetStatus::Done).unwrap();

    let index = state.index();
    assert_eq!(index.annotation_count(first), 2);
    assert_eq!(index.annotation_count(second), 1);
    assert_eq!(index.draft_count(first), 2);
    assert_eq!(index.label_annotation_count(car), 2);
    assert_eq!(index.done_media, 1);
    assert_eq!(index.drafts, 2);
    assert_eq!(index.review_queue, 2);
    let on_first: Vec<_> = state.annotations_on(first).iter().map(|a| a.id).collect();
    assert_eq!(on_first, [a, b]);

    // Reading again without a change reuses the same index.
    assert!(std::rc::Rc::ptr_eq(&index, &state.index()));

    state.remove_annotation(a).unwrap();
    assert_eq!(state.index().annotation_count(first), 1);
    assert_eq!(state.index().label_annotation_count(car), 1);

    state.undo();
    assert_eq!(state.index().annotation_count(first), 2);

    state.set_review_filter(ReviewFilter {
        label: Some(person),
        below_confidence: None,
    });
    assert_eq!(state.index().review_queue, 1);
    assert_eq!(state.review_queue_len(), 1);
}
