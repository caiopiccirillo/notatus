//! Saving projects through the window.

use super::*;
use gpui::{TestAppContext, WindowHandle, WindowOptions};

fn open_test_window(cx: &mut TestAppContext) -> (WindowHandle<Root>, gpui::Entity<NotatusWindow>) {
    let view_slot = Rc::new(RefCell::new(None));
    let slot = view_slot.clone();
    let window = cx.update(|cx| {
        gpui_component::init(cx);
        actions::bind_keys(cx);
        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| NotatusWindow::new(window, cx));
            *slot.borrow_mut() = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        })
        .unwrap()
    });
    cx.run_until_parked();
    let view = view_slot.borrow_mut().take().expect("window view");
    (window, view)
}

fn saved_label_count(dir: &Path) -> usize {
    LocalProjectStore::new(dir)
        .load_dataset()
        .unwrap()
        .labels
        .len()
}

#[gpui::test]
fn save_requested_during_a_save_writes_the_newest_state(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (_, view) = open_test_window(cx);

    view.update(cx, |notatus, cx| {
        notatus.project_location = ProjectLocation::local(dir.path());
        notatus.state.add_label("Person");
        notatus.save_now(cx);
        // The first save is still running when the second edit lands.
        notatus.state.add_label("Car");
        notatus.save_now(cx);
    });
    cx.run_until_parked();

    view.update(cx, |notatus, _| {
        assert!(!notatus.state.dirty);
        assert_eq!(notatus.save_status(), "Saved");
        assert_eq!(notatus.status_message.as_deref(), Some("Saved project"));
    });
    assert_eq!(saved_label_count(dir.path()), 2);
}

#[gpui::test]
fn late_save_for_a_replaced_project_is_ignored(cx: &mut TestAppContext) {
    let first = tempfile::tempdir().unwrap();
    let (_, view) = open_test_window(cx);
    view.update(cx, |notatus, cx| {
        notatus.project_location = ProjectLocation::local(first.path());
        notatus.state.add_label("Person");
        notatus.save_now(cx);
        // Another project replaces this one before the save lands.
        notatus.autosave.reset();
        notatus.state = UiState::new_project("other");
        notatus.project_location = ProjectLocation::Unsaved;
    });
    cx.run_until_parked();

    view.update(cx, |notatus, _| {
        assert!(!notatus.state.dirty);
        assert_eq!(notatus.save_status(), "Not saved to disk");
        assert_ne!(notatus.status_message.as_deref(), Some("Saved project"));
    });
    assert_eq!(saved_label_count(first.path()), 1);
}
