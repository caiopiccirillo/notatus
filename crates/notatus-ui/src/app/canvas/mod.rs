use super::helpers::*;
use super::*;

mod color;
mod hit_test;
mod interaction;
mod layout;
mod overlay;

use interaction::attach_canvas_interactions;
use layout::{apply_viewport_to_bounds, canvas_image_layout, compute_image_bounds};
use overlay::{
    AnnotationOverlay, paint_annotations, paint_crosshair, paint_drawing_preview,
    paint_polygon_preview,
};

impl NotatusWindow {
    pub(super) fn canvas_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_asset = self.selected_asset();
        let drawing = self.tools.draw_box;
        let polygon_drawing = self.tools.draw_polygon.clone();
        let canvas_image_layout = self.canvas_image_layout.clone();
        let annotations: Vec<_> = selected_asset
            .map(|asset| self.annotations_for_asset(asset))
            .unwrap_or_default();
        let options = self.view_options;
        let state_labels: Vec<_> = annotations
            .iter()
            .filter(|_| !options.hide_annotations)
            .map(|ann| {
                let label = self.state.dataset.label_by_id(ann.label_id);
                let color = label
                    .and_then(|l| l.color.as_deref())
                    .unwrap_or(DEFAULT_LABEL_COLOR);
                let selected = self.state.selected_annotation == Some(ann.id);
                AnnotationOverlay {
                    id: ann.id,
                    geometry: ann.geometry.clone(),
                    color: color.to_string(),
                    selected,
                    hovered: self.hovered_annotation == Some(ann.id),
                    label: options
                        .show_labels
                        .then(|| label.map(|l| review_label(&l.name, ann.review_state)))
                        .flatten(),
                    dimmed: options.focus_selected
                        && self.state.selected_annotation.is_some()
                        && !selected,
                    rejected: ann.review_state == notatus_core::ReviewState::Rejected,
                }
            })
            .collect();
        let active_tool = self.state.active_tool;
        let viewport = self.tools.viewport;
        let canvas_cursor = self.canvas_cursor;
        let has_media = !self.state.dataset.assets.is_empty();
        let preview_color = self
            .selected_label()
            .and_then(|l| l.color.as_deref())
            .unwrap_or(DEFAULT_LABEL_COLOR)
            .to_string();
        let view = cx.weak_entity();
        let colors = cx.theme().colors;

        div()
            .size_full()
            .p_6()
            .flex()
            .items_center()
            .justify_center()
            .bg(colors.background)
            .child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.muted)
                    .overflow_hidden()
                    .relative()
                    .when_some(selected_asset, |canvas, asset| {
                        canvas.child(interactive_image_canvas(InteractiveImageCanvas {
                            asset,
                            view,
                            drawing,
                            polygon_drawing,
                            shared_image_layout: canvas_image_layout,
                            annotations: &state_labels,
                            active_tool,
                            viewport,
                            canvas_cursor,
                            preview_color: preview_color.clone(),
                            options,
                            crosshair: (options.crosshair
                                && matches!(
                                    active_tool,
                                    AnnotationTool::DrawBox | AnnotationTool::DrawPolygon
                                ))
                            .then_some(self.tools.pointer_image_pos)
                            .flatten(),
                        }))
                    })
                    .when(selected_asset.is_none(), |canvas| {
                        canvas.child(canvas_empty_state(has_media))
                    })
                    .child(self.canvas_toolbar(cx)),
            )
    }
}

/// Label tag text, marked with the review outcome once decided.
fn review_label(name: &str, state: notatus_core::ReviewState) -> SharedString {
    use notatus_core::ReviewState;
    match state {
        ReviewState::Accepted => format!("{name} ✓").into(),
        ReviewState::Rejected => format!("{name} ✗").into(),
        ReviewState::Draft | ReviewState::Reviewed => SharedString::from(name.to_string()),
    }
}

/// Guides a new project to its first image, with the keys that do the same.
fn canvas_empty_state(has_media: bool) -> impl IntoElement {
    use super::actions::{CommandPalette, ImportFolder, ImportMedia, OpenProject};
    use gpui::Action as _;
    use gpui_component::empty::{
        Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant,
        EmptyTitle,
    };

    let (title, description) = if has_media {
        (
            "Select media to annotate",
            "Pick an image from the Media list, or press D to start with the first one.",
        )
    } else {
        (
            "Import images to start annotating",
            "Labels can be created while you draw. Press Ctrl+K at any time for every command.",
        )
    };
    let action_button = |id: &'static str, label: &'static str, action: Box<dyn gpui::Action>| {
        Button::new(id)
            .small()
            .label(label)
            .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
    };

    Empty::new()
        .header(
            EmptyHeader::new()
                .media(
                    EmptyMedia::new()
                        .with_variant(EmptyMediaVariant::Icon)
                        .child(Icon::new(IconName::GalleryVerticalEnd)),
                )
                .title(EmptyTitle::new().child(title))
                .description(EmptyDescription::new().child(description)),
        )
        .content(
            EmptyContent::new().child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        action_button("empty-import", "Import images", ImportMedia.boxed_clone())
                            .primary(),
                    )
                    .child(action_button(
                        "empty-import-folder",
                        "Import folder",
                        ImportFolder.boxed_clone(),
                    ))
                    .when(!has_media, |row| {
                        row.child(action_button(
                            "empty-open",
                            "Open project",
                            OpenProject.boxed_clone(),
                        ))
                    })
                    .child(action_button(
                        "empty-palette",
                        "Commands",
                        CommandPalette.boxed_clone(),
                    )),
            ),
        )
}

struct InteractiveImageCanvas<'a> {
    asset: &'a AssetRecord,
    view: gpui::WeakEntity<NotatusWindow>,
    drawing: Option<super::tools::DrawingState>,
    polygon_drawing: Option<super::tools::PolygonDrawingState>,
    shared_image_layout: SharedImageLayout,
    annotations: &'a [AnnotationOverlay],
    active_tool: AnnotationTool,
    viewport: super::tools::CanvasViewport,
    canvas_cursor: Option<gpui::CursorStyle>,
    preview_color: String,
    options: super::view_options::CanvasViewOptions,
    crosshair: Option<(f64, f64)>,
}

fn interactive_image_canvas(args: InteractiveImageCanvas<'_>) -> impl IntoElement {
    let InteractiveImageCanvas {
        asset,
        view,
        drawing,
        polygon_drawing,
        shared_image_layout,
        annotations,
        active_tool,
        viewport,
        canvas_cursor,
        preview_color,
        options,
        crosshair,
    } = args;

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
        .when_some(canvas_cursor, |canvas, cursor| canvas.cursor(cursor))
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
                move |_bounds, img_bounds, window, cx| {
                    paint_annotations(
                        &annotations_for_paint,
                        img_bounds,
                        img_width,
                        img_height,
                        options.fill_opacity,
                        window,
                        cx,
                    );
                    if let Some(pos) = crosshair {
                        paint_crosshair(pos, img_bounds, img_width, img_height, window);
                    }
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
                    if let Some(polygon_drawing) = &polygon_drawing {
                        paint_polygon_preview(
                            polygon_drawing,
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
