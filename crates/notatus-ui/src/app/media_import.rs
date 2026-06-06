use super::helpers::{dataset_has_local_path, plural};
use super::*;

impl NotatusWindow {
    fn apply_media_import(&mut self, imported: MediaImport) {
        let mut added = 0;
        let mut failed = imported.failures;

        for candidate in imported.candidates {
            let path = candidate.path.to_string_lossy().into_owned();
            if dataset_has_local_path(&self.state, &path) {
                failed.push(format!("{path}: already imported"));
                continue;
            }

            match self
                .state
                .add_local_image_asset(path, candidate.width, candidate.height)
            {
                Ok(_) => added += 1,
                Err(error) => failed.push(error.to_string()),
            }
        }

        if added > 0 {
            self.left_dock = LeftDock::Media;
        }
        self.status_message = Some(media_import_summary(added, failed.len()));
    }
}
struct MediaImport {
    candidates: Vec<MediaCandidate>,
    failures: Vec<String>,
}

struct MediaCandidate {
    path: PathBuf,
    width: u32,
    height: u32,
}

pub(super) fn open_media_picker(view: gpui::WeakEntity<NotatusWindow>, cx: &mut App) {
    let _ = view.update(cx, |window, cx| {
        window.status_message = Some("Waiting for media selection".to_string());
        cx.notify();
    });

    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: true,
        prompt: Some(SharedString::from("Import media")),
    });

    cx.spawn(async move |cx| match paths.await {
        Ok(Ok(Some(paths))) => {
            let imported = inspect_media_paths(paths);
            let _ = view.update(cx, |window, cx| {
                window.apply_media_import(imported);
                cx.notify();
            });
        }
        Ok(Ok(None)) => {
            let _ = view.update(cx, |window, cx| {
                window.status_message = Some("Media import cancelled".to_string());
                cx.notify();
            });
        }
        Ok(Err(error)) => {
            let _ = view.update(cx, |window, cx| {
                window.status_message = Some(format!("Media picker failed: {error}"));
                cx.notify();
            });
        }
        Err(_) => {
            let _ = view.update(cx, |window, cx| {
                window.status_message = Some("Media picker closed unexpectedly".to_string());
                cx.notify();
            });
        }
    })
    .detach();
}

fn inspect_media_paths(paths: Vec<PathBuf>) -> MediaImport {
    let mut candidates = Vec::new();
    let mut failures = Vec::new();

    for path in paths {
        match image::image_dimensions(&path) {
            Ok((width, height)) => candidates.push(MediaCandidate {
                path,
                width,
                height,
            }),
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }

    MediaImport {
        candidates,
        failures,
    }
}

fn media_import_summary(added: usize, failed: usize) -> String {
    match (added, failed) {
        (0, 0) => "No media selected".to_string(),
        (0, failed) => format!("Skipped {failed} unsupported file{}", plural(failed)),
        (added, 0) => format!("Imported {added} media item{}", plural(added)),
        (added, failed) => {
            format!(
                "Imported {added} media item{}; skipped {failed}",
                plural(added)
            )
        }
    }
}
