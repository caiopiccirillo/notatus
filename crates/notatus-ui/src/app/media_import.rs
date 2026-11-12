use super::helpers::plural;
use super::*;
use std::collections::HashSet;

impl NotatusWindow {
    fn apply_media_import(&mut self, imported: MediaImport) {
        let mut failed = imported.failures;
        let first_new = self.state.dataset.assets.len();
        // Collected once, so a large import does not rescan every asset for
        // every file.
        let mut known_paths: HashSet<String> = self
            .state
            .dataset
            .assets
            .iter()
            .filter_map(|asset| match &asset.location {
                AssetLocation::LocalPath { path } => Some(path.clone()),
                _ => None,
            })
            .collect();

        let added = self.state.batch("Import media", |state| {
            let mut added = 0;
            for candidate in imported.candidates {
                let path = candidate.path.to_string_lossy().into_owned();
                if !known_paths.insert(path.clone()) {
                    failed.push(format!("{path}: already imported"));
                    continue;
                }

                match state.add_local_image_asset(path, candidate.width, candidate.height) {
                    Ok(_) => added += 1,
                    Err(error) => failed.push(error.to_string()),
                }
            }
            added
        });

        if added > 0 {
            // Start on the first imported item rather than the last.
            self.state.selected_asset = self.state.dataset.assets.get(first_new).map(|a| a.id);
            self.tools.fit_canvas_to_view();
            self.canvas_image_layout.borrow_mut().take();
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

pub(super) fn open_media_picker(
    view: gpui::WeakEntity<NotatusWindow>,
    _window: &mut Window,
    cx: &mut App,
) {
    let picking = view.update(cx, |notatus, cx| {
        notatus.status_message = Some("Waiting for media selection".to_string());
        cx.notify();
    });
    if picking.is_err() {
        return;
    }

    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: true,
        prompt: Some(SharedString::from("Import media")),
    });

    cx.spawn(async move |cx| match paths.await {
        Ok(Ok(Some(paths))) => {
            // Reading every file's header can take a while for large
            // selections, so it runs off the UI thread.
            let imported = cx
                .background_executor()
                .spawn(async move { inspect_media_paths(paths) })
                .await;
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
        match displayed_dimensions(&path) {
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

/// The image's size as it is shown: after the rotation its EXIF orientation
/// asks for, which the canvas applies when drawing it and which OpenCV and
/// Ultralytics apply when training. Only the header is read.
fn displayed_dimensions(path: &Path) -> image::ImageResult<(u32, u32)> {
    use image::ImageDecoder as _;
    use image::metadata::Orientation;

    let mut decoder = image::ImageReader::open(path)?.into_decoder()?;
    let (width, height) = decoder.dimensions();
    let quarter_turn = matches!(
        decoder.orientation()?,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    );
    Ok(if quarter_turn {
        (height, width)
    } else {
        (width, height)
    })
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

#[cfg(test)]
mod tests {
    use super::super::test_support::open_test_window;
    use super::*;
    use gpui::TestAppContext;

    /// A JPEG of `width`x`height` stored pixels whose EXIF orientation says
    /// to rotate it a quarter turn clockwise, as phone cameras write them.
    fn sideways_jpeg(path: &Path, width: u32, height: u32) {
        let mut encoded = Vec::new();
        image::RgbImage::new(width, height)
            .write_to(
                &mut std::io::Cursor::new(&mut encoded),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        // APP1 Exif segment: little-endian TIFF header and one IFD entry,
        // Orientation (0x0112) = 6, "rotate 90 clockwise".
        let exif: &[u8] = &[
            0xFF, 0xE1, 0x00, 0x22, b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 0x2A, 0x00, 0x08,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x12, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x06,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut bytes = encoded[..2].to_vec();
        bytes.extend_from_slice(exif);
        bytes.extend_from_slice(&encoded[2..]);
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn sideways_photos_use_their_displayed_size() {
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("phone.jpg");
        sideways_jpeg(&photo, 8, 4);

        assert_eq!(displayed_dimensions(&photo).unwrap(), (4, 8));
        assert_eq!(image::image_dimensions(&photo).unwrap(), (8, 4), "raw size");
    }

    #[gpui::test]
    fn import_reads_sizes_and_skips_files_already_in_the_project(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let wide = dir.path().join("wide.png");
        let tall = dir.path().join("tall.png");
        let text = dir.path().join("notes.txt");
        image::RgbImage::new(6, 2).save(&wide).unwrap();
        image::RgbImage::new(2, 6).save(&tall).unwrap();
        std::fs::write(&text, "not an image").unwrap();
        let (_, view) = open_test_window(cx);

        view.update(cx, |notatus, _| {
            // The same file twice in one selection is imported once.
            notatus.apply_media_import(inspect_media_paths(vec![
                wide.clone(),
                wide.clone(),
                text.clone(),
            ]));
            assert_eq!(notatus.state.dataset.assets.len(), 1);
            let asset = &notatus.state.dataset.assets[0];
            assert_eq!((asset.dimensions.width, asset.dimensions.height), (6, 2));
            assert_eq!(
                notatus.status_message.as_deref(),
                Some("Imported 1 media item; skipped 2")
            );

            notatus.apply_media_import(inspect_media_paths(vec![wide.clone(), tall.clone()]));
            assert_eq!(notatus.state.dataset.assets.len(), 2);
            assert_eq!(
                notatus.status_message.as_deref(),
                Some("Imported 1 media item; skipped 1")
            );
        });
    }
}
