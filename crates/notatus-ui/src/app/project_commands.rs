use super::*;

#[derive(Clone, Copy)]
enum ProjectPickerAction {
    New,
    Open,
}

pub(super) fn new_project(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    run_or_confirm_discard(view, window, cx, ProjectPickerAction::New);
}

pub(super) fn open_project(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    run_or_confirm_discard(view, window, cx, ProjectPickerAction::Open);
}

/// Saves to the project folder through the autosave pipeline, so it never
/// writes at the same time as an autosave. Projects without a folder ask
/// for one first.
pub(super) fn save_project(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    let has_folder = view.update(cx, |notatus, cx| {
        if notatus.project_location.path().is_none() {
            return false;
        }
        notatus.save_now(cx);
        true
    });

    if matches!(has_folder, Ok(false)) {
        save_project_as(view, window, cx);
    }
}

pub(super) fn save_project_as(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    let suggested_name = view
        .update(cx, |notatus, _| {
            suggested_project_folder(&notatus.state.dataset.manifest.project.name)
        })
        .unwrap_or_else(|_| "notatus-project".to_string());
    let directory = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let path = cx.prompt_for_new_path(&directory, Some(&suggested_name));

    window
        .spawn(cx, async move |window| match path.await {
            Ok(Ok(Some(path))) => save_current_project_to_path(view, path, window).await,
            Ok(Ok(None)) => {
                let _ = view.update_in(window, |notatus, _, cx| {
                    notatus.status_message = Some("Save cancelled".to_string());
                    cx.notify();
                });
            }
            Ok(Err(error)) => {
                let _ = view.update_in(window, |notatus, _, cx| {
                    notatus.status_message = Some(format!("Save picker failed: {error}"));
                    cx.notify();
                });
            }
            Err(_) => {
                let _ = view.update_in(window, |notatus, _, cx| {
                    notatus.status_message = Some("Save picker closed unexpectedly".to_string());
                    cx.notify();
                });
            }
        })
        .detach();
}

fn run_or_confirm_discard(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
    action: ProjectPickerAction,
) {
    let dirty = view
        .update(cx, |notatus, _| notatus.state.dirty)
        .unwrap_or(false);

    if !dirty {
        start_project_picker(view, window, cx, action);
        return;
    }

    window.open_alert_dialog(cx, move |dialog, _, _| {
        let action_view = view.clone();
        dialog
            .confirm()
            .title("Unsaved changes")
            .child("Opening or creating another project will discard unsaved changes.")
            .button_props(
                DialogButtonProps::default()
                    .ok_text("Discard changes")
                    .cancel_text("Cancel"),
            )
            .on_ok(move |_, window, cx| {
                start_project_picker(action_view.clone(), window, cx, action);
                true
            })
    });
}

fn start_project_picker(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
    action: ProjectPickerAction,
) {
    match action {
        ProjectPickerAction::New => prompt_for_new_project(view, window, cx),
        ProjectPickerAction::Open => prompt_for_open_project(view, window, cx),
    }
}

fn prompt_for_new_project(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    let _ = view.update(cx, |notatus, cx| {
        notatus.status_message = Some("Choose a folder for the new project".to_string());
        cx.notify();
    });

    let directory = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let path = cx.prompt_for_new_path(&directory, Some("notatus-project"));

    window
        .spawn(cx, async move |window| match path.await {
            Ok(Ok(Some(path))) => {
                let result = initialize_project_at(&path);
                apply_project_load_result(
                    view,
                    result,
                    Some(path),
                    "Created project",
                    None,
                    window,
                );
            }
            Ok(Ok(None)) => {
                let _ = view.update_in(window, |notatus, _, cx| {
                    notatus.status_message = Some("New project cancelled".to_string());
                    cx.notify();
                });
            }
            Ok(Err(error)) => {
                let _ = view.update_in(window, |notatus, _, cx| {
                    notatus.status_message = Some(format!("New project picker failed: {error}"));
                    cx.notify();
                });
            }
            Err(_) => {
                let _ = view.update_in(window, |notatus, _, cx| {
                    notatus.status_message =
                        Some("New project picker closed unexpectedly".to_string());
                    cx.notify();
                });
            }
        })
        .detach();
}

fn prompt_for_open_project(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    let _ = view.update(cx, |notatus, cx| {
        notatus.status_message = Some("Choose a project folder".to_string());
        cx.notify();
    });

    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some(SharedString::from("Open project")),
    });

    window
        .spawn(cx, async move |window| match paths.await {
            Ok(Ok(Some(paths))) => {
                let Some(path) = paths.into_iter().next() else {
                    return;
                };
                let load_path = path.clone();
                let result =
                    window
                        .background_executor()
                        .spawn(async move {
                            LocalProjectStore::new(&load_path).load_dataset_with_report()
                        })
                        .await
                        .map_err(|error| error.to_string());
                let notice = result
                    .as_ref()
                    .ok()
                    .and_then(|(_, report)| load_report_notice(report));
                apply_project_load_result(
                    view,
                    result.map(|(dataset, _)| dataset),
                    Some(path),
                    "Opened project",
                    notice,
                    window,
                );
            }
            Ok(Ok(None)) => {
                let _ = view.update_in(window, |notatus, _, cx| {
                    notatus.status_message = Some("Open project cancelled".to_string());
                    cx.notify();
                });
            }
            Ok(Err(error)) => {
                let _ = view.update_in(window, |notatus, _, cx| {
                    notatus.status_message = Some(format!("Open project picker failed: {error}"));
                    cx.notify();
                });
            }
            Err(_) => {
                let _ = view.update_in(window, |notatus, _, cx| {
                    notatus.status_message =
                        Some("Open project picker closed unexpectedly".to_string());
                    cx.notify();
                });
            }
        })
        .detach();
}

fn initialize_project_at(path: &Path) -> Result<notatus_core::Dataset, String> {
    if path.join(MANIFEST_FILE).exists() {
        return Err(format!(
            "{} already contains a Notatus project; use Open project",
            path.display()
        ));
    }

    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Untitled project");
    LocalProjectStore::new(path)
        .initialize(name)
        .map_err(|error| error.to_string())
}

async fn save_current_project_to_path(
    view: gpui::WeakEntity<NotatusWindow>,
    path: PathBuf,
    window: &mut gpui::AsyncWindowContext,
) {
    let Ok((dataset, revision, conflict)) = view.update_in(window, |notatus, _, _| {
        (
            notatus.state.dataset.clone(),
            notatus.state.revision(),
            save_as_conflict(&path, notatus.project_location.path()),
        )
    }) else {
        return;
    };
    if let Some(conflict) = conflict {
        let _ = view.update_in(window, |notatus, _, cx| {
            notatus.status_message = Some(format!("Save failed: {conflict}"));
            cx.notify();
        });
        return;
    }
    let save_path = path.clone();
    let result = window
        .background_executor()
        .spawn(async move { LocalProjectStore::new(&save_path).save_dataset(&dataset) })
        .await
        .map_err(|error| error.to_string());

    let _ = view.update_in(window, |notatus, _, cx| {
        match result {
            Ok(()) => {
                // A save still running for the old folder must not mark
                // anything saved in the new one.
                notatus.autosave.reset();
                notatus.state.mark_revision_saved(revision);
                notatus.project_location = ProjectLocation::local(path);
                notatus.status_message = Some("Saved project".to_string());
            }
            Err(error) => {
                notatus.status_message = Some(format!("Save failed: {error}"));
            }
        }
        cx.notify();
    });
}

fn apply_project_load_result(
    view: gpui::WeakEntity<NotatusWindow>,
    result: Result<notatus_core::Dataset, String>,
    location: Option<PathBuf>,
    success_message: &'static str,
    notice: Option<String>,
    window: &mut gpui::AsyncWindowContext,
) {
    let _ = view.update_in(window, |notatus, window, cx| {
        match result
            .and_then(|dataset| UiState::from_dataset(dataset).map_err(|error| error.to_string()))
        {
            Ok(mut state) => {
                state.set_tool(AnnotationTool::DrawBox);
                notatus.state = state;
                notatus.autosave.reset();
                notatus.project_location = location
                    .map(ProjectLocation::local)
                    .unwrap_or(ProjectLocation::Unsaved);
                notatus.left_dock = LeftDock::Dataset;
                notatus.right_dock = RightDock::Annotations;
                notatus.tools.fit_canvas_to_view();
                notatus.canvas_image_layout.borrow_mut().take();
                notatus.status_message = Some(success_message.to_string());
                if let Some(notice) = notice {
                    notatus.status_message = Some(format!("{success_message}; {notice}"));
                    window.push_notification(
                        Notification::warning(notice)
                            .title("Some records were set aside")
                            .autohide(false),
                        cx,
                    );
                }
                notatus.sync_project_name_input(window, cx);
                notatus.sync_label_name_input(window, cx);
            }
            Err(error) => {
                notatus.status_message = Some(format!("{success_message} failed: {error}"));
            }
        }
        cx.notify();
    });
}

/// Why saving the project to `target` would destroy another project, if it
/// would: the folder already holds a different Notatus project.
fn save_as_conflict(target: &Path, current: Option<&Path>) -> Option<String> {
    if !target.join(MANIFEST_FILE).exists() {
        return None;
    }
    let same_folder = current.is_some_and(|current| {
        current == target
            || current
                .canonicalize()
                .ok()
                .zip(target.canonicalize().ok())
                .is_some_and(|(current, target)| current == target)
    });
    (!same_folder).then(|| {
        format!(
            "{} already contains another Notatus project; open it or choose an empty folder",
            target.display()
        )
    })
}

/// What to tell someone when opening their project skipped records.
fn load_report_notice(report: &notatus_storage::LoadReport) -> Option<String> {
    const SHOWN: usize = 3;
    if report.is_clean() {
        return None;
    }
    let count = report.issues.len();
    let mut notice = format!(
        "{count} record{} could not be used and {} left out",
        super::helpers::plural(count),
        if count == 1 { "was" } else { "were" }
    );
    if let Some(dir) = &report.recovery_dir {
        notice.push_str(&format!(
            ". The original files were copied to {}",
            dir.display()
        ));
    }
    notice.push(':');
    for issue in report.issues.iter().take(SHOWN) {
        notice.push_str(&format!("\n• {issue}"));
    }
    if count > SHOWN {
        notice.push_str(&format!("\n• …and {} more", count - SHOWN));
    }
    Some(notice)
}

fn suggested_project_folder(name: &str) -> String {
    let suggested: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let suggested = suggested.trim_matches('-');
    if suggested.is_empty() {
        "notatus-project".to_string()
    } else {
        suggested.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_as_refuses_to_overwrite_another_project() {
        let other = tempfile::tempdir().unwrap();
        LocalProjectStore::new(other.path())
            .initialize("other")
            .unwrap();
        let empty = tempfile::tempdir().unwrap();

        assert!(save_as_conflict(other.path(), None).is_some());
        assert!(save_as_conflict(other.path(), Some(empty.path())).is_some());
        assert_eq!(save_as_conflict(other.path(), Some(other.path())), None);
        assert_eq!(save_as_conflict(empty.path(), None), None);
    }

    #[test]
    fn load_notice_summarizes_skipped_records() {
        let clean = notatus_storage::LoadReport::default();
        assert_eq!(load_report_notice(&clean), None);

        let issue = |message: &str| notatus_storage::LoadIssue {
            file: notatus_storage::ANNOTATIONS_FILE,
            line: None,
            message: message.to_string(),
        };
        let report = notatus_storage::LoadReport {
            issues: (1..=5).map(|n| issue(&format!("problem {n}"))).collect(),
            recovery_dir: Some(PathBuf::from("/p/.notatus-recovery/x")),
        };
        let notice = load_report_notice(&report).unwrap();

        assert!(notice.starts_with("5 records could not be used and were left out"));
        assert!(notice.contains("/p/.notatus-recovery/x"));
        assert!(notice.contains("annotations.jsonl: problem 3"));
        assert!(!notice.contains("problem 4"));
        assert!(notice.ends_with("…and 2 more"));
    }

    #[test]
    fn suggested_project_folder_normalizes_name() {
        assert_eq!(suggested_project_folder("Road Signs 01"), "road-signs-01");
        assert_eq!(suggested_project_folder("   "), "notatus-project");
    }
}
