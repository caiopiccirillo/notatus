use super::workflow::push_export_workflow_notification;
use super::*;
use crate::app::helpers::plural;

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
