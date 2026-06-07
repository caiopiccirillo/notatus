use super::runner::export_annotations;
use super::*;
use crate::app::helpers::*;

pub(in crate::app) fn open_export_dialog(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(snapshot) = export_dialog_snapshot(view.clone(), cx) else {
        return;
    };
    let snapshot = Rc::new(RefCell::new(snapshot));

    window.open_dialog(cx, move |dialog, _, _| {
        let content_snapshot = snapshot.clone();
        let export_view = view.clone();

        dialog
            .confirm()
            .w(px(420.0))
            .title("Export annotations")
            .child(export_dialog_content(
                view.clone(),
                content_snapshot.clone(),
            ))
            .button_props(
                DialogButtonProps::default()
                    .ok_text("Export")
                    .cancel_text("Cancel"),
            )
            .on_ok(move |_, window, cx| {
                window.close_dialog(cx);
                export_annotations(export_view.clone(), window, cx);
                false
            })
    });
}

fn export_dialog_snapshot(
    view: gpui::WeakEntity<NotatusWindow>,
    cx: &mut App,
) -> Option<ExportDialogSnapshot> {
    view.update(cx, |notatus, _| ExportDialogSnapshot {
        media_count: notatus.state.dataset.assets.len(),
        label_count: notatus.state.dataset.labels.len(),
        annotation_count: notatus.state.dataset.annotations.len(),
        exportable_count: exportable_annotation_count(&notatus.state.dataset),
        yolo: notatus.export_yolo,
        coco: notatus.export_coco,
    })
    .ok()
}

fn export_dialog_content(
    view: gpui::WeakEntity<NotatusWindow>,
    snapshot: Rc<RefCell<ExportDialogSnapshot>>,
) -> gpui::AnyElement {
    let snapshot_read = snapshot.borrow();
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(section_title("Formats"))
                .child(export_dialog_format_buttons(
                    view.clone(),
                    snapshot.clone(),
                    snapshot_read.yolo,
                    snapshot_read.coco,
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(section_title("Summary"))
                .child(metric(
                    "Media",
                    media_count_label(snapshot_read.media_count),
                ))
                .child(metric(
                    "Labels",
                    label_count_label(snapshot_read.label_count),
                ))
                .child(metric(
                    "Annotations",
                    annotation_count_label(snapshot_read.annotation_count),
                ))
                .child(metric(
                    "Exportable",
                    annotation_count_label(snapshot_read.exportable_count),
                ))
                .child(metric("Filter", "All non-rejected".to_string()))
                .child(metric("Output", snapshot_read.output_label())),
        )
        .into_any_element()
}

fn export_dialog_format_buttons(
    view: gpui::WeakEntity<NotatusWindow>,
    snapshot: Rc<RefCell<ExportDialogSnapshot>>,
    yolo: bool,
    coco: bool,
) -> impl IntoElement {
    let yolo_view = view.clone();
    let yolo_snapshot = snapshot.clone();
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            Button::new("export-dialog-format-yolo")
                .small()
                .label("YOLO")
                .selected(yolo)
                .on_click(move |_, _, cx| {
                    {
                        let mut snapshot = yolo_snapshot.borrow_mut();
                        snapshot.yolo = !snapshot.yolo;
                        if !snapshot.yolo && !snapshot.coco {
                            snapshot.yolo = true;
                        }
                    }
                    let _ = yolo_view.update(cx, |notatus, cx| {
                        notatus.toggle_export_yolo(cx);
                    });
                }),
        )
        .child(
            Button::new("export-dialog-format-coco")
                .small()
                .label("COCO")
                .selected(coco)
                .on_click(move |_, _, cx| {
                    {
                        let mut snapshot = snapshot.borrow_mut();
                        snapshot.coco = !snapshot.coco;
                        if !snapshot.yolo && !snapshot.coco {
                            snapshot.coco = true;
                        }
                    }
                    let _ = view.update(cx, |notatus, cx| {
                        notatus.toggle_export_coco(cx);
                    });
                }),
        )
}

struct ExportDialogSnapshot {
    media_count: usize,
    label_count: usize,
    annotation_count: usize,
    exportable_count: usize,
    yolo: bool,
    coco: bool,
}

impl ExportDialogSnapshot {
    fn output_label(&self) -> String {
        match (self.yolo, self.coco) {
            (true, true) => "YOLO and COCO".to_string(),
            (true, false) => "YOLO".to_string(),
            (false, true) => "COCO".to_string(),
            (false, false) => "None".to_string(),
        }
    }
}
