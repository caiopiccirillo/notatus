use super::helpers::*;
use super::*;
use gpui::{
    MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    bounds, fill, outline, px,
};
use notatus_core::AnnotationGeometry;

impl NotatusWindow {
    pub(super) fn canvas_area(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_asset = self.selected_asset();
        let drawing = self.drawing;
        let canvas_image_bounds = self.canvas_image_bounds.clone();
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
                (ann.geometry.clone(), color.to_string(), self.state.selected_annotation == Some(ann.id))
            })
            .collect();
        let active_tool = self.state.active_tool;
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
                    .when_some(selected_asset, |canvas, asset| {
                        canvas.child(
                            interactive_image_canvas(
                                asset,
                                view,
                                drawing,
                                canvas_image_bounds,
                                &state_labels,
                                active_tool,
                                preview_color.clone(),
                                window,
                                cx,
                            )
                        )
                    })
                    .when(selected_asset.is_none(), |canvas| {
                        canvas
                            .child(div().text_lg().child("Choose images to start"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x4b5563))
                                    .child("No media selected"),
                            )
                    }),
            )
    }
}

fn interactive_image_canvas(
    asset: &AssetRecord,
    view: gpui::WeakEntity<NotatusWindow>,
    drawing: Option<DrawingState>,
    shared_img_bounds: SharedImageBounds,
    annotations: &[(AnnotationGeometry, String, bool)],
    active_tool: AnnotationTool,
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
    let is_drawing_tool = matches!(active_tool, AnnotationTool::DrawBox);

    let bounds_for_prepaint = shared_img_bounds.clone();

    div()
        .id("image-canvas")
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            img(image_path.clone())
                .size_full()
                .object_fit(ObjectFit::Contain)
                .with_loading(|| canvas_message("Loading image").into_any_element())
                .with_fallback(|| {
                    canvas_message("Unable to load selected image").into_any_element()
                }),
        )
        .child(
            gpui::canvas(
                move |bounds, _window, _cx| {
                    let img_bounds = compute_image_bounds(bounds, img_width, img_height);
                    *bounds_for_prepaint.borrow_mut() = Some(img_bounds);
                    img_bounds
                },
                move |_bounds, img_bounds, window, _cx| {
                    for (geometry, color, selected) in &annotations {
                        if let AnnotationGeometry::Bbox(bbox) = geometry {
                            let screen_rect = image_bbox_to_screen(
                                img_bounds,
                                img_width,
                                img_height,
                                bbox.x,
                                bbox.y,
                                bbox.width,
                                bbox.height,
                            );
                            let border_color = hex_to_rgba(color);
                            let bg_color = rgba_with_alpha(color, 0.08);
                            let border_width = if *selected { 3.0 } else { 2.0 };
                            window.paint_quad(fill(screen_rect, bg_color));
                            window.paint_quad(
                                outline(screen_rect, border_color, gpui::BorderStyle::Solid)
                                    .border_widths(gpui::Edges {
                                        top: px(border_width),
                                        right: px(border_width),
                                        bottom: px(border_width),
                                        left: px(border_width),
                                    }),
                            );
                        }
                    }
                    if let Some(drawing) = drawing {
                        let (x1, y1) = drawing.start_image_pos;
                        let (x2, y2) = drawing.current_image_pos;
                        let min_x = x1.min(x2);
                        let min_y = y1.min(y2);
                        let w = (x2 - x1).abs();
                        let h = (y2 - y1).abs();
                        let screen_rect = image_bbox_to_screen(
                            img_bounds,
                            img_width,
                            img_height,
                            min_x,
                            min_y,
                            w,
                            h,
                        );
                        let preview_border = hex_to_rgba(&preview_color);
                        let preview_bg = rgba_with_alpha(&preview_color, 0.08);
                        window.paint_quad(fill(screen_rect, preview_bg));
                        window.paint_quad(
                            outline(screen_rect, preview_border, gpui::BorderStyle::Solid)
                                .border_widths(gpui::Edges {
                                    top: px(2.0),
                                    right: px(2.0),
                                    bottom: px(2.0),
                                    left: px(2.0),
                                }),
                        );
                    }
                },
            )
            .size_full()
            .absolute()
            .top_0()
            .left_0(),
        )
        .when(is_drawing_tool, |canvas| {
            let view_down = view.clone();
            let view_move = view.clone();
            let view_up = view.clone();
            let bounds_down = shared_img_bounds.clone();
            let bounds_move = shared_img_bounds.clone();
            let bounds_up = shared_img_bounds.clone();
            canvas
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    move |event: &MouseDownEvent, _window, cx| {
                        let img_bounds = bounds_down.borrow();
                        if let Some(img_bounds) = *img_bounds {
                            let _ = view_down.update(cx, |notatus, cx| {
                                if let Some(asset) = notatus.selected_asset() {
                                    let (ix, iy) =
                                        screen_to_image(img_bounds, event.position, asset);
                                    notatus.drawing = Some(DrawingState {
                                        start_image_pos: (ix, iy),
                                        current_image_pos: (ix, iy),
                                    });
                                    cx.notify();
                                }
                            });
                        }
                    },
                )
                .on_mouse_move(move |event: &MouseMoveEvent, _window, cx| {
                    let img_bounds = bounds_move.borrow();
                    if let Some(img_bounds) = *img_bounds {
                        let _ = view_move.update(cx, |notatus, cx| {
                            if notatus.drawing.is_some() {
                                if let Some(asset) = notatus.selected_asset() {
                                    let (ix, iy) =
                                        screen_to_image(img_bounds, event.position, asset);
                                    if let Some(ref mut d) = notatus.drawing {
                                        d.current_image_pos = (ix, iy);
                                    }
                                    cx.notify();
                                }
                            }
                        });
                    }
                })
                .on_mouse_up(
                    gpui::MouseButton::Left,
                    move |_event: &MouseUpEvent, _window, cx| {
                        let img_bounds_ref = bounds_up.borrow();
                        if let Some(img_bounds) = *img_bounds_ref {
                            let _ = view_up.update(cx, |notatus, cx| {
                                if let Some(drawing) = notatus.drawing.take() {
                                    if let Some(asset) = notatus.selected_asset() {
                                        if let Some(label_id) = notatus.state.selected_label {
                                            let (x1, y1) = drawing.start_image_pos;
                                            let (x2, y2) = drawing.current_image_pos;
                                            let min_x = x1.min(x2);
                                            let min_y = y1.min(y2);
                                            let w = (x2 - x1).abs();
                                            let h = (y2 - y1).abs();
                                            if w > 1.0 && h > 1.0 {
                                                if let Ok(bbox) = BoundingBox::from_xywh(
                                                    min_x, min_y, w, h,
                                                ) {
                                                    let _ = img_bounds;
                                                    match notatus.state.add_human_bbox(
                                                        asset.id,
                                                        label_id,
                                                        bbox,
                                                        None,
                                                    ) {
                                                        Ok(_) => {
                                                            notatus.status_message =
                                                                Some("Created annotation".into());
                                                        }
                                                        Err(e) => {
                                                            notatus.status_message =
                                                                Some(e.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            notatus.status_message =
                                                Some("Select a label first".into());
                                        }
                                    }
                                }
                                cx.notify();
                            });
                        }
                    },
                )
        })
}

fn compute_image_bounds(
    canvas_bounds: Bounds<Pixels>,
    img_width: f64,
    img_height: f64,
) -> Bounds<Pixels> {
    let canvas_w: f32 = canvas_bounds.size.width.into();
    let canvas_h: f32 = canvas_bounds.size.height.into();
    let scale = (canvas_w / img_width as f32).min(canvas_h / img_height as f32);
    let display_w = img_width as f32 * scale;
    let display_h = img_height as f32 * scale;
    let offset_x = (canvas_w - display_w) / 2.0;
    let offset_y = (canvas_h - display_h) / 2.0;
    bounds(
        gpui::point(
            canvas_bounds.origin.x + px(offset_x),
            canvas_bounds.origin.y + px(offset_y),
        ),
        size(px(display_w), px(display_h)),
    )
}

fn screen_to_image(
    img_bounds: Bounds<Pixels>,
    screen_pos: Point<Pixels>,
    asset: &AssetRecord,
) -> (f64, f64) {
    let rel_x: f32 = (screen_pos.x - img_bounds.origin.x).into();
    let rel_y: f32 = (screen_pos.y - img_bounds.origin.y).into();
    let img_w: f32 = img_bounds.size.width.into();
    let img_h: f32 = img_bounds.size.height.into();
    let scale_x = asset.dimensions.width as f32 / img_w;
    let scale_y = asset.dimensions.height as f32 / img_h;
    let ix = (rel_x * scale_x) as f64;
    let iy = (rel_y * scale_y) as f64;
    (ix.clamp(0.0, asset.dimensions.width as f64), iy.clamp(0.0, asset.dimensions.height as f64))
}

fn image_bbox_to_screen(
    img_bounds: Bounds<Pixels>,
    native_width: f64,
    native_height: f64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Bounds<Pixels> {
    let display_w: f32 = img_bounds.size.width.into();
    let display_h: f32 = img_bounds.size.height.into();
    let scale_x = display_w / native_width as f32;
    let scale_y = display_h / native_height as f32;
    let sx: f32 = img_bounds.origin.x.into();
    let sy: f32 = img_bounds.origin.y.into();
    let screen_x = sx + (x as f32 * scale_x);
    let screen_y = sy + (y as f32 * scale_y);
    let screen_w = w as f32 * scale_x;
    let screen_h = h as f32 * scale_y;
    bounds(
        gpui::point(px(screen_x), px(screen_y)),
        size(px(screen_w), px(screen_h)),
    )
}

fn hex_to_rgba(hex: &str) -> gpui::Rgba {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    let val = u32::from_str_radix(h, 16).unwrap_or(0x2563EB);
    gpui::rgba((val << 8) | 0xFF)
}

fn rgba_with_alpha(hex: &str, alpha: f32) -> gpui::Rgba {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    let val = u32::from_str_radix(h, 16).unwrap_or(0x2563EB);
    let a = (alpha * 255.0) as u32;
    gpui::rgba((val << 8) | a)
}
