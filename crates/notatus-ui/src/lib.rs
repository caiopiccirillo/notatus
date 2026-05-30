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
        IntoElement, MouseButton, Pixels, Point, Render, ResizeEdge, Size, Window,
        WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowOptions, canvas, div,
        point, px, rgb, size,
    };
    use gpui_component::{Root, TitleBar};

    const CLIENT_RESIZE_EDGE: Pixels = px(8.0);

    struct NotatusWindow {
        state: UiState,
    }

    impl NotatusWindow {
        fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
            let mut state = UiState::new_project("Untitled dataset");
            state.set_tool(AnnotationTool::DrawBox);
            Self { state }
        }

        fn app_titlebar(&self) -> impl IntoElement {
            TitleBar::new()
                .bg(rgb(0xf9fafb))
                .border_color(rgb(0xd6d9de))
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
        }

        fn toolbar(&self) -> impl IntoElement {
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
                                .child("Notatus"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x4b5563))
                                .child(self.state.dataset.manifest.project.name.clone()),
                        ),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x166534))
                        .child("local project"),
                )
        }

        fn sidebar(&self) -> impl IntoElement {
            let labels = if self.state.dataset.labels.is_empty() {
                vec![
                    div()
                        .text_sm()
                        .text_color(rgb(0x6b7280))
                        .child("No labels yet"),
                ]
            } else {
                self.state
                    .dataset
                    .labels
                    .iter()
                    .map(|label| {
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .h(px(28.0))
                            .child(label.name.clone())
                            .child(div().text_xs().text_color(rgb(0x6b7280)).child("0"))
                    })
                    .collect()
            };

            div()
                .flex_none()
                .w(px(240.0))
                .h_full()
                .flex()
                .flex_col()
                .gap_4()
                .p_4()
                .border_r_1()
                .border_color(rgb(0xd6d9de))
                .bg(rgb(0xffffff))
                .child(section_title("Assets"))
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x4b5563))
                        .child(format!("{} images", self.state.dataset.assets.len())),
                )
                .child(section_title("Labels"))
                .children(labels)
        }

        fn canvas_placeholder(&self) -> impl IntoElement {
            div()
                .flex_1()
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
                        .child(div().text_lg().child("Image canvas"))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x4b5563))
                                .child("Bounding-box drawing will attach here"),
                        ),
                )
        }

        fn inspector(&self) -> impl IntoElement {
            div()
                .flex_none()
                .w(px(280.0))
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

        fn app_frame(&self) -> impl IntoElement {
            div()
                .size_full()
                .flex()
                .flex_col()
                .text_color(rgb(0x111827))
                .bg(rgb(0xf3f4f6))
                .child(self.app_titlebar())
                .child(self.toolbar())
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .overflow_hidden()
                        .child(self.sidebar())
                        .child(self.canvas_placeholder())
                        .child(self.inspector()),
                )
        }
    }

    impl Render for NotatusWindow {
        fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
                .child(self.app_frame())
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
