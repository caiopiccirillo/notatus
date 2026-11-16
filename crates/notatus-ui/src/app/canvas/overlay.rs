use super::color::{hex_to_rgba, rgba_with_alpha};
use super::hit_test::bbox_handle_points;
use super::layout::{image_bbox_to_screen, image_point_to_screen};
use crate::app::tools::{DrawingState, PolygonDrawingState};
use gpui::{
    App, Bounds, Pixels, Point, SharedString, Window, bounds, fill, outline, px, rgb, size,
};
use notatus_core::{AnnotationGeometry, AnnotationId, BoundingBox, Polygon};

const HANDLE_DIAMETER_PX: f32 = 10.0;
const POLYGON_VERTEX_DIAMETER_PX: f32 = 8.0;

#[derive(Clone)]
pub(super) struct AnnotationOverlay {
    pub(super) id: AnnotationId,
    pub(super) geometry: AnnotationGeometry,
    pub(super) color: String,
    pub(super) selected: bool,
    pub(super) hovered: bool,
    /// Label name to tag the shape with, when label names are shown.
    pub(super) label: Option<SharedString>,
    /// Drawn faintly, to focus on the selected annotation.
    pub(super) dimmed: bool,
    /// Rejected in review: drawn dashed and faded.
    pub(super) rejected: bool,
}

const DIMMED_ALPHA: f32 = 0.25;
const LABEL_FONT_PX: f32 = 12.0;
const LABEL_PAD_X: f32 = 4.0;
const LABEL_PAD_Y: f32 = 1.0;

impl AnnotationOverlay {
    /// Fill alpha: the base opacity, raised a little when emphasized.
    fn fill_alpha(&self, base: f32) -> f32 {
        let emphasis = if self.selected || self.hovered {
            0.08
        } else {
            0.0
        };
        let alpha = (base + emphasis).min(1.0);
        if self.dimmed {
            alpha * DIMMED_ALPHA
        } else if self.rejected {
            alpha * 0.5
        } else {
            alpha
        }
    }

    fn stroke(&self) -> gpui::Rgba {
        let alpha = if self.dimmed {
            DIMMED_ALPHA
        } else if self.rejected {
            0.7
        } else {
            1.0
        };
        rgba_with_alpha(&self.color, alpha)
    }

    fn border_style(&self) -> gpui::BorderStyle {
        if self.rejected {
            gpui::BorderStyle::Dashed
        } else {
            gpui::BorderStyle::Solid
        }
    }

    fn stroke_width(&self) -> Pixels {
        if self.selected || self.hovered {
            px(3.0)
        } else {
            px(2.0)
        }
    }
}

pub(super) fn paint_annotations(
    annotations: &[AnnotationOverlay],
    img_bounds: Bounds<Pixels>,
    img_width: f64,
    img_height: f64,
    fill_opacity: f32,
    window: &mut Window,
    cx: &mut App,
) {
    for annotation in annotations {
        let anchor = match &annotation.geometry {
            AnnotationGeometry::Bbox(bbox) => paint_bbox_annotation(
                *bbox,
                annotation,
                img_bounds,
                img_width,
                img_height,
                fill_opacity,
                window,
            ),
            AnnotationGeometry::Polygon(polygon) => paint_polygon_annotation(
                polygon,
                annotation,
                img_bounds,
                img_width,
                img_height,
                fill_opacity,
                window,
            ),
        };
        if let (Some(label), Some(anchor)) = (&annotation.label, anchor)
            && !annotation.dimmed
        {
            paint_label_tag(label, &annotation.color, anchor, window, cx);
        }
    }
}

/// Paints the label name in a colored tag sitting on top of `anchor`, the
/// shape's top-left corner, or just inside it when there's no room above.
fn paint_label_tag(
    label: &SharedString,
    color: &str,
    anchor: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let font_size = px(LABEL_FONT_PX);
    let run = gpui::TextRun {
        len: label.len(),
        font: window.text_style().font(),
        color: contrasting_text(color),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line = window
        .text_system()
        .shape_line(label.clone(), font_size, &[run], None);
    let line_height = font_size * 1.3;
    let tag_size = size(
        line.width + px(LABEL_PAD_X * 2.0),
        line_height + px(LABEL_PAD_Y * 2.0),
    );
    let above = anchor.y - tag_size.height;
    // Tuck the tag inside the shape when it would be clipped above it.
    let visible_top = window.content_mask().bounds.origin.y;
    let y = if above >= visible_top {
        above
    } else {
        anchor.y
    };
    let origin = gpui::point(anchor.x, y);
    window.paint_quad(fill(bounds(origin, tag_size), hex_to_rgba(color)).corner_radii(px(2.0)));
    let _ = line.paint(
        origin + gpui::point(px(LABEL_PAD_X), px(LABEL_PAD_Y)),
        line_height,
        gpui::TextAlign::Left,
        None,
        window,
        cx,
    );
}

/// Black or white, whichever reads better on the label color.
fn contrasting_text(hex: &str) -> gpui::Hsla {
    let color = hex_to_rgba(hex);
    let luminance = 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;
    if luminance > 0.6 {
        gpui::black()
    } else {
        gpui::white()
    }
}

/// Thin guide lines through the pointer, across the whole image.
pub(super) fn paint_crosshair(
    (x, y): (f64, f64),
    img_bounds: Bounds<Pixels>,
    img_width: f64,
    img_height: f64,
    window: &mut Window,
) {
    if x < 0.0 || y < 0.0 || x > img_width || y > img_height {
        return;
    }
    let point = image_point_to_screen(img_bounds, img_width, img_height, x, y);
    let color = gpui::rgba(0xffffffb3);
    let shadow = gpui::rgba(0x00000066);
    let vertical = |offset: f32| {
        bounds(
            gpui::point(point.x + px(offset), img_bounds.origin.y),
            size(px(1.0), img_bounds.size.height),
        )
    };
    let horizontal = |offset: f32| {
        bounds(
            gpui::point(img_bounds.origin.x, point.y + px(offset)),
            size(img_bounds.size.width, px(1.0)),
        )
    };
    window.paint_quad(fill(vertical(1.0), shadow));
    window.paint_quad(fill(horizontal(1.0), shadow));
    window.paint_quad(fill(vertical(0.0), color));
    window.paint_quad(fill(horizontal(0.0), color));
}

fn paint_bbox_annotation(
    bbox: BoundingBox,
    annotation: &AnnotationOverlay,
    img_bounds: Bounds<Pixels>,
    img_width: f64,
    img_height: f64,
    fill_opacity: f32,
    window: &mut Window,
) -> Option<Point<Pixels>> {
    let screen_rect = image_bbox_to_screen(
        img_bounds,
        img_width,
        img_height,
        bbox.x,
        bbox.y,
        bbox.width,
        bbox.height,
    );
    let bg_color = rgba_with_alpha(&annotation.color, annotation.fill_alpha(fill_opacity));
    window.paint_quad(fill(screen_rect, bg_color));
    window.paint_quad(
        outline(screen_rect, annotation.stroke(), annotation.border_style())
            .border_widths(annotation.stroke_width()),
    );

    if annotation.selected {
        paint_bbox_handles(screen_rect, &annotation.color, window);
    }
    Some(screen_rect.origin)
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

fn paint_polygon_annotation(
    polygon: &Polygon,
    annotation: &AnnotationOverlay,
    img_bounds: Bounds<Pixels>,
    img_width: f64,
    img_height: f64,
    fill_opacity: f32,
    window: &mut Window,
) -> Option<Point<Pixels>> {
    let points = polygon_screen_points(polygon, img_bounds, img_width, img_height);
    if points.len() < 3 {
        return None;
    }

    paint_closed_polygon_path(
        &points,
        rgba_with_alpha(&annotation.color, annotation.fill_alpha(fill_opacity)),
        window,
    );
    paint_polyline_path(
        &points,
        true,
        annotation.stroke_width(),
        annotation.stroke(),
        window,
    );

    if annotation.selected {
        paint_polygon_vertices(&points, &annotation.color, window);
    }
    // Tag the topmost vertex, leftmost on ties.
    points.iter().copied().min_by(|a, b| {
        a.y.partial_cmp(&b.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    })
}

fn polygon_screen_points(
    polygon: &Polygon,
    img_bounds: Bounds<Pixels>,
    img_width: f64,
    img_height: f64,
) -> Vec<Point<Pixels>> {
    polygon
        .points
        .iter()
        .map(|point| image_point_to_screen(img_bounds, img_width, img_height, point.x, point.y))
        .collect()
}

fn paint_closed_polygon_path(points: &[Point<Pixels>], color: gpui::Rgba, window: &mut Window) {
    let mut path = gpui::PathBuilder::fill();
    path.move_to(points[0]);
    for point in &points[1..] {
        path.line_to(*point);
    }
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

fn paint_polyline_path(
    points: &[Point<Pixels>],
    closed: bool,
    width: Pixels,
    color: gpui::Rgba,
    window: &mut Window,
) {
    if points.len() < 2 {
        return;
    }

    let mut path = gpui::PathBuilder::stroke(width);
    path.move_to(points[0]);
    for point in &points[1..] {
        path.line_to(*point);
    }
    if closed {
        path.close();
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

fn paint_polygon_vertices(points: &[Point<Pixels>], color: &str, window: &mut Window) {
    let border_color = hex_to_rgba(color);
    let fill_color = rgb(0xffffff);
    for point in points {
        let x: f32 = point.x.into();
        let y: f32 = point.y.into();
        let vertex_bounds = bounds(
            gpui::point(
                px(x - POLYGON_VERTEX_DIAMETER_PX / 2.0),
                px(y - POLYGON_VERTEX_DIAMETER_PX / 2.0),
            ),
            size(
                px(POLYGON_VERTEX_DIAMETER_PX),
                px(POLYGON_VERTEX_DIAMETER_PX),
            ),
        );
        window.paint_quad(
            fill(vertex_bounds, fill_color)
                .corner_radii(px(POLYGON_VERTEX_DIAMETER_PX / 2.0))
                .border_color(border_color)
                .border_widths(px(2.0)),
        );
    }
}

pub(super) fn paint_drawing_preview(
    drawing: DrawingState,
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

pub(super) fn paint_polygon_preview(
    drawing: &PolygonDrawingState,
    img_bounds: Bounds<Pixels>,
    img_width: f64,
    img_height: f64,
    preview_color: &str,
    window: &mut Window,
) {
    let mut points: Vec<_> = drawing
        .points
        .iter()
        .map(|(x, y)| image_point_to_screen(img_bounds, img_width, img_height, *x, *y))
        .collect();
    if let Some((x, y)) = drawing.current_image_pos
        && drawing.points.last().copied() != Some((x, y))
    {
        points.push(image_point_to_screen(
            img_bounds, img_width, img_height, x, y,
        ));
    }

    if points.len() >= 2 {
        paint_polyline_path(&points, false, px(2.0), hex_to_rgba(preview_color), window);
    }
    paint_polygon_vertices(&points, preview_color, window);
}
