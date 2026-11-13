//! The keyboard shortcut sheet, opened with `?`.
//!
//! Rows name actions and show whatever key is bound to them, so the sheet
//! cannot drift from the real bindings.

use super::actions::*;
use super::*;
use gpui_component::kbd::Kbd;

/// One titled group of shortcuts.
struct ShortcutGroup {
    title: &'static str,
    rows: Vec<(&'static str, Box<dyn gpui::Action>)>,
}

fn shortcut_groups() -> Vec<ShortcutGroup> {
    fn row(
        label: &'static str,
        action: impl gpui::Action,
    ) -> (&'static str, Box<dyn gpui::Action>) {
        (label, Box::new(action))
    }
    vec![
        ShortcutGroup {
            title: "Draw",
            rows: vec![
                row("Box tool", BoxTool),
                row("Polygon tool", PolygonTool),
                row("Select tool", SelectTool),
                row("Choose label", PickLabel),
                row("Cancel drawing", Cancel),
                row("Delete selection", DeleteSelection),
                row("Undo", Undo),
                row("Redo", Redo),
            ],
        },
        ShortcutGroup {
            title: "Media",
            rows: vec![
                row("Next media", NextMedia),
                row("Previous media", PreviousMedia),
                row("Next unfinished media", NextPendingMedia),
                row("Mark done", ToggleMediaDone),
                row("Done and next", DoneAndNext),
            ],
        },
        ShortcutGroup {
            title: "Review",
            rows: vec![
                row("Accept and go to next", AcceptAnnotation),
                row("Reject and go to next", RejectAnnotation),
                row("Reset to draft", ResetAnnotationReview),
                row("Next annotation", NextAnnotation),
                row("Previous annotation", PreviousAnnotation),
            ],
        },
        ShortcutGroup {
            title: "View",
            rows: vec![
                row("Fit image", FitCanvas),
                row("Zoom in", ZoomIn),
                row("Zoom out", ZoomOut),
                row("Hide annotations", ToggleHideAnnotations),
                row("Label names", ToggleLabelNames),
                row("Crosshair", ToggleCrosshair),
                row("Focus selected", ToggleFocusSelected),
            ],
        },
        ShortcutGroup {
            title: "Project",
            rows: vec![
                row("Command palette", CommandPalette),
                row("Save", SaveProject),
                row("Import images", ImportMedia),
                row("Import folder", ImportFolder),
                row("Export", ExportAnnotations),
            ],
        },
    ]
}

impl NotatusWindow {
    pub(super) fn open_shortcuts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        window.open_dialog(cx, |dialog, window, cx| {
            let muted = cx.theme().muted_foreground;
            let columns = shortcut_groups()
                .into_iter()
                .map(|group| shortcut_group(group, muted, window))
                .collect::<Vec<_>>();
            dialog
                .title("Keyboard shortcuts")
                .width(px(720.0))
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

fn shortcut_group(group: ShortcutGroup, muted: gpui::Hsla, window: &Window) -> impl IntoElement {
    div()
        .w(px(300.0))
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
        .children(group.rows.into_iter().map(|(label, action)| {
            let binding = Kbd::binding_for_action(action.as_ref(), Some(CONTEXT), window);
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .text_sm()
                .child(label)
                .when_some(binding, |row, binding| row.child(binding))
        }))
}

/// The label slot keys are nine separate bindings, so they get one line.
fn label_slots_hint(muted: gpui::Hsla) -> impl IntoElement {
    div()
        .w(px(300.0))
        .text_sm()
        .text_color(muted)
        .child("Keys 1 to 9 pick the first nine labels; ? shows this sheet.")
}

#[cfg(test)]
mod tests {
    use super::super::test_support::open_test_window;
    use super::*;
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
    fn every_listed_action_has_a_key(cx: &mut TestAppContext) {
        let (window, _) = open_test_window(cx);

        let unbound: Vec<_> = cx
            .update_window(window.into(), |_, window, _| {
                shortcut_groups()
                    .into_iter()
                    .flat_map(|group| group.rows)
                    // The same lookup the sheet renders with.
                    .filter(|(_, action)| {
                        Kbd::binding_for_action(action.as_ref(), Some(CONTEXT), window).is_none()
                    })
                    .map(|(label, _)| label)
                    .collect()
            })
            .unwrap();

        assert!(unbound.is_empty(), "listed without a key: {unbound:?}");
    }
}
