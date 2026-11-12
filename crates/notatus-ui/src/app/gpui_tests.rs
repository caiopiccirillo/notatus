use super::*;
use gpui::{TestAppContext, WindowHandle, WindowOptions};

fn open_test_window(cx: &mut TestAppContext) -> (WindowHandle<Root>, gpui::Entity<NotatusWindow>) {
    let notatus_view = Rc::new(RefCell::new(None));
    let view_slot = notatus_view.clone();

    let window = cx.update(|cx| {
        gpui_component::init(cx);
        actions::bind_keys(cx);
        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| NotatusWindow::new(window, cx));
            *view_slot.borrow_mut() = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        })
        .unwrap()
    });

    cx.run_until_parked();
    let view = notatus_view
        .borrow_mut()
        .take()
        .expect("test window should capture NotatusWindow view");
    (window, view)
}

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
        assert_eq!(notatus.export_workflow_issue(), None);

        notatus.export_yolo = false;
        notatus.export_coco = false;
        notatus.export_classifications = false;
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

        notatus.export_yolo = false;
        notatus.export_coco = false;
        notatus.export_classifications = true;

        assert_eq!(notatus.export_workflow_issue(), None);
    });
}

#[gpui::test]
fn export_format_toggles_keep_at_least_one_format(cx: &mut TestAppContext) {
    let (_, view) = open_test_window(cx);

    view.update(cx, |notatus, cx| {
        assert!(notatus.export_yolo);
        assert!(notatus.export_coco);
        assert!(notatus.export_classifications);

        notatus.toggle_export_yolo(cx);
        assert!(!notatus.export_yolo);
        assert!(notatus.export_coco);
        assert!(notatus.export_classifications);

        notatus.toggle_export_coco(cx);
        assert!(!notatus.export_yolo);
        assert!(!notatus.export_coco);
        assert!(notatus.export_classifications);
        assert_eq!(notatus.status_message, None);

        notatus.toggle_export_classifications(cx);
        assert!(!notatus.export_yolo);
        assert!(!notatus.export_coco);
        assert!(notatus.export_classifications);
        assert_eq!(
            notatus.status_message.as_deref(),
            Some("Select an export format")
        );

        notatus.toggle_export_yolo(cx);
        assert!(notatus.export_yolo);
        assert!(!notatus.export_coco);
        assert!(notatus.export_classifications);
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
        assert_eq!(notatus.left_dock, LeftDock::Dataset);
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
        assert_eq!(notatus.state.selected_asset, Some(assets[1]));
    });

    cx.simulate_keystrokes(window, "2");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.selected_label, Some(car));
    });

    cx.simulate_keystrokes(window, "v");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.active_tool, AnnotationTool::Select);
    });

    cx.simulate_keystrokes(window, "ctrl-enter");
    view.update(cx, |notatus, _| {
        let asset = notatus.state.dataset.asset_by_id(assets[1]).unwrap();
        assert_eq!(asset.status, notatus_core::AssetStatus::Done);
        assert_eq!(notatus.state.selected_asset, Some(assets[2]));
        assert_eq!(
            notatus.progress_summary().as_deref(),
            Some("3 of 3 · 1 done")
        );
    });

    cx.simulate_keystrokes(window, "ctrl-z");
    view.update(cx, |notatus, _| {
        let asset = notatus.state.dataset.asset_by_id(assets[1]).unwrap();
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
        assert!(notatus.state.dataset.annotations.is_empty());
    });

    cx.simulate_keystrokes(window, "ctrl-z");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.selected_annotation, Some(annotation_id));
        assert_eq!(notatus.state.dataset.annotations.len(), 1);
    });

    cx.simulate_keystrokes(window, "ctrl-shift-z");
    view.update(cx, |notatus, _| {
        assert!(notatus.state.dataset.annotations.is_empty());
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
        assert_eq!(notatus.state.selected_asset, Some(assets[0]));
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
        assert_eq!(notatus.state.active_tool, AnnotationTool::DrawPolygon);
    });
    let open = cx
        .update_window(window, |_, window, cx| window.has_active_dialog(cx))
        .unwrap();
    assert!(!open);

    // Focus returns to the window, so bare-key shortcuts work again.
    cx.simulate_keystrokes(window, "v");
    view.update(cx, |notatus, _| {
        assert_eq!(notatus.state.active_tool, AnnotationTool::Select);
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
        assert!(notatus.state.dirty);
        assert_eq!(notatus.save_status(), "Edited");
    });
    assert!(!dir.path().join(MANIFEST_FILE).exists());

    cx.executor().advance_clock(autosave::AUTOSAVE_DELAY);
    cx.run_until_parked();

    view.update(cx, |notatus, _| {
        assert!(!notatus.state.dirty);
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
        assert!(notatus.state.dirty);
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
    assert_eq!(state.dataset.labels.len(), 1);
    assert_eq!(state.dataset.labels[0].name, commands::FALLBACK_LABEL_NAME);
    assert!(state.dataset.labels[0].color.is_some());
    assert_eq!(state.undo(), Some("Draw box"));
    assert!(state.dataset.labels.is_empty());
    assert!(state.dataset.annotations.is_empty());
}

#[test]
fn drawing_reuses_the_first_label_when_none_is_selected() {
    let mut state = UiState::new_project("demo");
    let first = state.add_label("Person");
    state.add_label("Car");
    state.selected_label = None;

    assert_eq!(commands::drawing_label(&mut state), (first, false));
    assert_eq!(state.selected_label, Some(first));
}

#[gpui::test]
fn label_picker_creates_a_typed_label_and_relabels_the_selection(cx: &mut TestAppContext) {
    let (window_handle, view) = open_test_window(cx);
    let window: gpui::AnyWindowHandle = window_handle.into();
    let annotation_id = view.update(cx, |notatus, _| {
        let (_, assets) = seed_three_media(notatus);
        let label_id = notatus.state.dataset.labels[0].id;
        let bbox = BoundingBox::from_xywh(1.0, 1.0, 5.0, 5.0).unwrap();
        let id = notatus
            .state
            .add_human_bbox(assets[0], label_id, bbox, None)
            .unwrap();
        notatus.state.selected_annotation = Some(id);
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
        let label = notatus.state.dataset.labels.last().unwrap();
        assert_eq!(label.name, "Bicycle");
        assert!(label.color.is_some());
        assert_eq!(notatus.state.selected_label, Some(label.id));
        let annotation = notatus
            .state
            .dataset
            .annotations
            .iter()
            .find(|annotation| annotation.id == annotation_id)
            .unwrap();
        assert_eq!(annotation.label_id, label.id);
    });
}
