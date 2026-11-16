//! The export dialog: formats, which media, and what will be written.
//!
//! The dialog layer rebuilds its content on every render, so the content
//! reads the window's options and dataset each time rather than keeping a
//! copy that could drift.

use super::options::{ExportFormat, ExportOptions};
use super::runner::export_annotations;
use crate::app::helpers::{
    annotation_count_label, exportable_annotation_count, label_count_label, media_count_label,
    section_title, themed,
};
use crate::app::window::NotatusWindow;
use gpui::prelude::*;
use gpui::{App, IntoElement, SharedString, Window, div, px};
use gpui_component::checkbox::Checkbox;
use gpui_component::description_list::DescriptionList;
use gpui_component::dialog::DialogButtonProps;
use gpui_component::switch::Switch;
use gpui_component::{Sizable as _, WindowExt as _};

pub(in crate::app) fn open_export_dialog(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    window.open_alert_dialog(cx, move |dialog, _, cx| {
        let export_view = view.clone();
        let content = view
            .upgrade()
            .map(|notatus| {
                export_dialog_content(&ExportSummary::of(notatus.read(cx)), view.clone())
                    .into_any_element()
            })
            .unwrap_or_else(|| div().into_any_element());
        dialog
            .confirm()
            .width(px(440.0))
            .title("Export annotations")
            .child(content)
            .button_props(
                DialogButtonProps::default()
                    .ok_text("Choose folder and export")
                    .cancel_text("Cancel"),
            )
            .on_ok(move |_, window, cx| {
                window.close_dialog(cx);
                export_annotations(export_view.clone(), window, cx);
                false
            })
    });
}

/// What the dialog shows, read from the window.
struct ExportSummary {
    options: ExportOptions,
    media: usize,
    done: usize,
    exportable_objects: usize,
    image_labels: usize,
    labels: usize,
}

impl ExportSummary {
    fn of(notatus: &NotatusWindow) -> Self {
        let dataset = notatus.state.dataset();
        Self {
            options: notatus.export,
            media: dataset.assets.len(),
            done: notatus.state.index().done_media,
            exportable_objects: exportable_annotation_count(dataset),
            image_labels: dataset.classifications.len(),
            labels: dataset.labels.len(),
        }
    }

    fn media_label(&self) -> String {
        if self.options.finished_only {
            format!("{} done, of {}", self.done, media_count_label(self.media))
        } else {
            media_count_label(self.media)
        }
    }
}

fn export_dialog_content(
    summary: &ExportSummary,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    let formats = ExportFormat::ALL.into_iter().map(|format| {
        let view = view.clone();
        Checkbox::new(SharedString::from(format!("export-format-{format:?}")))
            .checked(summary.options.includes(format))
            .label(format.title())
            .child(themed(move |theme| {
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(format.description())
            }))
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |notatus, cx| notatus.toggle_export_format(format, cx));
            })
    });

    div()
        .flex()
        .flex_col()
        .gap_5()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(section_title("Formats"))
                .children(formats),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().text_sm().child("Only media marked done"))
                        .child(themed(|theme| {
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("Unfinished images would be exported as empty")
                        })),
                )
                .child(
                    Switch::new("export-finished-only")
                        .checked(summary.options.finished_only)
                        .on_click(move |_, _, cx| {
                            let _ = view
                                .update(cx, |notatus, cx| notatus.toggle_export_finished_only(cx));
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(section_title("Will export"))
                .child(
                    DescriptionList::new()
                        .columns(1)
                        .bordered(false)
                        .label_width(px(120.0))
                        .small()
                        .item("Media", summary.media_label(), 1)
                        .item(
                            "Objects",
                            annotation_count_label(summary.exportable_objects),
                            1,
                        )
                        .item("Image labels", summary.image_labels.to_string(), 1)
                        .item("Labels", label_count_label(summary.labels), 1)
                        .item("Formats", summary.options.formats_label(), 1),
                )
                .child(themed(|theme| {
                    div().text_xs().text_color(theme.muted_foreground).child(
                        "Rejected annotations and unreviewed model suggestions are left out.",
                    )
                })),
        )
}
