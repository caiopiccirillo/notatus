use super::*;
use gpui::{TestAppContext, WindowHandle, WindowOptions};

fn open_test_window(cx: &mut TestAppContext) -> (WindowHandle<Root>, gpui::Entity<NotatusWindow>) {
    let notatus_view = Rc::new(RefCell::new(None));
    let view_slot = notatus_view.clone();

    let window = cx.update(|cx| {
        gpui_component::init(cx);
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
