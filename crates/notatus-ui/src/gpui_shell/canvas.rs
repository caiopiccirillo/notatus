use super::helpers::*;
use super::*;
use gpui::{MouseDownEvent, MouseMoveEvent, MouseUpEvent, bounds, fill, outline, px};
use notatus_core::AnnotationGeometry;

impl NotatusWindow {
    pub(super) fn canvas_area(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected_asset = self.selected_asset();
        let drawing = self.tools.draw_box;
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
                (
                    ann.geometry.clone(),
                    color.to_string(),
                    self.state.selected_annotation == Some(ann.id),
                )
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
                    .relative()
                    .when_some(selected_asset, |canvas, asset| {
                        canvas.child(interactive_image_canvas(
                            asset,
                            view,
                            drawing,
                            canvas_image_bounds,
                            &state_labels,
                            active_tool,
                            preview_color.clone(),
                            window,
                            cx,
                        ))
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
                    })
                    .child(self.canvas_toolbar(cx)),
            )
    }
}

fn interactive_image_canvas(
    asset: &AssetRecord,
    view: gpui::WeakEntity<NotatusWindow>,
    drawing: Option<super::tools::DrawingState>,
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
    let accepts_drag_events = super::tools::tool_accepts_canvas_drag(active_tool);

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
                            img_bounds, img_width, img_height, min_x, min_y, w, h,
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
        .when(accepts_drag_events, |canvas| {
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
                                    notatus.tools.begin_draw_box((ix, iy));
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
                            if notatus.tools.draw_box.is_some() {
                                if let Some(asset) = notatus.selected_asset() {
                                    let (ix, iy) =
                                        screen_to_image(img_bounds, event.position, asset);
                                    notatus.tools.update_draw_box((ix, iy));
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
                                if let Some(completion) = notatus.tools.finish_draw_box() {
                                    if let Some(asset) = notatus.selected_asset() {
                                        if let Some(label_id) = notatus.state.selected_label {
                                            if let Some(bbox) = completion.bbox {
                                                let _ = img_bounds;
                                                match notatus
                                                    .state
                                                    .add_human_bbox(asset.id, label_id, bbox, None)
                                                {
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
    (
        ix.clamp(0.0, asset.dimensions.width as f64),
        iy.clamp(0.0, asset.dimensions.height as f64),
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{bounds, px, size};
    use notatus_core::{AssetLocation, AssetRecord};

    fn make_test_asset(width: u32, height: u32) -> AssetRecord {
        AssetRecord::new_image(AssetLocation::local("test.jpg"), width, height).unwrap()
    }

    #[test]
    fn screen_to_image_at_origin() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(100.0), px(50.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 0.0);
        assert_eq!(iy, 0.0);
    }

    #[test]
    fn screen_to_image_at_center() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(300.0), px(200.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 400.0);
        assert_eq!(iy, 300.0);
    }

    #[test]
    fn screen_to_image_at_bottom_right() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(500.0), px(350.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 800.0);
        assert_eq!(iy, 600.0);
    }

    #[test]
    fn screen_to_image_clamps_out_of_bounds() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(600.0), px(400.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 800.0);
        assert_eq!(iy, 600.0);
    }

    #[test]
    fn screen_to_image_clamps_negative() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(50.0), px(20.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 0.0);
        assert_eq!(iy, 0.0);
    }

    #[test]
    fn screen_to_image_with_scaling() {
        let img_bounds = bounds(gpui::point(px(0.0), px(0.0)), size(px(200.0), px(150.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(100.0), px(75.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 400.0);
        assert_eq!(iy, 300.0);
    }

    #[test]
    fn image_bbox_to_screen_at_origin() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));

        let screen = image_bbox_to_screen(img_bounds, 800.0, 600.0, 0.0, 0.0, 100.0, 100.0);

        let origin_x: f32 = screen.origin.x.into();
        let origin_y: f32 = screen.origin.y.into();
        let width: f32 = screen.size.width.into();
        let height: f32 = screen.size.height.into();

        assert_eq!(origin_x, 100.0);
        assert_eq!(origin_y, 50.0);
        assert_eq!(width, 50.0);
        assert_eq!(height, 50.0);
    }

    #[test]
    fn image_bbox_to_screen_at_center() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));

        let screen = image_bbox_to_screen(img_bounds, 800.0, 600.0, 400.0, 300.0, 200.0, 200.0);

        let origin_x: f32 = screen.origin.x.into();
        let origin_y: f32 = screen.origin.y.into();
        let width: f32 = screen.size.width.into();
        let height: f32 = screen.size.height.into();

        assert_eq!(origin_x, 300.0);
        assert_eq!(origin_y, 200.0);
        assert_eq!(width, 100.0);
        assert_eq!(height, 100.0);
    }

    #[test]
    fn image_bbox_to_screen_full_image() {
        let img_bounds = bounds(gpui::point(px(0.0), px(0.0)), size(px(400.0), px(300.0)));

        let screen = image_bbox_to_screen(img_bounds, 800.0, 600.0, 0.0, 0.0, 800.0, 600.0);

        let origin_x: f32 = screen.origin.x.into();
        let origin_y: f32 = screen.origin.y.into();
        let width: f32 = screen.size.width.into();
        let height: f32 = screen.size.height.into();

        assert_eq!(origin_x, 0.0);
        assert_eq!(origin_y, 0.0);
        assert_eq!(width, 400.0);
        assert_eq!(height, 300.0);
    }

    #[test]
    fn compute_image_bounds_fits_width() {
        let canvas_bounds = bounds(gpui::point(px(0.0), px(0.0)), size(px(400.0), px(400.0)));

        let img_bounds = compute_image_bounds(canvas_bounds, 800.0, 600.0);

        let origin_x: f32 = img_bounds.origin.x.into();
        let origin_y: f32 = img_bounds.origin.y.into();
        let width: f32 = img_bounds.size.width.into();
        let height: f32 = img_bounds.size.height.into();

        assert_eq!(width, 400.0);
        assert_eq!(height, 300.0);
        assert_eq!(origin_x, 0.0);
        assert_eq!(origin_y, 50.0);
    }

    #[test]
    fn compute_image_bounds_fits_height() {
        let canvas_bounds = bounds(gpui::point(px(0.0), px(0.0)), size(px(600.0), px(300.0)));

        let img_bounds = compute_image_bounds(canvas_bounds, 800.0, 600.0);

        let origin_x: f32 = img_bounds.origin.x.into();
        let origin_y: f32 = img_bounds.origin.y.into();
        let width: f32 = img_bounds.size.width.into();
        let height: f32 = img_bounds.size.height.into();

        assert_eq!(width, 400.0);
        assert_eq!(height, 300.0);
        assert_eq!(origin_x, 100.0);
        assert_eq!(origin_y, 0.0);
    }

    #[test]
    fn compute_image_bounds_exact_fit() {
        let canvas_bounds = bounds(gpui::point(px(0.0), px(0.0)), size(px(800.0), px(600.0)));

        let img_bounds = compute_image_bounds(canvas_bounds, 800.0, 600.0);

        let origin_x: f32 = img_bounds.origin.x.into();
        let origin_y: f32 = img_bounds.origin.y.into();
        let width: f32 = img_bounds.size.width.into();
        let height: f32 = img_bounds.size.height.into();

        assert_eq!(width, 800.0);
        assert_eq!(height, 600.0);
        assert_eq!(origin_x, 0.0);
        assert_eq!(origin_y, 0.0);
    }

    #[test]
    fn hex_to_rgba_with_hash() {
        let color = hex_to_rgba("#ff0000");
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn hex_to_rgba_without_hash() {
        let color = hex_to_rgba("00ff00");
        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 1.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn hex_to_rgba_blue() {
        let color = hex_to_rgba("#0000ff");
        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 1.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn hex_to_rgba_mixed_color() {
        let color = hex_to_rgba("#2563eb");
        assert!((color.r - 0.145).abs() < 0.01);
        assert!((color.g - 0.388).abs() < 0.01);
        assert!((color.b - 0.922).abs() < 0.01);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn hex_to_rgba_invalid_defaults() {
        let color = hex_to_rgba("invalid");
        let default = hex_to_rgba("#2563eb");
        assert_eq!(color.r, default.r);
        assert_eq!(color.g, default.g);
        assert_eq!(color.b, default.b);
    }

    #[test]
    fn rgba_with_alpha_full() {
        let color = rgba_with_alpha("#ff0000", 1.0);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn rgba_with_alpha_half() {
        let color = rgba_with_alpha("#ff0000", 0.5);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert!((color.a - 0.5).abs() < 0.01);
    }

    #[test]
    fn rgba_with_alpha_quarter() {
        let color = rgba_with_alpha("#00ff00", 0.25);
        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 1.0);
        assert_eq!(color.b, 0.0);
        assert!((color.a - 0.25).abs() < 0.01);
    }

    #[test]
    fn rgba_with_alpha_zero() {
        let color = rgba_with_alpha("#0000ff", 0.0);
        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 1.0);
        assert_eq!(color.a, 0.0);
    }

    #[test]
    fn coordinate_conversion_roundtrip() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);

        let screen_pos = gpui::point(px(250.0), px(162.5));
        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        let screen_bbox = image_bbox_to_screen(img_bounds, 800.0, 600.0, ix, iy, 1.0, 1.0);
        let result_x: f32 = screen_bbox.origin.x.into();
        let result_y: f32 = screen_bbox.origin.y.into();

        assert!((result_x - 250.0).abs() < 0.1);
        assert!((result_y - 162.5).abs() < 0.1);
    }
}
