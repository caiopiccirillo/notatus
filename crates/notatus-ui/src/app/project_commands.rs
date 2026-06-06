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

pub(super) fn save_project(
    view: gpui::WeakEntity<NotatusWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    let save_target = view.update(cx, |notatus, _| {
        notatus
            .project_location
            .path()
            .map(|path| (path.to_path_buf(), notatus.state.dataset.clone()))
    });

    match save_target {
        Ok(Some((path, dataset))) => {
            let result = LocalProjectStore::new(&path).save_dataset(&dataset);
            let _ = view.update(cx, |notatus, cx| {
                match result {
                    Ok(()) => {
                        notatus.state.mark_saved();
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
        Ok(None) => save_project_as(view, window, cx),
        Err(_) => {}
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
            Ok(Ok(Some(path))) => save_current_project_to_path(view, path, window),
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

    window.open_dialog(cx, move |dialog, _, _| {
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
                apply_project_load_result(view, result, Some(path), "Created project", window);
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
                let result = LocalProjectStore::new(&path)
                    .load_dataset()
                    .map_err(|error| error.to_string());
                apply_project_load_result(view, result, Some(path), "Opened project", window);
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

fn save_current_project_to_path(
    view: gpui::WeakEntity<NotatusWindow>,
    path: PathBuf,
    window: &mut gpui::AsyncWindowContext,
) {
    let dataset = view.update_in(window, |notatus, _, _| notatus.state.dataset.clone());
    let result = dataset
        .map_err(|_| "window closed".to_string())
        .and_then(|dataset| {
            LocalProjectStore::new(&path)
                .save_dataset(&dataset)
                .map_err(|error| error.to_string())
        });

    let _ = view.update_in(window, |notatus, _, cx| {
        match result {
            Ok(()) => {
                notatus.state.mark_saved();
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
    window: &mut gpui::AsyncWindowContext,
) {
    let _ = view.update_in(window, |notatus, window, cx| {
        match result
            .and_then(|dataset| UiState::from_dataset(dataset).map_err(|error| error.to_string()))
        {
            Ok(mut state) => {
                state.set_tool(AnnotationTool::DrawBox);
                notatus.state = state;
                notatus.project_location = location
                    .map(ProjectLocation::local)
                    .unwrap_or(ProjectLocation::Unsaved);
                notatus.left_dock = LeftDock::Project;
                notatus.right_dock = RightDock::Info;
                notatus.tools.fit_canvas_to_view();
                notatus.canvas_image_layout.borrow_mut().take();
                notatus.status_message = Some(success_message.to_string());
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
    fn suggested_project_folder_normalizes_name() {
        assert_eq!(suggested_project_folder("Road Signs 01"), "road-signs-01");
        assert_eq!(suggested_project_folder("   "), "notatus-project");
    }
}
