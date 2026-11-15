use super::workflow::push_export_workflow_notification;
use super::*;
use crate::app::helpers::plural;

pub(in crate::app) fn export_annotations(
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
                    dataset: notatus.state.dataset().clone(),
                    yolo: notatus.export.yolo,
                    coco: notatus.export.coco,
                    classifications: notatus.export.classifications,
                    finished_only: notatus.export.finished_only,
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
    classifications: bool,
    finished_only: bool,
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

fn run_export(mut request: ExportRequest, output_dir: &Path) -> Result<String, String> {
    let mut left_out = 0;
    if request.finished_only {
        let selected = notatus_export::finished_media(&request.dataset);
        left_out = request.dataset.assets.len() - selected.assets.len();
        request.dataset = selected;
    }
    let filter = notatus_export::AnnotationFilter::ready_for_training();
    let mut formats = Vec::new();
    let mut annotation_count = 0;
    let mut classification_count = 0;

    let mut skipped_polygons = 0;
    if request.yolo {
        let summary = notatus_export::yolo::write_detection_export(
            &request.dataset,
            &filter,
            output_dir.join("yolo"),
        )
        .map_err(|error| error.to_string())?;
        formats.push("YOLO");
        skipped_polygons = summary.skipped_polygons;
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

    if request.classifications {
        let summary = notatus_export::classification::write_classification_export(
            &request.dataset,
            &filter,
            output_dir.join("classification"),
        )
        .map_err(|error| error.to_string())?;
        formats.push("Classification");
        classification_count = summary.classification_count;
    }

    let mut summary = format!(
        "Exported {} object annotation{} and {} image label{} as {}",
        annotation_count,
        plural(annotation_count),
        classification_count,
        plural(classification_count),
        formats.join(" and ")
    );
    if skipped_polygons > 0 {
        summary.push_str(&format!(
            "; YOLO detection left out {skipped_polygons} polygon{}",
            plural(skipped_polygons)
        ));
    }
    if left_out > 0 {
        summary.push_str(&format!(
            "; left out {left_out} unfinished media item{}",
            plural(left_out)
        ));
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notatus_core::{AnnotationGeometry, AssetStatus, Dataset};

    fn two_media_dataset() -> Dataset {
        let mut dataset = Dataset::new("demo");
        let label_id = dataset.add_label("car");
        for (name, status) in [
            ("done.jpg", AssetStatus::Done),
            ("todo.jpg", AssetStatus::Pending),
        ] {
            let mut asset = AssetRecord::new_image(AssetLocation::local(name), 100, 100).unwrap();
            asset.status = status;
            let asset_id = dataset.add_asset(asset);
            dataset.add_annotation(AnnotationRecord::new_human(
                asset_id,
                label_id,
                AnnotationGeometry::Bbox(BoundingBox::from_xywh(1.0, 1.0, 9.0, 9.0).unwrap()),
                None,
            ));
        }
        dataset
    }

    fn request(finished_only: bool) -> ExportRequest {
        ExportRequest {
            dataset: two_media_dataset(),
            yolo: false,
            coco: true,
            classifications: false,
            finished_only,
        }
    }

    fn exported_images(output: &Path) -> usize {
        std::fs::read_to_string(output.join("coco/annotations.json"))
            .unwrap()
            .matches("\"file_name\"")
            .count()
    }

    #[test]
    fn finished_only_leaves_out_unfinished_media() {
        let output = tempfile::tempdir().unwrap();

        let summary = run_export(request(true), output.path()).unwrap();

        assert_eq!(exported_images(output.path()), 1);
        assert!(
            summary.ends_with("left out 1 unfinished media item"),
            "{summary}"
        );
    }

    #[test]
    fn exporting_everything_keeps_unfinished_media() {
        let output = tempfile::tempdir().unwrap();

        let summary = run_export(request(false), output.path()).unwrap();

        assert_eq!(exported_images(output.path()), 2);
        assert!(!summary.contains("left out"));
    }
}
