//! Saving, closing, and reopening projects through the window.

use super::test_support::open_test_window;
use super::*;
use gpui::{TestAppContext, VisualTestContext, WindowHandle};

fn saved_label_count(dir: &Path) -> usize {
    LocalProjectStore::new(dir)
        .load_dataset()
        .unwrap()
        .labels
        .len()
}

fn has_dialog(window: WindowHandle<Root>, cx: &mut TestAppContext) -> bool {
    // The untyped handle leaves the root view free for the dialog check.
    cx.update_window(window.into(), |_, window, cx| window.has_active_dialog(cx))
        .unwrap()
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
fn closing_saves_pending_changes_before_the_window_goes(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (window, view) = open_test_window(cx);
    view.update(cx, |notatus, cx| {
        notatus.project_location = ProjectLocation::local(dir.path());
        notatus.state.add_label("Person");
        cx.notify();
    });
    cx.run_until_parked();
    assert!(!dir.path().join(MANIFEST_FILE).exists());

    let closed = VisualTestContext::from_window(window.into(), cx).simulate_close();

    assert!(!closed, "the window waits for the save");
    cx.run_until_parked();
    assert_eq!(saved_label_count(dir.path()), 1);
    assert!(cx.windows().is_empty(), "the window closes once saved");
}

#[gpui::test]
fn closing_without_changes_is_immediate(cx: &mut TestAppContext) {
    let (window, _) = open_test_window(cx);

    assert!(VisualTestContext::from_window(window.into(), cx).simulate_close());
}

#[gpui::test]
fn closing_a_never_saved_project_asks_first(cx: &mut TestAppContext) {
    let (window, view) = open_test_window(cx);
    view.update(cx, |notatus, cx| {
        notatus.state.add_label("Person");
        cx.notify();
    });
    cx.run_until_parked();

    let closed = VisualTestContext::from_window(window.into(), cx).simulate_close();
    cx.run_until_parked();

    assert!(!closed);
    assert!(has_dialog(window, cx));
    assert_eq!(cx.windows().len(), 1);
}

#[gpui::test]
fn failed_save_on_close_keeps_the_window_open(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    // A file where the project folder should be makes every save fail.
    let blocked = dir.path().join("not-a-folder");
    std::fs::write(&blocked, "").unwrap();
    let (window, view) = open_test_window(cx);
    view.update(cx, |notatus, cx| {
        notatus.project_location = ProjectLocation::local(blocked.join("project"));
        notatus.state.add_label("Person");
        cx.notify();
    });

    let closed = VisualTestContext::from_window(window.into(), cx).simulate_close();
    cx.run_until_parked();

    assert!(!closed);
    assert_eq!(cx.windows().len(), 1);
    assert!(has_dialog(window, cx));
    view.update(cx, |notatus, _| {
        assert!(notatus.state.dirty);
        assert!(
            notatus
                .status_message
                .as_deref()
                .is_some_and(|message| message.starts_with("Autosave failed"))
        );
    });
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

#[gpui::test]
fn ctrl_s_saves_without_panicking(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (window, view) = open_test_window(cx);
    view.update(cx, |notatus, cx| {
        notatus.project_location = ProjectLocation::local(dir.path());
        notatus.state.add_label("Person");
        cx.notify();
    });
    cx.run_until_parked();

    cx.simulate_keystrokes(window.into(), "ctrl-s");
    cx.run_until_parked();

    assert_eq!(saved_label_count(dir.path()), 1);
    view.update(cx, |notatus, _| assert!(!notatus.state.dirty));
}

#[gpui::test]
fn project_shortcuts_run_without_panicking(cx: &mut TestAppContext) {
    // Each of these used to update the window from inside its own update.
    for keys in ["ctrl-n", "ctrl-o", "ctrl-shift-s", "ctrl-i", "ctrl-e"] {
        let (window, _) = open_test_window(cx);
        cx.simulate_keystrokes(window.into(), keys);
        cx.run_until_parked();
    }
}

/// Presses the title bar's close button. The test platform never draws
/// client-side decorations, so this calls the button's handler directly.
fn press_title_bar_close(
    window: WindowHandle<Root>,
    view: &gpui::Entity<NotatusWindow>,
    cx: &mut TestAppContext,
) {
    let view = view.downgrade();
    cx.update_window(window.into(), |_, window, cx| {
        super::titlebar::close_from_title_bar(&view, window, cx)
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn title_bar_close_button_saves_before_closing(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (window, view) = open_test_window(cx);
    view.update(cx, |notatus, cx| {
        notatus.project_location = ProjectLocation::local(dir.path());
        notatus.state.add_label("Person");
        cx.notify();
    });
    cx.run_until_parked();

    press_title_bar_close(window, &view, cx);

    assert_eq!(saved_label_count(dir.path()), 1);
    assert!(cx.windows().is_empty(), "closes once saved");
}

#[gpui::test]
fn title_bar_close_button_asks_before_discarding(cx: &mut TestAppContext) {
    let (window, view) = open_test_window(cx);
    view.update(cx, |notatus, cx| {
        notatus.state.add_label("Person");
        cx.notify();
    });
    cx.run_until_parked();

    press_title_bar_close(window, &view, cx);

    assert_eq!(cx.windows().len(), 1);
    assert!(has_dialog(window, cx));
}

#[gpui::test]
fn title_bar_close_button_closes_a_clean_window(cx: &mut TestAppContext) {
    let (window, view) = open_test_window(cx);

    press_title_bar_close(window, &view, cx);

    assert!(cx.windows().is_empty());
}
