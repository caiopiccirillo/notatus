use super::helpers::*;
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExportWorkflowIssue {
    Labels,
    Media,
    Annotations,
    Format,
}

impl ExportWorkflowIssue {
    fn target_dock(self) -> LeftDock {
        match self {
            Self::Labels => LeftDock::Dataset,
            Self::Media => LeftDock::Media,
            Self::Annotations | Self::Format => LeftDock::Dataset,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Labels => "Labels required",
            Self::Media => "Media required",
            Self::Annotations => "Annotations required",
            Self::Format => "Format required",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Labels => "Create a label before exporting annotations.",
            Self::Media => "Import media before exporting annotations.",
            Self::Annotations => "Create exportable annotations before exporting.",
            Self::Format => "Select at least one export format.",
        }
    }

    fn status(self) -> &'static str {
        match self {
            Self::Labels => "Create a label before exporting",
            Self::Media => "Import media before exporting",
            Self::Annotations => "No exportable annotations",
            Self::Format => "Select an export format",
        }
    }
}

impl NotatusWindow {
    pub(super) fn export_workflow_issue(&self) -> Option<ExportWorkflowIssue> {
        if self.state.dataset.labels.is_empty() {
            Some(ExportWorkflowIssue::Labels)
        } else if self.state.dataset.assets.is_empty() {
            Some(ExportWorkflowIssue::Media)
        } else if exportable_annotation_count(&self.state.dataset) == 0 {
            Some(ExportWorkflowIssue::Annotations)
        } else if !self.export_yolo && !self.export_coco {
            Some(ExportWorkflowIssue::Format)
        } else {
            None
        }
    }

    pub(super) fn apply_export_workflow_issue(
        &mut self,
        issue: ExportWorkflowIssue,
        cx: &mut Context<Self>,
    ) {
        self.left_dock = issue.target_dock();
        self.status_message = Some(issue.status().to_string());
        cx.notify();
    }

    pub(super) fn toggle_export_yolo(&mut self, cx: &mut Context<Self>) {
        self.export_yolo = !self.export_yolo;
        if !self.export_yolo && !self.export_coco {
            self.export_yolo = true;
            self.status_message = Some(ExportWorkflowIssue::Format.status().to_string());
        } else {
            self.status_message = None;
        }
        cx.notify();
    }

    pub(super) fn toggle_export_coco(&mut self, cx: &mut Context<Self>) {
        self.export_coco = !self.export_coco;
        if !self.export_yolo && !self.export_coco {
            self.export_coco = true;
            self.status_message = Some(ExportWorkflowIssue::Format.status().to_string());
        } else {
            self.status_message = None;
        }
        cx.notify();
    }
}

pub(super) fn open_export_dialog(
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

pub(super) fn push_export_workflow_notification(
    issue: ExportWorkflowIssue,
    window: &mut Window,
    cx: &mut App,
) {
    window.push_notification(
        Notification::warning(issue.message())
            .id1::<ExportWorkflowNotification>("workflow")
            .title(issue.title())
            .autohide(false),
        cx,
    );
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

pub(super) fn export_annotations(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    let issue = view
        .update(cx, |notatus, cx| {
            let issue = notatus.export_workflow_issue();
            if let Some(issue) = issue {
                notatus.apply_export_workflow_issue(issue, cx);
            } else {
                notatus.left_dock = LeftDock::Dataset;
                notatus.status_message = Some("Choose an export folder".to_string());
                cx.notify();
            }
            issue
        })
        .unwrap_or(None);

    if let Some(issue) = issue {
        push_export_workflow_notification(issue, window, cx);
        return;
    }

    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some(SharedString::from("Export annotations")),
    });

    window
        .spawn(cx, async move |window| match paths.await {
            Ok(Ok(Some(paths))) => {
                let Some(output_dir) = paths.into_iter().next() else {
                    return;
                };
                let export_request = view.update_in(window, |notatus, _, _| ExportRequest {
                    dataset: notatus.state.dataset.clone(),
                    yolo: notatus.export_yolo,
                    coco: notatus.export_coco,
                });
                let result = export_request
                    .map_err(|_| "window closed".to_string())
                    .and_then(|request| run_export(request, &output_dir));
                let _ = view.update_in(window, |notatus, window, cx| {
                    match result {
                        Ok(summary) => {
                            push_export_success_notification(summary.clone(), window, cx);
                            notatus.status_message = Some(summary);
                        }
                        Err(error) => {
                            let message = format!("Export failed: {error}");
                            push_export_error_notification(message.clone(), window, cx);
                            notatus.status_message = Some(message);
                        }
                    }
                    cx.notify();
                });
            }
            Ok(Ok(None)) => {
                let _ = view.update_in(window, |notatus, _, cx| {
                    notatus.status_message = Some("Export cancelled".to_string());
                    cx.notify();
                });
            }
            Ok(Err(error)) => {
                let _ = view.update_in(window, |notatus, window, cx| {
                    let message = format!("Export picker failed: {error}");
                    push_export_error_notification(message.clone(), window, cx);
                    notatus.status_message = Some(message);
                    cx.notify();
                });
            }
            Err(_) => {
                let _ = view.update_in(window, |notatus, window, cx| {
                    let message = "Export picker closed unexpectedly".to_string();
                    push_export_error_notification(message.clone(), window, cx);
                    notatus.status_message = Some(message);
                    cx.notify();
                });
            }
        })
        .detach();
}

struct ExportRequest {
    dataset: notatus_core::Dataset,
    yolo: bool,
    coco: bool,
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

struct ExportWorkflowNotification;
struct ExportResultNotification;

fn push_export_success_notification(
    message: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut App,
) {
    window.push_notification(
        Notification::success(message)
            .id1::<ExportResultNotification>("success")
            .title("Export complete"),
        cx,
    );
}

fn push_export_error_notification(
    message: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut App,
) {
    window.push_notification(
        Notification::error(message)
            .id1::<ExportResultNotification>("error")
            .title("Export failed")
            .autohide(false),
        cx,
    );
}

fn run_export(request: ExportRequest, output_dir: &Path) -> Result<String, String> {
    let filter = notatus_export::AnnotationFilter::all_non_rejected();
    let mut formats = Vec::new();
    let mut annotation_count = 0;

    if request.yolo {
        let summary = notatus_export::yolo::write_detection_export(
            &request.dataset,
            &filter,
            output_dir.join("yolo"),
        )
        .map_err(|error| error.to_string())?;
        formats.push("YOLO");
        annotation_count = annotation_count.max(summary.annotation_count);
    }

    if request.coco {
        let summary = notatus_export::coco::write_detection_export(
            &request.dataset,
            &filter,
            output_dir.join("coco"),
        )
        .map_err(|error| error.to_string())?;
        formats.push("COCO");
        annotation_count = annotation_count.max(summary.annotation_count);
    }

    Ok(format!(
        "Exported {} annotation{} as {}",
        annotation_count,
        plural(annotation_count),
        formats.join(" and ")
    ))
}
