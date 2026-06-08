use super::overlay::AnnotationOverlay;
use super::*;

const HANDLE_HIT_RADIUS_PX: f64 = 8.0;
const EDGE_HIT_RADIUS_PX: f64 = 6.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BboxHitTarget {
    Body(AnnotationId),
    Handle(AnnotationId, super::super::tools::ResizeHandle),
}

#[cfg(test)]
pub(super) fn hit_test_bbox_annotation(
    annotations: &[AnnotationOverlay],
    image_pos: (f64, f64),
) -> Option<AnnotationId> {
    annotations
        .iter()
        .rev()
        .find_map(|annotation| match &annotation.geometry {
            AnnotationGeometry::Bbox(bbox) => {
                bbox_contains_point(*bbox, image_pos).then_some(annotation.id)
            }
            AnnotationGeometry::Polygon(polygon) => {
                polygon_contains_point(polygon, image_pos).then_some(annotation.id)
            }
        })
}

pub(super) fn hit_test_bbox_edit_target(
    annotations: &[AnnotationOverlay],
    image_pos: (f64, f64),
    img_bounds: Bounds<Pixels>,
    asset: &AssetRecord,
) -> Option<BboxHitTarget> {
    annotations
        .iter()
        .rev()
        .find_map(|annotation| match &annotation.geometry {
            AnnotationGeometry::Bbox(bbox) => {
                if annotation.selected
                    && let Some(handle) = hit_test_bbox_handle(*bbox, image_pos, img_bounds, asset)
                {
                    return Some(BboxHitTarget::Handle(annotation.id, handle));
                }

                bbox_contains_point(*bbox, image_pos).then_some(BboxHitTarget::Body(annotation.id))
            }
            AnnotationGeometry::Polygon(polygon) => polygon_contains_point(polygon, image_pos)
                .then_some(BboxHitTarget::Body(annotation.id)),
        })
}

fn bbox_contains_point(bbox: BoundingBox, (x, y): (f64, f64)) -> bool {
    x >= bbox.x && x <= bbox.max_x() && y >= bbox.y && y <= bbox.max_y()
}

fn polygon_contains_point(polygon: &Polygon, (x, y): (f64, f64)) -> bool {
    if polygon.points.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut previous = polygon.points.len() - 1;
    for current in 0..polygon.points.len() {
        let current_point = polygon.points[current];
        let previous_point = polygon.points[previous];
        let crosses_y = (current_point.y > y) != (previous_point.y > y);
        if crosses_y {
            let edge_x = (previous_point.x - current_point.x) * (y - current_point.y)
                / (previous_point.y - current_point.y)
                + current_point.x;
            if x < edge_x {
                inside = !inside;
            }
        }
        previous = current;
    }

    inside
}

fn hit_test_bbox_handle(
    bbox: BoundingBox,
    image_pos: (f64, f64),
    img_bounds: Bounds<Pixels>,
    asset: &AssetRecord,
) -> Option<super::super::tools::ResizeHandle> {
    let image_radius_x = pixel_radius_to_image_x(HANDLE_HIT_RADIUS_PX, img_bounds, asset);
    let image_radius_y = pixel_radius_to_image_y(HANDLE_HIT_RADIUS_PX, img_bounds, asset);

    for (handle, point) in bbox_handle_points(bbox) {
        if point_near(image_pos, point, image_radius_x, image_radius_y) {
            return Some(handle);
        }
    }

    let edge_radius_x = pixel_radius_to_image_x(EDGE_HIT_RADIUS_PX, img_bounds, asset);
    let edge_radius_y = pixel_radius_to_image_y(EDGE_HIT_RADIUS_PX, img_bounds, asset);
    let (x, y) = image_pos;
    let within_x = x >= bbox.x && x <= bbox.max_x();
    let within_y = y >= bbox.y && y <= bbox.max_y();

    if within_x && (y - bbox.y).abs() <= edge_radius_y {
        Some(super::super::tools::ResizeHandle::Top)
    } else if within_x && (y - bbox.max_y()).abs() <= edge_radius_y {
        Some(super::super::tools::ResizeHandle::Bottom)
    } else if within_y && (x - bbox.x).abs() <= edge_radius_x {
        Some(super::super::tools::ResizeHandle::Left)
    } else if within_y && (x - bbox.max_x()).abs() <= edge_radius_x {
        Some(super::super::tools::ResizeHandle::Right)
    } else {
        None
    }
}

pub(super) fn bbox_handle_points(
    bbox: BoundingBox,
) -> [(super::super::tools::ResizeHandle, (f64, f64)); 8] {
    let mid_x = bbox.x + bbox.width / 2.0;
    let mid_y = bbox.y + bbox.height / 2.0;
    [
        (super::super::tools::ResizeHandle::TopLeft, (bbox.x, bbox.y)),
        (super::super::tools::ResizeHandle::Top, (mid_x, bbox.y)),
        (
            super::super::tools::ResizeHandle::TopRight,
            (bbox.max_x(), bbox.y),
        ),
        (
            super::super::tools::ResizeHandle::Right,
            (bbox.max_x(), mid_y),
        ),
        (
            super::super::tools::ResizeHandle::BottomRight,
            (bbox.max_x(), bbox.max_y()),
        ),
        (
            super::super::tools::ResizeHandle::Bottom,
            (mid_x, bbox.max_y()),
        ),
        (
            super::super::tools::ResizeHandle::BottomLeft,
            (bbox.x, bbox.max_y()),
        ),
        (super::super::tools::ResizeHandle::Left, (bbox.x, mid_y)),
    ]
}

fn point_near(image_pos: (f64, f64), point: (f64, f64), radius_x: f64, radius_y: f64) -> bool {
    (image_pos.0 - point.0).abs() <= radius_x && (image_pos.1 - point.1).abs() <= radius_y
}

fn pixel_radius_to_image_x(radius_px: f64, img_bounds: Bounds<Pixels>, asset: &AssetRecord) -> f64 {
    let display_w: f32 = img_bounds.size.width.into();
    radius_px * asset.dimensions.width as f64 / display_w as f64
}

fn pixel_radius_to_image_y(radius_px: f64, img_bounds: Bounds<Pixels>, asset: &AssetRecord) -> f64 {
    let display_h: f32 = img_bounds.size.height.into();
    radius_px * asset.dimensions.height as f64 / display_h as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{bounds, px, size};

    fn asset() -> AssetRecord {
        AssetRecord::new_image(AssetLocation::local("test.jpg"), 200, 100).unwrap()
    }

    fn img_bounds() -> Bounds<Pixels> {
        bounds(gpui::point(px(0.0), px(0.0)), size(px(200.0), px(100.0)))
    }

    #[test]
    fn hit_test_selects_topmost_bbox_annotation() {
        let bottom_id = AnnotationId::new();
        let top_id = AnnotationId::new();
        let annotations = vec![
            AnnotationOverlay {
                id: bottom_id,
                geometry: AnnotationGeometry::Bbox(
                    BoundingBox::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap(),
                ),
                color: DEFAULT_LABEL_COLOR.to_string(),
                selected: false,
                hovered: false,
            },
            AnnotationOverlay {
                id: top_id,
                geometry: AnnotationGeometry::Bbox(
                    BoundingBox::from_xywh(25.0, 25.0, 100.0, 100.0).unwrap(),
                ),
                color: DEFAULT_LABEL_COLOR.to_string(),
                selected: false,
                hovered: false,
            },
        ];

        assert_eq!(
            hit_test_bbox_annotation(&annotations, (50.0, 50.0)),
            Some(top_id)
        );
        assert_eq!(hit_test_bbox_annotation(&annotations, (200.0, 200.0)), None);
    }

    #[test]
    fn hit_test_selects_polygon_body() {
        let id = AnnotationId::new();
        let annotations = vec![AnnotationOverlay {
            id,
            geometry: AnnotationGeometry::Polygon(
                Polygon::new(vec![
                    notatus_core::Point::new(10.0, 10.0).unwrap(),
                    notatus_core::Point::new(80.0, 10.0).unwrap(),
                    notatus_core::Point::new(80.0, 80.0).unwrap(),
                    notatus_core::Point::new(10.0, 80.0).unwrap(),
                ])
                .unwrap(),
            ),
            color: DEFAULT_LABEL_COLOR.to_string(),
            selected: false,
            hovered: false,
        }];

        assert_eq!(
            hit_test_bbox_edit_target(&annotations, (30.0, 30.0), img_bounds(), &asset()),
            Some(BboxHitTarget::Body(id))
        );
        assert_eq!(
            hit_test_bbox_edit_target(&annotations, (90.0, 90.0), img_bounds(), &asset()),
            None
        );
    }

    #[test]
    fn selected_bbox_handle_wins_over_body() {
        let id = AnnotationId::new();
        let annotations = vec![AnnotationOverlay {
            id,
            geometry: AnnotationGeometry::Bbox(
                BoundingBox::from_xywh(20.0, 20.0, 60.0, 40.0).unwrap(),
            ),
            color: DEFAULT_LABEL_COLOR.to_string(),
            selected: true,
            hovered: false,
        }];

        assert_eq!(
            hit_test_bbox_edit_target(&annotations, (20.0, 20.0), img_bounds(), &asset()),
            Some(BboxHitTarget::Handle(
                id,
                super::super::tools::ResizeHandle::TopLeft
            ))
        );
    }

    #[test]
    fn side_handle_is_detected_near_edge() {
        let id = AnnotationId::new();
        let annotations = vec![AnnotationOverlay {
            id,
            geometry: AnnotationGeometry::Bbox(
                BoundingBox::from_xywh(20.0, 20.0, 60.0, 40.0).unwrap(),
            ),
            color: DEFAULT_LABEL_COLOR.to_string(),
            selected: true,
            hovered: false,
        }];

        assert_eq!(
            hit_test_bbox_edit_target(&annotations, (50.0, 20.0), img_bounds(), &asset()),
            Some(BboxHitTarget::Handle(
                id,
                super::super::tools::ResizeHandle::Top
            ))
        );
    }

    #[test]
    fn bbox_body_is_detected_when_not_on_handle() {
        let id = AnnotationId::new();
        let annotations = vec![AnnotationOverlay {
            id,
            geometry: AnnotationGeometry::Bbox(
                BoundingBox::from_xywh(20.0, 20.0, 60.0, 40.0).unwrap(),
            ),
            color: DEFAULT_LABEL_COLOR.to_string(),
            selected: true,
            hovered: false,
        }];

        assert_eq!(
            hit_test_bbox_edit_target(&annotations, (50.0, 40.0), img_bounds(), &asset()),
            Some(BboxHitTarget::Body(id))
        );
    }
}
