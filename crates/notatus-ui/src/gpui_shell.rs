
use super::{AnnotationTool, UiState};
use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, Context, FontWeight, IntoElement, ObjectFit, PathPromptOptions,
    Pixels, Render, SharedString, WeakEntity, Window, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowOptions, div, img, px, rgb, size,
};
use gpui_component::{
    Root, Sizable as _, TitleBar,
    button::{Button, ButtonVariants as _},
    resizable::{h_resizable, resizable_panel},
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
};
use notatus_core::{AssetLocation, AssetRecord};
use std::path::{Path, PathBuf};

struct NotatusWindow {
    state: UiState,
    import_status: Option<String>,
}

impl NotatusWindow {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let mut state = UiState::new_project("Untitled dataset");
        state.set_tool(AnnotationTool::DrawBox);
        Self {
            state,
            import_status: None,
        }
    }

    fn app_titlebar(&self) -> impl IntoElement {
        TitleBar::new().child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Notatus"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6b7280))
                        .child("visual annotation"),
                ),
        )
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let asset_count = self.state.dataset.assets.len();
        let selected_asset = self
            .selected_asset()
            .map(compact_asset_name)
            .unwrap_or_else(|| "No image selected".to_string());

        div()
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .h(px(48.0))
            .px_4()
            .border_b_1()
            .border_color(rgb(0xd6d9de))
            .bg(rgb(0xf7f8fa))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap_3()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex_none()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.state.dataset.manifest.project.name.clone()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_sm()
                            .text_color(rgb(0x4b5563))
                            .child(format!("{asset_count} images")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .text_color(rgb(0x4b5563))
                            .child(selected_asset),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_3()
                    .max_w(px(420.0))
                    .overflow_hidden()
                    .when_some(self.import_status.clone(), |bar, status| {
                        bar.child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_sm()
                                .text_color(rgb(0x4b5563))
                                .child(status),
                        )
                    })
                    .child(
                        Button::new("choose-images")
                            .primary()
                            .small()
                            .label("+ Add images")
                            .on_click(move |_, _, cx| {
                                open_image_picker(view.clone(), cx);
                            }),
                    ),
            )
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let asset_items: Vec<_> = if self.state.dataset.assets.is_empty() {
            vec![SidebarMenuItem::new("No images yet").disable(true)]
        } else {
            self.state
                .dataset
                .assets
                .iter()
                .map(|asset| {
                    let asset_id = asset.id;
                    let view = view.clone();
                    SidebarMenuItem::new(compact_asset_name(asset))
                        .active(self.state.selected_asset == Some(asset_id))
                        .on_click(move |_, _, cx| {
                            let _ = view.update(cx, |window, cx| {
                                if let Err(error) = window.state.select_asset(asset_id) {
                                    window.import_status = Some(error.to_string());
                                }
                                cx.notify();
                            });
                        })
                })
                .collect()
        };

        let label_items: Vec<_> = if self.state.dataset.labels.is_empty() {
            vec![SidebarMenuItem::new("No labels yet").disable(true)]
        } else {
            self.state
                .dataset
                .labels
                .iter()
                .map(|label| SidebarMenuItem::new(label.name.clone()).suffix(sidebar_count("0")))
                .collect()
        };

        Sidebar::left().collapsible(false).w_full().child(
            SidebarMenu::new()
                .child(
                    SidebarMenuItem::new("Images")
                        .default_open(true)
                        .suffix(sidebar_count(self.state.dataset.assets.len().to_string()))
                        .children(asset_items),
                )
                .child(
                    SidebarMenuItem::new("Labels")
                        .default_open(true)
                        .children(label_items),
                ),
        )
    }

    fn canvas_placeholder(&self) -> impl IntoElement {
        let selected_asset = self.selected_asset();

        div()
            .size_full()
            .p_6()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0xf3f4f6))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .border_1()
                    .border_color(rgb(0xcbd5e1))
                    .bg(rgb(0xffffff))
                    .overflow_hidden()
                    .when_some(selected_asset, |canvas, asset| {
                        canvas.child(image_canvas_content(asset))
                    })
                    .when(selected_asset.is_none(), |canvas| {
                        canvas
                            .child(div().text_lg().child("Choose images to start"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x4b5563))
                                    .child("Use Add images in the command bar"),
                            )
                    }),
            )
    }

    fn inspector(&self) -> impl IntoElement {
        div()
            .size_full()
            .h_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .border_l_1()
            .border_color(rgb(0xd6d9de))
            .bg(rgb(0xffffff))
            .child(section_title("Selection"))
            .child(metric(
                "Active tool",
                format!("{:?}", self.state.active_tool),
            ))
            .child(metric(
                "Image",
                self.selected_asset()
                    .map(compact_asset_name)
                    .unwrap_or_else(|| "None".to_string()),
            ))
            .child(metric(
                "Dimensions",
                self.selected_asset()
                    .map(asset_dimensions_label)
                    .unwrap_or_else(|| "-".to_string()),
            ))
            .child(metric(
                "Annotations",
                self.state.dataset.annotations.len().to_string(),
            ))
            .child(metric(
                "Review queue",
                self.state
                    .dataset
                    .annotations
                    .iter()
                    .filter(|annotation| {
                        annotation.review_state == notatus_core::ReviewState::Draft
                    })
                    .count()
                    .to_string(),
            ))
    }

    fn selected_asset(&self) -> Option<&AssetRecord> {
        self.state
            .selected_asset
            .and_then(|asset_id| self.state.dataset.asset_by_id(asset_id))
    }

    fn app_frame(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_color(rgb(0x111827))
            .bg(rgb(0xf3f4f6))
            .child(self.app_titlebar())
            .child(self.toolbar(cx))
            .child(
                div().flex_1().overflow_hidden().child(
                    h_resizable("notatus-annotation-panels")
                        .child(
                            resizable_panel()
                                .size(px(240.0))
                                .size_range(px(180.0)..px(380.0))
                                .child(self.sidebar(cx)),
                        )
                        .child(
                            resizable_panel()
                                .size_range(px(320.0)..Pixels::MAX)
                                .child(self.canvas_placeholder()),
                        )
                        .child(
                            resizable_panel()
                                .size(px(280.0))
                                .size_range(px(220.0)..px(420.0))
                                .child(self.inspector()),
                        ),
                ),
            )
    }
}

impl Render for NotatusWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("notatus-window")
            .size_full()
            .bg(rgb(0xf3f4f6))
            .child(self.app_frame(cx))
    }
}

fn open_image_picker(view: WeakEntity<NotatusWindow>, cx: &mut App) {
    let _ = view.update(cx, |window, cx| {
        window.import_status = Some("Waiting for image selection".to_string());
        cx.notify();
    });

    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: true,
        prompt: Some(SharedString::from("Add images")),
    });

    cx.spawn(async move |cx| match paths.await {
        Ok(Ok(Some(paths))) => {
            let imported = inspect_image_paths(paths);
            let _ = view.update(cx, |window, cx| {
                window.apply_image_import(imported);
                cx.notify();
            });
        }
        Ok(Ok(None)) => {
            let _ = view.update(cx, |window, cx| {
                window.import_status = Some("Image import cancelled".to_string());
                cx.notify();
            });
        }
        Ok(Err(error)) => {
            let _ = view.update(cx, |window, cx| {
                window.import_status = Some(format!("Image picker failed: {error}"));
                cx.notify();
            });
        }
        Err(_) => {
            let _ = view.update(cx, |window, cx| {
                window.import_status = Some("Image picker closed unexpectedly".to_string());
                cx.notify();
            });
        }
    })
    .detach();
}

struct ImageImport {
    candidates: Vec<ImageCandidate>,
    failures: Vec<String>,
}

struct ImageCandidate {
    path: PathBuf,
    width: u32,
    height: u32,
}

fn inspect_image_paths(paths: Vec<PathBuf>) -> ImageImport {
    let mut candidates = Vec::new();
    let mut failures = Vec::new();

    for path in paths {
        match image::image_dimensions(&path) {
            Ok((width, height)) => candidates.push(ImageCandidate {
                path,
                width,
                height,
            }),
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }

    ImageImport {
        candidates,
        failures,
    }
}

impl NotatusWindow {
    fn apply_image_import(&mut self, imported: ImageImport) {
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

        self.import_status = Some(import_summary(added, failed.len()));
    }
}

fn import_summary(added: usize, failed: usize) -> String {
    match (added, failed) {
        (0, 0) => "No images selected".to_string(),
        (0, failed) => format!("Skipped {failed} invalid image{}", plural(failed)),
        (added, 0) => format!("Imported {added} image{}", plural(added)),
        (added, failed) => format!("Imported {added} image{}; skipped {failed}", plural(added)),
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn image_canvas_content(asset: &AssetRecord) -> impl IntoElement {
    match &asset.location {
        AssetLocation::LocalPath { path } => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                img(PathBuf::from(path))
                    .size_full()
                    .object_fit(ObjectFit::Contain)
                    .with_loading(|| canvas_message("Loading image").into_any_element())
                    .with_fallback(|| {
                        canvas_message("Unable to load selected image").into_any_element()
                    }),
            ),
        AssetLocation::S3Object { .. } => {
            canvas_message("Remote image preview is not implemented yet")
        }
    }
}

fn canvas_message(message: &'static str) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(rgb(0x4b5563))
        .child(message)
}

fn sidebar_count(value: impl Into<String>) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .min_w(px(24.0))
        .h(px(20.0))
        .px_1()
        .rounded_sm()
        .text_xs()
        .text_color(rgb(0x4b5563))
        .bg(rgb(0xf3f4f6))
        .child(value.into())
}

fn asset_display_name(asset: &AssetRecord) -> String {
    let display_path = asset.location.display_path();
    Path::new(display_path.as_ref())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(display_path.as_ref())
        .to_string()
}

fn compact_asset_name(asset: &AssetRecord) -> String {
    compact_text(&asset_display_name(asset), 36)
}

fn asset_dimensions_label(asset: &AssetRecord) -> String {
    format!("{}x{}", asset.dimensions.width, asset.dimensions.height)
}

fn compact_text(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars || max_chars < 8 {
        return value.to_string();
    }

    let head_count = (max_chars - 3) * 2 / 3;
    let tail_count = max_chars - 3 - head_count;
    let head: String = value.chars().take(head_count).collect();
    let tail: String = value
        .chars()
        .rev()
        .take(tail_count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}...{tail}")
}

fn dataset_has_local_path(state: &UiState, path: &str) -> bool {
    state.dataset.assets.iter().any(|asset| {
        matches!(
            &asset.location,
            AssetLocation::LocalPath { path: existing } if existing == path
        )
    })
}

fn section_title(title: &'static str) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x374151))
        .child(title)
}

fn metric(label: &'static str, value: String) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .text_sm()
        .child(div().text_color(rgb(0x4b5563)).child(label))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(value))
}

fn requested_window_decorations() -> Option<WindowDecorations> {
    if cfg!(target_os = "linux") {
        Some(WindowDecorations::Client)
    } else {
        None
    }
}

pub fn launch_gpui() {
    Application::new()
        .with_assets(gpui_component_assets::Assets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);

            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1200.0), px(760.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitleBar::title_bar_options()),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: requested_window_decorations(),
                    window_min_size: Some(size(px(720.0), px(460.0))),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| {
                        cx.observe_window_appearance(window, |_, window, _| {
                            window.refresh();
                        })
                        .detach();
                        NotatusWindow::new(window, cx)
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .unwrap();
            cx.activate(true);
        });
}
