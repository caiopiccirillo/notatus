//! Failures that cost or block work, reported where they cannot be missed.
//!
//! The status bar's message is replaced by the next command's, so a failed
//! save or model run is also shown as a notification that stays until it is
//! dismissed.

use super::*;
use gpui_component::notification::Notification;

/// Groups failure notifications, so a repeated failure replaces the last
/// rather than stacking.
struct FailureNotification;

impl NotatusWindow {
    /// Shows `message` in the status bar and in a notification titled
    /// `title` that stays until dismissed.
    pub(super) fn report_failure(
        &mut self,
        title: &'static str,
        message: String,
        cx: &mut Context<Self>,
    ) {
        self.status_message = Some(format!("{title}: {message}"));
        let handle = self.window_handle;
        // Notifications live in the window's root; reach it after this
        // update, as the window may be mid-update now.
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                window.push_notification(
                    Notification::error(message)
                        .id::<FailureNotification>()
                        .title(title)
                        .autohide(false),
                    cx,
                );
            });
        });
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use crate::app::tests::support::open_test_window;
    use gpui::{AppContext as _, TestAppContext};
    use gpui_component::WindowExt as _;

    #[gpui::test]
    fn a_failure_stays_in_the_status_bar_and_raises_a_notification(cx: &mut TestAppContext) {
        let (window, view) = open_test_window(cx);

        view.update(cx, |notatus, cx| {
            notatus.report_failure("Save failed", "disk full".to_string(), cx);
        });
        cx.run_until_parked();

        view.read_with(cx, |notatus, _| {
            assert_eq!(
                notatus.status_message.as_deref(),
                Some("Save failed: disk full")
            );
        });
        let notifications = cx
            .update_window(window.into(), |_, window, cx| {
                window.notifications(cx).len()
            })
            .unwrap();
        assert_eq!(notifications, 1);
    }
}
