//! The keyboard shortcut sheet, opened with `?`.
//!
//! Rows name actions and show whatever key is bound to them, so the sheet
//! cannot drift from the real bindings.

use crate::app::actions::CONTEXT;
use crate::app::catalog::{Command, GROUPS, Group};
use crate::app::window::NotatusWindow;
use gpui::prelude::*;
use gpui::{Context, FontWeight, IntoElement, Window, div, px};
use gpui_component::kbd::Kbd;
use gpui_component::{ActiveTheme as _, WindowExt as _};

impl NotatusWindow {
    pub(super) fn open_shortcuts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        window.open_dialog(cx, |dialog, window, cx| {
            let muted = cx.theme().muted_foreground;
            let columns = GROUPS
                .iter()
                .filter_map(|group| shortcut_group(group, muted, window))
                .collect::<Vec<_>>();
            dialog
                .title("Keyboard shortcuts")
                .width(px(960.0))
                .overlay_closable(true)
                // A quick reference, so it appears at once, like the palette.
                .entrance(gpui_component::dialog::DialogEntrance::None)
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_x_8()
                        .gap_y_5()
                        .children(columns)
                        .child(label_slots_hint(muted)),
                )
        });
    }
}

/// A group's commands that have a key, or nothing when none does.
fn shortcut_group(group: &Group, muted: gpui::Hsla, window: &Window) -> Option<impl IntoElement> {
    let rows: Vec<_> = group
        .commands()
        .filter_map(|command: &Command| {
            let binding = Kbd::binding_for_action(command.action, Some(CONTEXT), window)?;
            Some(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_sm()
                    .child(command.title)
                    .child(binding),
            )
        })
        .collect();
    if rows.is_empty() {
        return None;
    }
    Some(
        div()
            .w(px(280.0))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child(group.title),
            )
            .children(rows),
    )
}

/// The label slot keys are nine separate bindings, so they get one line.
fn label_slots_hint(muted: gpui::Hsla) -> impl IntoElement {
    div()
        .w(px(280.0))
        .text_sm()
        .text_color(muted)
        .child("Keys 1 to 9 pick the first nine labels.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::actions::{
        AcceptAnnotation, BoxTool, DoneAndNext, NextDraft, NextMedia, RejectAnnotation,
        SaveProject, Undo,
    };
    use crate::app::tests::support::open_test_window;
    use gpui::TestAppContext;

    #[gpui::test]
    fn question_mark_opens_the_shortcut_sheet(cx: &mut TestAppContext) {
        let (window, _) = open_test_window(cx);

        cx.simulate_keystrokes(window.into(), "?");
        cx.run_until_parked();

        let open = cx
            .update_window(window.into(), |_, window, cx| window.has_active_dialog(cx))
            .unwrap();
        assert!(open);
    }

    #[gpui::test]
    fn the_annotation_loop_has_keys(cx: &mut TestAppContext) {
        let (window, _) = open_test_window(cx);
        // The sheet leaves out commands without a key; these must not be.
        let loop_actions: [&dyn gpui::Action; 8] = [
            &BoxTool,
            &AcceptAnnotation,
            &RejectAnnotation,
            &NextDraft,
            &NextMedia,
            &DoneAndNext,
            &Undo,
            &SaveProject,
        ];
        let unbound: Vec<_> = cx
            .update_window(window.into(), |_, window, _| {
                loop_actions
                    .iter()
                    .filter(|action| {
                        Kbd::binding_for_action(**action, Some(CONTEXT), window).is_none()
                    })
                    .map(|action| action.name())
                    .collect()
            })
            .unwrap();
        assert!(unbound.is_empty(), "no key for {unbound:?}");
    }
}
