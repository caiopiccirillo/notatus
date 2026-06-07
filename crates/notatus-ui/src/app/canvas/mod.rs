use super::helpers::*;
use super::*;

mod color;
mod hit_test;
mod interaction;
mod layout;
mod overlay;

use interaction::attach_canvas_interactions;
use layout::{apply_viewport_to_bounds, canvas_image_layout, compute_image_bounds};
use overlay::{AnnotationOverlay, paint_annotations, paint_drawing_preview};

impl NotatusWindow {
    pub(super) fn canvas_area(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected_asset = self.selected_asset();
        let drawing = self.tools.draw_box;
        let canvas_image_layout = self.canvas_image_layout.clone();
        let annotations: Vec<_> = selected_asset
            .map(|asset| self.annotations_for_asset(asset))
            .unwrap_or_default();
        let state_labels: Vec<_> = annotations
            .iter()
            .map(|ann| {
                let label = self.state.dataset.label_by_id(ann.label_id);
                let color = label
                    .and_then(|l| l.color.as_deref())
                    .unwrap_or(DEFAULT_LABEL_COLOR);
                AnnotationOverlay {
                    id: ann.id,
                    geometry: ann.geometry.clone(),
                    color: color.to_string(),
                    selected: self.state.selected_annotation == Some(ann.id),
                    hovered: self.hovered_annotation == Some(ann.id),
                }
            })
            .collect();
        let active_tool = self.state.active_tool;
        let viewport = self.tools.viewport;
        let (empty_title, empty_message) = if self.state.dataset.labels.is_empty() {
            ("Create labels to continue", "No labels in this project")
        } else if self.state.dataset.assets.is_empty() {
            ("Import media to continue", "No media in this project")
        } else {
            ("Select media to annotate", "No media selected")
        };
        let preview_color = self
            .selected_label()
            .and_then(|l| l.color.as_deref())
            .unwrap_or(DEFAULT_LABEL_COLOR)
            .to_string();
        let view = cx.weak_entity();

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
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(rgb(0xcbd5e1))
                    .bg(rgb(0xffffff))
                    .overflow_hidden()
                    .relative()
                    .when_some(selected_asset, |canvas, asset| {
                        canvas.child(interactive_image_canvas(
                            asset,
                            view,
                            drawing,
                            canvas_image_layout,
                            &state_labels,
                            active_tool,
                            viewport,
                            preview_color.clone(),
                            window,
                            cx,
                        ))
                    })
                    .when(selected_asset.is_none(), |canvas| {
                        canvas.child(canvas_empty_state(empty_title, empty_message))
                    })
                    .child(self.canvas_toolbar(cx)),
            )
    }
}

fn canvas_empty_state(title: &'static str, message: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .text_center()
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(div().text_sm().text_color(rgb(0x4b5563)).child(message))
}

fn interactive_image_canvas(
    asset: &AssetRecord,
    view: gpui::WeakEntity<NotatusWindow>,
    drawing: Option<super::tools::DrawingState>,
    shared_image_layout: SharedImageLayout,
    annotations: &[AnnotationOverlay],
    active_tool: AnnotationTool,
    viewport: super::tools::CanvasViewport,
    preview_color: String,
    _window: &mut Window,
    _cx: &mut Context<NotatusWindow>,
) -> impl IntoElement {
    let image_path = match &asset.location {
        AssetLocation::LocalPath { path } => PathBuf::from(path),
        AssetLocation::S3Object { .. } => PathBuf::new(),
    };
    let img_width = asset.dimensions.width as f64;
    let img_height = asset.dimensions.height as f64;
    let annotations = annotations.to_vec();
    let annotations_for_paint = annotations.clone();
    let rendered_image_layout = *shared_image_layout.borrow();

    let layout_for_prepaint = shared_image_layout.clone();
    let image = img(image_path.clone())
        .with_loading(|| canvas_message("Loading image").into_any_element())
        .with_fallback(|| canvas_message("Unable to load selected image").into_any_element());

    let canvas = div()
        .id("image-canvas")
        .size_full()
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .child(match rendered_image_layout {
            Some(layout) => image
                .absolute()
                .left(layout.image_bounds_in_canvas.origin.x)
                .top(layout.image_bounds_in_canvas.origin.y)
                .w(layout.image_bounds_in_canvas.size.width)
                .h(layout.image_bounds_in_canvas.size.height)
                .object_fit(ObjectFit::Fill)
                .into_any_element(),
            None => image
                .size_full()
                .object_fit(ObjectFit::Contain)
                .into_any_element(),
        })
        .child(
            gpui::canvas(
                move |bounds, window, cx| {
                    let fit_bounds = compute_image_bounds(bounds, img_width, img_height);
                    let img_bounds = apply_viewport_to_bounds(fit_bounds, viewport);
                    let layout = canvas_image_layout(bounds, fit_bounds, img_bounds);
                    if *layout_for_prepaint.borrow() != Some(layout) {
                        *layout_for_prepaint.borrow_mut() = Some(layout);
                        cx.notify(window.current_view());
                    }
                    img_bounds
                },
                move |_bounds, img_bounds, window, _cx| {
                    paint_annotations(
                        &annotations_for_paint,
                        img_bounds,
                        img_width,
                        img_height,
                        window,
                    );
                    if let Some(drawing) = drawing {
                        paint_drawing_preview(
                            drawing,
                            img_bounds,
                            img_width,
                            img_height,
                            &preview_color,
                            window,
                        );
                    }
                },
            )
            .size_full()
            .absolute()
            .top_0()
            .left_0(),
        );

    attach_canvas_interactions(canvas, active_tool, view, shared_image_layout, annotations)
}
