//! Desktop application state for the Notatus GPUI client.
//!
//! The GPUI windowing code is intentionally thin. The mutable annotation state
//! lives here so the same behavior can be tested without a renderer.

use notatus_core::{
    AnnotationGeometry, AnnotationId, AnnotationRecord, AssetId, AssetLocation, AssetRecord,
    BoundingBox, Dataset, GeometryError, LabelId, ValidationError,
};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnnotationTool {
    Select,
    DrawBox,
    Pan,
}

#[derive(Clone, Debug)]
pub struct UiState {
    pub dataset: Dataset,
    pub active_tool: AnnotationTool,
    pub selected_asset: Option<AssetId>,
    pub selected_annotation: Option<AnnotationId>,
    pub dirty: bool,
}

impl UiState {
    pub fn new_project(name: impl Into<String>) -> Self {
        Self {
            dataset: Dataset::new(name),
            active_tool: AnnotationTool::Select,
            selected_asset: None,
            selected_annotation: None,
            dirty: false,
        }
    }

    pub fn from_dataset(dataset: Dataset) -> Result<Self, UiMutationError> {
        dataset.validate()?;
        Ok(Self {
            dataset,
            active_tool: AnnotationTool::Select,
            selected_asset: None,
            selected_annotation: None,
            dirty: false,
        })
    }

    pub fn set_tool(&mut self, tool: AnnotationTool) {
        self.active_tool = tool;
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn add_label(&mut self, name: impl Into<String>) -> LabelId {
        let label_id = self.dataset.add_label(name);
        self.dirty = true;
        label_id
    }

    pub fn add_local_image_asset(
        &mut self,
        path: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<AssetId, UiMutationError> {
        let asset = AssetRecord::new_image(AssetLocation::local(path), width, height)?;
        let asset_id = self.dataset.add_asset(asset);
        self.selected_asset = Some(asset_id);
        self.dirty = true;
        Ok(asset_id)
    }

    pub fn select_asset(&mut self, asset_id: AssetId) -> Result<(), UiMutationError> {
        if self.dataset.asset_by_id(asset_id).is_none() {
            return Err(UiMutationError::MissingAsset { asset_id });
        }

        self.selected_asset = Some(asset_id);
        self.selected_annotation = None;
        Ok(())
    }

    pub fn add_human_bbox(
        &mut self,
        asset_id: AssetId,
        label_id: LabelId,
        bbox: BoundingBox,
        user_id: Option<String>,
    ) -> Result<AnnotationId, UiMutationError> {
        let asset = self
            .dataset
            .asset_by_id(asset_id)
            .ok_or(UiMutationError::MissingAsset { asset_id })?;
        if self.dataset.label_by_id(label_id).is_none() {
            return Err(UiMutationError::MissingLabel { label_id });
        }
        bbox.validate_within_image(asset.dimensions)?;

        let annotation = AnnotationRecord::new_human(
            asset_id,
            label_id,
            AnnotationGeometry::Bbox(bbox),
            user_id,
        );
        let annotation_id = annotation.id;
        self.dataset.add_annotation(annotation);
        self.selected_annotation = Some(annotation_id);
        self.dirty = true;
        Ok(annotation_id)
    }
}

#[derive(Debug)]
pub enum UiMutationError {
    Geometry(GeometryError),
    Validation(ValidationError),
    MissingAsset { asset_id: AssetId },
    MissingLabel { label_id: LabelId },
}

impl fmt::Display for UiMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geometry(source) => write!(f, "{source}"),
            Self::Validation(source) => write!(f, "{source}"),
            Self::MissingAsset { asset_id } => write!(f, "missing asset {asset_id}"),
            Self::MissingLabel { label_id } => write!(f, "missing label {label_id}"),
        }
    }
}

impl Error for UiMutationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Geometry(source) => Some(source),
            Self::Validation(source) => Some(source),
            Self::MissingAsset { .. } | Self::MissingLabel { .. } => None,
        }
    }
}

impl From<GeometryError> for UiMutationError {
    fn from(source: GeometryError) -> Self {
        Self::Geometry(source)
    }
}

impl From<ValidationError> for UiMutationError {
    fn from(source: ValidationError) -> Self {
        Self::Validation(source)
    }
}

#[cfg(feature = "gpui-ui")]
pub fn launch_gpui() {
    gpui_shell::launch_gpui();
}

#[cfg(feature = "gpui-ui")]
mod gpui_shell {
    use super::{AnnotationTool, UiState};
    use gpui::prelude::*;
    use gpui::{
        App, Application, Bounds, Context, CursorStyle, Decorations, FontWeight, HitboxBehavior,
        IntoElement, MouseButton, ObjectFit, PathPromptOptions, Pixels, Point, Render, ResizeEdge,
        SharedString, Size, WeakEntity, Window, WindowBackgroundAppearance, WindowBounds,
        WindowDecorations, WindowOptions, canvas, div, img, point, px, rgb, size,
    };
    use gpui_component::{
        Icon, IconName, Root, Sizable as _,
        button::{Button, ButtonVariants as _},
        resizable::{h_resizable, resizable_panel},
        sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    };
    use notatus_core::{AssetLocation, AssetRecord};
    use std::path::{Path, PathBuf};

    const CLIENT_TITLEBAR_HEIGHT: Pixels = px(36.0);
    const CLIENT_RESIZE_EDGE: Pixels = px(8.0);

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
            div()
                .id("notatus-client-titlebar")
                .flex_none()
                .flex()
                .items_center()
                .justify_between()
                .h(CLIENT_TITLEBAR_HEIGHT)
                .pl_3()
                .bg(rgb(0xf9fafb))
                .border_b_1()
                .border_color(rgb(0xd6d9de))
                .on_mouse_down(MouseButton::Left, |event, window, cx| {
                    if event.click_count >= 2 {
                        window.zoom_window();
                    } else {
                        window.start_window_move();
                    }
                    cx.stop_propagation();
                })
                .on_click(|event, window, cx| {
                    if event.is_right_click() {
                        window.show_window_menu(event.position());
                        cx.stop_propagation();
                    }
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
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
                .child(titlebar_controls())
        }

        fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
            let view = cx.weak_entity();
            let asset_count = self.state.dataset.assets.len();
            let selected_asset = self
                .selected_asset()
                .map(asset_display_name)
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
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(self.state.dataset.manifest.project.name.clone()),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x4b5563))
                                .child(format!("{asset_count} images")),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x4b5563))
                                .child(selected_asset),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .when_some(self.import_status.clone(), |bar, status| {
                            bar.child(div().text_sm().text_color(rgb(0x4b5563)).child(status))
                        })
                        .child(
                            Button::new("choose-images")
                                .primary()
                                .small()
                                .icon(IconName::Plus)
                                .label("Add images")
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
                        SidebarMenuItem::new(asset_display_name(asset))
                            .icon(IconName::File)
                            .active(self.state.selected_asset == Some(asset_id))
                            .suffix(sidebar_count(asset_dimensions_label(asset)))
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
                    .map(|label| {
                        SidebarMenuItem::new(label.name.clone()).suffix(sidebar_count("0"))
                    })
                    .collect()
            };

            Sidebar::left()
                .collapsible(false)
                .w_full()
                .header(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(self.state.dataset.manifest.project.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x6b7280))
                                        .child("local project"),
                                ),
                        )
                        .child(sidebar_count(self.state.dataset.assets.len().to_string())),
                )
                .child(
                    SidebarMenu::new()
                        .child(
                            SidebarMenuItem::new("Images")
                                .icon(IconName::GalleryVerticalEnd)
                                .default_open(true)
                                .suffix(sidebar_count(self.state.dataset.assets.len().to_string()))
                                .children(asset_items),
                        )
                        .child(
                            SidebarMenuItem::new("Labels")
                                .icon(IconName::CaseSensitive)
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
                        .map(asset_display_name)
                        .unwrap_or_else(|| "None".to_string()),
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

        fn app_frame(&self, app_chrome: bool, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .flex()
                .flex_col()
                .text_color(rgb(0x111827))
                .bg(rgb(0xf3f4f6))
                .when(app_chrome, |frame| frame.child(self.app_titlebar()))
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
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let app_chrome = should_use_app_chrome(window.window_decorations());
            if app_chrome {
                window.set_client_inset(CLIENT_RESIZE_EDGE);
            }

            div()
                .id("notatus-window")
                .size_full()
                .bg(rgb(0xf3f4f6))
                .when(app_chrome, |root| {
                    root.child(resize_cursor_layer())
                        .on_mouse_move(|_, window, _| window.refresh())
                        .on_mouse_down(MouseButton::Left, |event, window, cx| {
                            let size = window.window_bounds().get_bounds().size;
                            if let Some(edge) =
                                resize_edge(event.position, CLIENT_RESIZE_EDGE, size)
                            {
                                window.start_window_resize(edge);
                                cx.stop_propagation();
                            }
                        })
                })
                .child(self.app_frame(app_chrome, cx))
        }
    }

    fn resize_cursor_layer() -> impl IntoElement {
        canvas(
            |_bounds, window, _cx| {
                window.insert_hitbox(
                    Bounds::new(
                        point(px(0.0), px(0.0)),
                        window.window_bounds().get_bounds().size,
                    ),
                    HitboxBehavior::Normal,
                )
            },
            |_bounds, hitbox, window, _cx| {
                let size = window.window_bounds().get_bounds().size;
                let Some(edge) = resize_edge(window.mouse_position(), CLIENT_RESIZE_EDGE, size)
                else {
                    return;
                };
                window.set_cursor_style(cursor_for_resize_edge(edge), &hitbox);
            },
        )
        .size_full()
        .absolute()
    }

    fn titlebar_controls() -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .h_full()
            .child(titlebar_button(
                "window-minimize",
                IconName::WindowMinimize,
                rgb(0xe5e7eb),
                |window, _| window.minimize_window(),
            ))
            .child(titlebar_button(
                "window-maximize",
                IconName::WindowMaximize,
                rgb(0xe5e7eb),
                |window, _| window.zoom_window(),
            ))
            .child(titlebar_button(
                "window-close",
                IconName::WindowClose,
                rgb(0xdc2626),
                |window, _| window.remove_window(),
            ))
    }

    fn titlebar_button(
        id: &'static str,
        icon: IconName,
        hover_color: impl Into<gpui::Hsla>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        let hover_color = hover_color.into();

        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .w(CLIENT_TITLEBAR_HEIGHT)
            .h_full()
            .text_color(rgb(0x111827))
            .hover(move |button| button.bg(hover_color).text_color(rgb(0xffffff)))
            .active(|button| button.bg(rgb(0xd1d5db)))
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_click(move |_, window, cx| {
                on_click(window, cx);
                cx.stop_propagation();
            })
            .child(Icon::new(icon).small())
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

    fn asset_dimensions_label(asset: &AssetRecord) -> String {
        format!("{}x{}", asset.dimensions.width, asset.dimensions.height)
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

    fn cursor_for_resize_edge(edge: ResizeEdge) -> CursorStyle {
        match edge {
            ResizeEdge::Top | ResizeEdge::Bottom => CursorStyle::ResizeUpDown,
            ResizeEdge::Left | ResizeEdge::Right => CursorStyle::ResizeLeftRight,
            ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorStyle::ResizeUpLeftDownRight,
            ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
        }
    }

    fn resize_edge(
        pos: Point<Pixels>,
        edge_size: Pixels,
        size: Size<Pixels>,
    ) -> Option<ResizeEdge> {
        let edge = if pos.y < edge_size && pos.x < edge_size {
            ResizeEdge::TopLeft
        } else if pos.y < edge_size && pos.x > size.width - edge_size {
            ResizeEdge::TopRight
        } else if pos.y < edge_size {
            ResizeEdge::Top
        } else if pos.y > size.height - edge_size && pos.x < edge_size {
            ResizeEdge::BottomLeft
        } else if pos.y > size.height - edge_size && pos.x > size.width - edge_size {
            ResizeEdge::BottomRight
        } else if pos.y > size.height - edge_size {
            ResizeEdge::Bottom
        } else if pos.x < edge_size {
            ResizeEdge::Left
        } else if pos.x > size.width - edge_size {
            ResizeEdge::Right
        } else {
            return None;
        };
        Some(edge)
    }

    fn should_use_app_chrome(decorations: Decorations) -> bool {
        cfg!(any(target_os = "linux", target_os = "freebsd"))
            || matches!(decorations, Decorations::Client { .. })
    }

    fn requested_window_decorations() -> Option<WindowDecorations> {
        if cfg!(any(target_os = "linux", target_os = "freebsd")) {
            Some(WindowDecorations::Client)
        } else {
            None
        }
    }

    pub fn launch_gpui() {
        Application::new().run(|cx: &mut App| {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_bbox_through_ui_state() {
        let mut state = UiState::new_project("demo");
        let label_id = state.add_label("car");
        let asset_id = state
            .add_local_image_asset("images/a.jpg", 640, 480)
            .unwrap();
        let annotation_id = state
            .add_human_bbox(
                asset_id,
                label_id,
                BoundingBox::from_xywh(10.0, 20.0, 30.0, 40.0).unwrap(),
                None,
            )
            .unwrap();

        assert_eq!(state.selected_annotation, Some(annotation_id));
        assert!(state.dirty);
        assert!(state.dataset.validate().is_ok());
    }
}
