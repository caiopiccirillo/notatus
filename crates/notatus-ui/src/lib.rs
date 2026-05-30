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
        App, Application, Bounds, Context, FontWeight, IntoElement, Render, Window, WindowBounds,
        WindowOptions, div, px, rgb, size,
    };

    struct NotatusWindow {
        state: UiState,
    }

    impl NotatusWindow {
        fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
            let mut state = UiState::new_project("Untitled dataset");
            state.set_tool(AnnotationTool::DrawBox);
            Self { state }
        }

        fn toolbar(&self) -> impl IntoElement {
            div()
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
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(0xf3f4f6))
                .child(
                    div()
                        .w(px(560.0))
                        .h(px(360.0))
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
    }

    impl Render for NotatusWindow {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .flex()
                .flex_col()
                .text_color(rgb(0x111827))
                .bg(rgb(0xf3f4f6))
                .child(self.toolbar())
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .child(self.sidebar())
                        .child(self.canvas_placeholder())
                        .child(self.inspector()),
                )
        }
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

    pub fn launch_gpui() {
        Application::new().run(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(1200.0), px(760.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| NotatusWindow::new(window, cx)),
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
