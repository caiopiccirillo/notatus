use super::color::{hex_to_rgba, rgba_with_alpha};
use super::hit_test::bbox_handle_points;
use super::layout::image_bbox_to_screen;
use super::*;

const HANDLE_DIAMETER_PX: f32 = 10.0;

#[derive(Clone)]
pub(super) struct AnnotationOverlay {
    pub(super) id: AnnotationId,
    pub(super) geometry: AnnotationGeometry,
    pub(super) color: String,
    pub(super) selected: bool,
    pub(super) hovered: bool,
}

pub(super) fn paint_annotations(
    annotations: &[AnnotationOverlay],
    img_bounds: Bounds<Pixels>,
    img_width: f64,
    img_height: f64,
    window: &mut Window,
) {
    for annotation in annotations {
        if let AnnotationGeometry::Bbox(bbox) = &annotation.geometry {
            let screen_rect = image_bbox_to_screen(
                img_bounds,
                img_width,
                img_height,
                bbox.x,
                bbox.y,
                bbox.width,
                bbox.height,
            );
            let border_color = hex_to_rgba(&annotation.color);
            let bg_alpha = if annotation.selected {
                0.14
            } else if annotation.hovered {
                0.16
            } else {
                0.08
            };
            let bg_color = rgba_with_alpha(&annotation.color, bg_alpha);
            let border_width = if annotation.selected || annotation.hovered {
                3.0
            } else {
                2.0
            };
            window.paint_quad(fill(screen_rect, bg_color));
            window.paint_quad(
                outline(screen_rect, border_color, gpui::BorderStyle::Solid).border_widths(
                    gpui::Edges {
                        top: px(border_width),
                        right: px(border_width),
                        bottom: px(border_width),
                        left: px(border_width),
                    },
                ),
            );

            if annotation.selected {
                paint_bbox_handles(screen_rect, &annotation.color, window);
            }
        }
    }
}

fn paint_bbox_handles(screen_rect: Bounds<Pixels>, color: &str, window: &mut Window) {
    let border_color = hex_to_rgba(color);
    let fill_color = rgb(0xffffff);
    let origin_x: f32 = screen_rect.origin.x.into();
    let origin_y: f32 = screen_rect.origin.y.into();
    let width: f32 = screen_rect.size.width.into();
    let height: f32 = screen_rect.size.height.into();
    let bbox = BoundingBox::from_xywh(
        origin_x as f64,
        origin_y as f64,
        width as f64,
        height as f64,
    )
    .ok();
    let Some(bbox) = bbox else {
        return;
    };

    for (_, (x, y)) in bbox_handle_points(bbox) {
        let handle_bounds = bounds(
            gpui::point(
                px(x as f32 - HANDLE_DIAMETER_PX / 2.0),
                px(y as f32 - HANDLE_DIAMETER_PX / 2.0),
            ),
            size(px(HANDLE_DIAMETER_PX), px(HANDLE_DIAMETER_PX)),
        );
        window.paint_quad(
            fill(handle_bounds, fill_color)
                .corner_radii(px(HANDLE_DIAMETER_PX / 2.0))
                .border_color(border_color)
                .border_widths(px(2.0)),
        );
    }
}

pub(super) fn paint_drawing_preview(
    drawing: super::super::tools::DrawingState,
    img_bounds: Bounds<Pixels>,
    img_width: f64,
    img_height: f64,
    preview_color: &str,
    window: &mut Window,
) {
    let (x1, y1) = drawing.start_image_pos;
    let (x2, y2) = drawing.current_image_pos;
    let min_x = x1.min(x2);
    let min_y = y1.min(y2);
    let w = (x2 - x1).abs();
    let h = (y2 - y1).abs();
    let screen_rect = image_bbox_to_screen(img_bounds, img_width, img_height, min_x, min_y, w, h);
    let preview_border = hex_to_rgba(preview_color);
    let preview_bg = rgba_with_alpha(preview_color, 0.08);
    window.paint_quad(fill(screen_rect, preview_bg));
    window.paint_quad(
        outline(screen_rect, preview_border, gpui::BorderStyle::Solid).border_widths(gpui::Edges {
            top: px(2.0),
            right: px(2.0),
            bottom: px(2.0),
            left: px(2.0),
        }),
    );
}
