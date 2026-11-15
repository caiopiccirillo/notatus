//! The main window driven end to end: export, keyboard, palette, autosave,
//! drawing, labels, and review.

use super::support::open_test_window;
use crate::ReviewFilter;
use crate::app::*;
use gpui::TestAppContext;
use notatus_core::ReviewState;

#[gpui::test]
fn export_workflow_issue_tracks_required_steps(cx: &mut TestAppContext) {
    let (_, view) = open_test_window(cx);

    view.update(cx, |notatus, _| {
        assert_eq!(
            notatus.export_workflow_issue(),
            Some(export_commands::ExportWorkflowIssue::Labels)
        );

        let label_id = notatus.state.add_label("Person");
        assert_eq!(
            notatus.export_workflow_issue(),
            Some(export_commands::ExportWorkflowIssue::Media)
        );

        let asset_id = notatus
            .state
            .add_local_image_asset("/tmp/notatus-test.png", 100, 100)
            .unwrap();
        assert_eq!(
            notatus.export_workflow_issue(),
            Some(export_commands::ExportWorkflowIssue::Annotations)
        );

        let bbox = BoundingBox::from_xywh(10.0, 10.0, 20.0, 20.0).unwrap();
        notatus
            .state
            .add_human_bbox(asset_id, label_id, bbox, None)
            .unwrap();
        assert_eq!(
            notatus.export_workflow_issue(),
            Some(export_commands::ExportWorkflowIssue::FinishedMedia)
        );

        notatus
            .state
            .set_asset_status(asset_id, notatus_core::AssetStatus::Done)
            .unwrap();
        assert_eq!(notatus.export_workflow_issue(), None);

        notatus.export.yolo = false;
        notatus.export.coco = false;
        notatus.export.classifications = false;
        assert_eq!(
            notatus.export_workflow_issue(),
            Some(export_commands::ExportWorkflowIssue::Format)
        );
    });
}

#[gpui::test]
fn classification_only_project_can_export(cx: &mut TestAppContext) {
    let (_, view) = open_test_window(cx);

    view.update(cx, |notatus, _| {
        let label_id = notatus.state.add_label("Outdoor");
        let asset_id = notatus
            .state
            .add_local_image_asset("/tmp/notatus-test.png", 100, 100)
            .unwrap();
        notatus
            .state
            .toggle_image_classification(asset_id, label_id)
            .unwrap();
        notatus
            .state
            .set_asset_status(asset_id, notatus_core::AssetStatus::Done)
            .unwrap();

        notatus.export.yolo = false;
        notatus.export.coco = false;
        notatus.export.classifications = true;

        assert_eq!(notatus.export_workflow_issue(), None);
    });
}

#[gpui::test]
fn export_format_toggles_keep_at_least_one_format(cx: &mut TestAppContext) {
    let (_, view) = open_test_window(cx);

    view.update(cx, |notatus, cx| {
        assert!(notatus.export.yolo);
        assert!(notatus.export.coco);
        assert!(notatus.export.classifications);

        notatus.toggle_export_format(export_commands::ExportFormat::Yolo, cx);
        assert!(!notatus.export.yolo);
        assert!(notatus.export.coco);
        assert!(notatus.export.classifications);

        notatus.toggle_export_format(export_commands::ExportFormat::Coco, cx);
        assert!(!notatus.export.yolo);
        assert!(!notatus.export.coco);
        assert!(notatus.export.classifications);
        assert_eq!(notatus.status_message, None);

        notatus.toggle_export_format(export_commands::ExportFormat::Classification, cx);
        assert!(!notatus.export.yolo);
        assert!(!notatus.export.coco);
        assert!(notatus.export.classifications);
        assert_eq!(
            notatus.status_message.as_deref(),
            Some("Select an export format")
        );

        notatus.toggle_export_format(export_commands::ExportFormat::Yolo, cx);
        assert!(notatus.export.yolo);
        assert!(!notatus.export.coco);
        assert!(notatus.export.classifications);
        assert_eq!(notatus.status_message, None);
    });
}

#[gpui::test]
fn empty_project_export_pushes_notification_without_panicking(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);

    cx.update_window(*window_handle, |_, window, cx| {
        export_commands::export_annotations(view.downgrade(), window, cx);
    })
    .unwrap();
    cx.run_until_parked();

    view.update(cx, |notatus, _| {
        assert_eq!(notatus.left_dock, LeftDock::Labels);
        assert_eq!(
            notatus.status_message.as_deref(),
            Some("Create a label before exporting")
        );
    });

    let notification_count = cx
        .update_window(*window_handle, |_, window, cx| {
            window.notifications(cx).len()
        })
        .unwrap();
    assert_eq!(notification_count, 1);
}

fn seed_three_media(notatus: &mut NotatusWindow) -> (LabelId, Vec<AssetId>) {
    let person = notatus.state.add_label("Person");
    let car = notatus.state.add_label("Car");
    let assets = ["/tmp/a.png", "/tmp/b.png", "/tmp/c.png"]
        .into_iter()
        .map(|path| notatus.state.add_local_image_asset(path, 100, 100).unwrap())
        .collect::<Vec<_>>();
    notatus.state.select_asset(assets[0]).unwrap();
    notatus.state.select_label(person).unwrap();
    (car, assets)
}

#[gpui::test]
fn keyboard_shortcuts_drive_the_annotation_loop(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);
    let window: gpui::AnyWindowHandle = window_handle.into();
    let (car, assets) = view.update(cx, |notatus, _| seed_three_media(notatus));
    cx.run_until_parked();

    cx.simulate_keystrokes(window, "d");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.selected_asset(), Some(assets[1]));
    });

    cx.simulate_keystrokes(window, "2");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.selected_label(), Some(car));
    });

    cx.simulate_keystrokes(window, "v");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.active_tool(), AnnotationTool::Select);
    });

    cx.simulate_keystrokes(window, "ctrl-enter");
    view.update(cx, |notatus, _| {
        let asset = notatus.state.dataset().asset_by_id(assets[1]).unwrap();
        assert_eq!(asset.status, notatus_core::AssetStatus::Done);
        assert_eq!(notatus.state.selected_asset(), Some(assets[2]));
        assert_eq!(
            notatus.progress_summary().as_deref(),
            Some("3 of 3 · 1 done")
        );
    });

    cx.simulate_keystrokes(window, "ctrl-z");
    view.update(cx, |notatus, _| {
        let asset = notatus.state.dataset().asset_by_id(assets[1]).unwrap();
        assert_eq!(asset.status, notatus_core::AssetStatus::Pending);
    });
}

#[gpui::test]
fn delete_key_removes_selected_annotation_and_undo_restores_it(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);
    let window: gpui::AnyWindowHandle = window_handle.into();
    let annotation_id = view.update(cx, |notatus, _| {
        let (label, assets) = seed_three_media(notatus);
        let bbox = BoundingBox::from_xywh(10.0, 10.0, 20.0, 20.0).unwrap();
        let id = notatus
            .state
            .add_human_bbox(assets[0], label, bbox, None)
            .unwrap();
        notatus.state.select_annotation(Some(id)).unwrap();
        id
    });
    cx.run_until_parked();

    cx.simulate_keystrokes(window, "delete");
    view.update(cx, |notatus, _| {
        assert!(notatus.state.dataset().annotations.is_empty());
    });

    cx.simulate_keystrokes(window, "ctrl-z");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.selected_annotation(), Some(annotation_id));
        assert_eq!(notatus.state.dataset().annotations.len(), 1);
    });

    cx.simulate_keystrokes(window, "ctrl-shift-z");
    view.update(cx, |notatus, _| {
        assert!(notatus.state.dataset().annotations.is_empty());
    });
}

#[gpui::test]
fn bare_key_shortcuts_are_ignored_while_typing(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);
    let window: gpui::AnyWindowHandle = window_handle.into();
    let assets = view.update(cx, |notatus, _| seed_three_media(notatus).1);
    cx.run_until_parked();

    cx.update_window(window, |_, window, cx| {
        let input = view.read(cx).project_name_input.clone();
        input.update(cx, |input, cx| input.focus(window, cx));
    })
    .unwrap();
    cx.simulate_keystrokes(window, "d");

    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.selected_asset(), Some(assets[0]));
    });
}

#[gpui::test]
fn command_palette_runs_the_confirmed_command(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);
    let window: gpui::AnyWindowHandle = window_handle.into();
    view.update(cx, |notatus, _| {
        seed_three_media(notatus);
    });
    cx.run_until_parked();

    cx.simulate_keystrokes(window, "ctrl-k");
    let open = cx
        .update_window(window, |_, window, cx| window.has_active_dialog(cx))
        .unwrap();
    assert!(open);

    cx.simulate_input(window, "polygon tool");
    cx.simulate_keystrokes(window, "enter");
    cx.run_until_parked();

    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.active_tool(), AnnotationTool::DrawPolygon);
    });
    let open = cx
        .update_window(window, |_, window, cx| window.has_active_dialog(cx))
        .unwrap();
    assert!(!open);

    // Focus returns to the window, so bare-key shortcuts work again.
    cx.simulate_keystrokes(window, "v");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.active_tool(), AnnotationTool::Select);
    });
}

#[gpui::test]
fn autosave_writes_local_projects_after_a_pause(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (_, view) = open_test_window(cx);

    view.update(cx, |notatus, cx| {
        notatus.project_location = ProjectLocation::local(dir.path());
        notatus.state.add_label("Person");
        cx.notify();
    });
    cx.run_until_parked();
    view.update(cx, |notatus, _| {
        assert!(notatus.state.is_dirty());
        assert_eq!(notatus.save_status(), "Edited");
    });
    assert!(!dir.path().join(MANIFEST_FILE).exists());

    cx.executor().advance_clock(autosave::AUTOSAVE_DELAY);
    cx.run_until_parked();

    view.update(cx, |notatus, _| {
        assert!(!notatus.state.is_dirty());
        assert_eq!(notatus.save_status(), "Saved");
    });
    let saved = LocalProjectStore::new(dir.path()).load_dataset().unwrap();
    assert_eq!(saved.labels.len(), 1);
}

#[gpui::test]
fn autosave_skips_projects_without_a_location(cx: &mut TestAppContext) {
    let (_, view) = open_test_window(cx);
    view.update(cx, |notatus, cx| {
        notatus.state.add_label("Person");
        cx.notify();
    });
    cx.executor().advance_clock(autosave::AUTOSAVE_DELAY);
    cx.run_until_parked();
    view.update(cx, |notatus, _| {
        assert!(notatus.state.is_dirty());
        assert_eq!(notatus.save_status(), "Not saved to disk");
    });
}

#[test]
fn drawing_without_labels_creates_one_and_undoes_together() {
    let mut state = UiState::new_project("demo");
    let asset_id = state.add_local_image_asset("/tmp/a.png", 100, 100).unwrap();
    let bbox = BoundingBox::from_xywh(10.0, 10.0, 20.0, 20.0).unwrap();

    let created = state.batch("Draw box", |state| {
        let (label_id, created) = commands::drawing_label(state);
        state
            .add_human_bbox(asset_id, label_id, bbox, None)
            .unwrap();
        created
    });

    assert!(created);
    assert_eq!(state.dataset().labels.len(), 1);
    assert_eq!(
        state.dataset().labels[0].name,
        commands::FALLBACK_LABEL_NAME
    );
    assert!(state.dataset().labels[0].color.is_some());
    assert_eq!(state.undo(), Some("Draw box"));
    assert!(state.dataset().labels.is_empty());
    assert!(state.dataset().annotations.is_empty());
}

#[test]
fn drawing_reuses_the_first_label_when_none_is_selected() {
    let mut state = UiState::new_project("demo");
    let first = state.add_label("Person");
    state.add_label("Car");
    state.clear_selected_label();

    assert_eq!(commands::drawing_label(&mut state), (first, false));
    assert_eq!(state.selected_label(), Some(first));
}

#[gpui::test]
fn label_picker_creates_a_typed_label_and_relabels_the_selection(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);
    let window: gpui::AnyWindowHandle = window_handle.into();
    let annotation_id = view.update(cx, |notatus, _| {
        let (_, assets) = seed_three_media(notatus);
        let label_id = notatus.state.dataset().labels[0].id;
        let bbox = BoundingBox::from_xywh(1.0, 1.0, 5.0, 5.0).unwrap();
        let id = notatus
            .state
            .add_human_bbox(assets[0], label_id, bbox, None)
            .unwrap();
        notatus.state.select_annotation(Some(id)).unwrap();
        id
    });
    cx.run_until_parked();

    cx.simulate_keystrokes(window, "l");
    cx.run_until_parked();
    cx.simulate_input(window, "Bicycle");
    cx.run_until_parked();
    cx.simulate_keystrokes(window, "enter");
    cx.run_until_parked();

    view.update(cx, |notatus, _| {
        let label = notatus.state.dataset().labels.last().unwrap();
        assert_eq!(label.name, "Bicycle");
        assert!(label.color.is_some());
        assert_eq!(notatus.state.selected_label(), Some(label.id));
        let annotation = notatus
            .state
            .dataset()
            .annotations
            .iter()
            .find(|annotation| annotation.id == annotation_id)
            .unwrap();
        assert_eq!(annotation.label_id, label.id);
    });
}

/// Drives the canvas with real mouse events in image coordinates.
struct CanvasDriver {
    cx: gpui::VisualTestContext,
    view: gpui::Entity<NotatusWindow>,
}

impl CanvasDriver {
    fn new(
        window: gpui::AnyWindowHandle,
        view: gpui::Entity<NotatusWindow>,
        cx: &TestAppContext,
    ) -> Self {
        let cx = gpui::VisualTestContext::from_window(window, cx);
        cx.run_until_parked();
        Self { cx, view }
    }

    fn screen(&mut self, (x, y): (f64, f64)) -> gpui::Point<gpui::Pixels> {
        self.view.update(&mut self.cx, |notatus, _| {
            let layout = notatus
                .canvas_image_layout
                .borrow()
                .expect("canvas laid out");
            let asset = notatus.selected_asset().expect("selected media");
            let bounds = layout.image_bounds;
            let sx = f32::from(bounds.size.width) / asset.dimensions.width as f32;
            let sy = f32::from(bounds.size.height) / asset.dimensions.height as f32;
            gpui::point(
                bounds.origin.x + px(x as f32 * sx),
                bounds.origin.y + px(y as f32 * sy),
            )
        })
    }

    fn drag(&mut self, from: (f64, f64), to: (f64, f64), modifiers: gpui::Modifiers) {
        let start = self.screen(from);
        let end = self.screen(to);
        let left = gpui::MouseButton::Left;
        self.cx.simulate_mouse_move(start, None, modifiers);
        self.cx.simulate_mouse_down(start, left, modifiers);
        self.cx.simulate_mouse_move(end, left, modifiers);
        self.cx.simulate_mouse_up(end, left, modifiers);
        self.cx.run_until_parked();
    }
}

#[gpui::test]
fn box_tool_draws_on_empty_space_and_moves_existing_boxes(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);
    view.update(cx, |notatus, cx| {
        seed_three_media(notatus);
        notatus.set_canvas_tool(AnnotationTool::DrawBox, cx);
    });
    let mut canvas = CanvasDriver::new(window_handle.into(), view.clone(), cx);
    let none = gpui::Modifiers::none();

    canvas.drag((10.0, 10.0), (40.0, 40.0), none);
    let drawn = view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.dataset().annotations.len(), 1);
        notatus.state.dataset().annotations[0].id
    });

    // Dragging the box body moves it instead of drawing a second box.
    canvas.drag((25.0, 25.0), (55.0, 35.0), none);
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.dataset().annotations.len(), 1);
        assert_eq!(notatus.state.selected_annotation(), Some(drawn));
        let bbox = notatus.state.dataset().annotations[0]
            .geometry
            .as_bbox()
            .unwrap();
        assert!((bbox.x - 40.0).abs() < 1.5, "moved x: {}", bbox.x);
        assert!((bbox.y - 20.0).abs() < 1.5, "moved y: {}", bbox.y);
    });

    // Alt forces drawing, even when starting inside a box.
    canvas.drag((50.0, 25.0), (60.0, 45.0), gpui::Modifiers::alt());
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.dataset().annotations.len(), 2);
    });

    // A move is one undo step.
    view.update(cx, |notatus, _| {
        notatus.state.undo();
        notatus.state.undo();
        let bbox = notatus.state.dataset().annotations[0]
            .geometry
            .as_bbox()
            .unwrap();
        assert!((bbox.x - 10.0).abs() < 1.5);
    });
}

#[gpui::test]
fn view_shortcuts_toggle_display_options(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);
    let window: gpui::AnyWindowHandle = window_handle.into();
    let revision = view.update(cx, |notatus, _| {
        seed_three_media(notatus);
        notatus.state.revision()
    });
    cx.run_until_parked();

    cx.simulate_keystrokes(window, "h shift-l");
    view.update(cx, |notatus, _| {
        assert!(notatus.view_options.hide_annotations);
        assert!(!notatus.view_options.show_labels);
        // Display options never touch the dataset.
        assert_eq!(notatus.state.revision(), revision);
    });
    cx.simulate_keystrokes(window, "h");
    view.update(cx, |notatus, _| {
        assert!(!notatus.view_options.hide_annotations);
    });
}

#[gpui::test]
fn review_keys_accept_and_reject_then_advance(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);
    let window: gpui::AnyWindowHandle = window_handle.into();
    let ids = view.update(cx, |notatus, _| {
        let (label, assets) = seed_three_media(notatus);
        let ids: Vec<_> = (0..3)
            .map(|i| {
                let bbox = BoundingBox::from_xywh(i as f64 * 20.0, 0.0, 10.0, 10.0).unwrap();
                notatus
                    .state
                    .add_human_bbox(assets[0], label, bbox, None)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        notatus.state.select_annotation(None).unwrap();
        ids
    });
    cx.run_until_parked();

    // With nothing selected, Y starts at the first annotation.
    cx.simulate_keystrokes(window, "y x");
    view.update(cx, |notatus, _| {
        let states: Vec<_> = notatus
            .state
            .dataset()
            .annotations
            .iter()
            .map(|annotation| annotation.review_state)
            .collect();
        assert_eq!(
            states,
            [
                ReviewState::Accepted,
                ReviewState::Rejected,
                ReviewState::Draft
            ]
        );
        assert_eq!(notatus.state.selected_annotation(), Some(ids[2]));
    });

    cx.simulate_keystrokes(window, "ctrl-z");
    view.update(cx, |notatus, _| {
        assert_eq!(
            notatus.state.dataset().annotations[1].review_state,
            ReviewState::Draft
        );
    });
}

#[gpui::test]
fn the_review_queue_crosses_media_only_when_asked(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);
    let window: gpui::AnyWindowHandle = window_handle.into();
    let (assets, first, other_media) = view.update(cx, |notatus, _| {
        let (label, assets) = seed_three_media(notatus);
        let draw = |notatus: &mut NotatusWindow, asset, x: f64| {
            let bbox = BoundingBox::from_xywh(x, 0.0, 10.0, 10.0).unwrap();
            notatus
                .state
                .add_human_bbox(asset, label, bbox, None)
                .unwrap()
        };
        let first = draw(notatus, assets[0], 0.0);
        let other_media = draw(notatus, assets[2], 0.0);
        notatus.state.select_asset(assets[0]).unwrap();
        (assets, first, other_media)
    });
    cx.run_until_parked();

    // Y reviews the only draft here and stays, pointing at N.
    cx.simulate_keystrokes(window, "y");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.selected_asset(), Some(assets[0]));
        assert_eq!(notatus.state.selected_annotation(), Some(first));
        let message = notatus.status_message.clone().unwrap();
        assert!(
            message.contains("press N for the next one (1 left)"),
            "{message}"
        );
        assert_eq!(
            notatus.progress_summary().as_deref(),
            Some("1 of 3 · 0 done · 1 to review")
        );
    });

    // N moves to the draft on the third media.
    cx.simulate_keystrokes(window, "n");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.selected_asset(), Some(assets[2]));
        assert_eq!(notatus.state.selected_annotation(), Some(other_media));
        assert_eq!(notatus.status_message.as_deref(), Some("1 draft to review"));
    });

    // Rejecting the last draft empties the queue.
    cx.simulate_keystrokes(window, "x");
    view.update(cx, |notatus, _| {
        assert_eq!(
            notatus.status_message.as_deref(),
            Some("Rejected annotation. Nothing left to review")
        );
    });
}

#[gpui::test]
fn the_review_filter_limits_where_n_goes(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);
    let window: gpui::AnyWindowHandle = window_handle.into();
    let (assets, car_draft) = view.update(cx, |notatus, cx| {
        let (car, assets) = seed_three_media(notatus);
        let person = notatus.state.dataset().labels[0].id;
        let bbox = BoundingBox::from_xywh(0.0, 0.0, 10.0, 10.0).unwrap();
        notatus
            .state
            .add_human_bbox(assets[1], person, bbox, None)
            .unwrap();
        let car_draft = notatus
            .state
            .add_human_bbox(assets[2], car, bbox, None)
            .unwrap();
        notatus.state.select_asset(assets[0]).unwrap();
        notatus.set_review_filter(
            ReviewFilter {
                label: Some(car),
                below_confidence: None,
            },
            cx,
        );
        assert_eq!(
            notatus.status_message.as_deref(),
            Some("1 draft to review matching the filter")
        );
        (assets, car_draft)
    });
    cx.run_until_parked();

    cx.simulate_keystrokes(window, "n");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.selected_asset(), Some(assets[2]));
        assert_eq!(notatus.state.selected_annotation(), Some(car_draft));
    });

    // Shift+N wraps around to the same draft, so there is nowhere else to go.
    cx.simulate_keystrokes(window, "shift-n");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.selected_annotation(), Some(car_draft));
        assert_eq!(
            notatus.status_message.as_deref(),
            Some("No other draft to review")
        );
    });
}
