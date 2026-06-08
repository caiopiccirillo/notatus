use super::super::*;

const MIN_BBOX_EXTENT: f64 = 1.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum BboxEditMode {
    Move,
    Resize(ResizeHandle),
}

#[derive(Clone, Copy, Debug)]
pub(in crate::app) struct BboxEditState {
    pub(in crate::app) annotation_id: AnnotationId,
    mode: BboxEditMode,
    start_bbox: BoundingBox,
    start_image_pos: (f64, f64),
}

impl BboxEditState {
    pub(in crate::app) fn new(
        annotation_id: AnnotationId,
        mode: BboxEditMode,
        start_bbox: BoundingBox,
        start_image_pos: (f64, f64),
    ) -> Self {
        Self {
            annotation_id,
            mode,
            start_bbox,
            start_image_pos,
        }
    }

    pub(in crate::app) fn updated_bbox(
        self,
        image_pos: (f64, f64),
        asset: &AssetRecord,
    ) -> Option<BoundingBox> {
        match self.mode {
            BboxEditMode::Move => {
                move_bbox(self.start_bbox, self.start_image_pos, image_pos, asset)
            }
            BboxEditMode::Resize(handle) => resize_bbox(self.start_bbox, handle, image_pos, asset),
        }
    }

    pub(in crate::app) fn cursor_style(self) -> gpui::CursorStyle {
        cursor_for_edit_mode(self.mode)
    }
}

pub(in crate::app) fn cursor_for_edit_mode(mode: BboxEditMode) -> gpui::CursorStyle {
    match mode {
        BboxEditMode::Move => gpui::CursorStyle::ClosedHand,
        BboxEditMode::Resize(handle) => cursor_for_resize_handle(handle),
    }
}

pub(in crate::app) fn cursor_for_resize_handle(handle: ResizeHandle) -> gpui::CursorStyle {
    match handle {
        ResizeHandle::Top | ResizeHandle::Bottom => gpui::CursorStyle::ResizeUpDown,
        ResizeHandle::Left | ResizeHandle::Right => gpui::CursorStyle::ResizeLeftRight,
        ResizeHandle::TopLeft | ResizeHandle::BottomRight => {
            gpui::CursorStyle::ResizeUpLeftDownRight
        }
        ResizeHandle::TopRight | ResizeHandle::BottomLeft => {
            gpui::CursorStyle::ResizeUpRightDownLeft
        }
    }
}

pub(in crate::app) fn move_bbox(
    bbox: BoundingBox,
    start_pos: (f64, f64),
    image_pos: (f64, f64),
    asset: &AssetRecord,
) -> Option<BoundingBox> {
    let dx = image_pos.0 - start_pos.0;
    let dy = image_pos.1 - start_pos.1;
    let max_x = asset.dimensions.width as f64 - bbox.width;
    let max_y = asset.dimensions.height as f64 - bbox.height;
    let x = (bbox.x + dx).clamp(0.0, max_x.max(0.0));
    let y = (bbox.y + dy).clamp(0.0, max_y.max(0.0));
    BoundingBox::from_xywh(x, y, bbox.width, bbox.height).ok()
}

pub(in crate::app) fn resize_bbox(
    bbox: BoundingBox,
    handle: ResizeHandle,
    image_pos: (f64, f64),
    asset: &AssetRecord,
) -> Option<BoundingBox> {
    let image_x = image_pos.0.clamp(0.0, asset.dimensions.width as f64);
    let image_y = image_pos.1.clamp(0.0, asset.dimensions.height as f64);
    let mut left = bbox.x;
    let mut top = bbox.y;
    let mut right = bbox.max_x();
    let mut bottom = bbox.max_y();

    match handle {
        ResizeHandle::TopLeft => {
            left = image_x;
            top = image_y;
        }
        ResizeHandle::Top => top = image_y,
        ResizeHandle::TopRight => {
            right = image_x;
            top = image_y;
        }
        ResizeHandle::Right => right = image_x,
        ResizeHandle::BottomRight => {
            right = image_x;
            bottom = image_y;
        }
        ResizeHandle::Bottom => bottom = image_y,
        ResizeHandle::BottomLeft => {
            left = image_x;
            bottom = image_y;
        }
        ResizeHandle::Left => left = image_x,
    }

    normalized_bbox(
        left,
        top,
        right,
        bottom,
        asset.dimensions.width as f64,
        asset.dimensions.height as f64,
    )
}

fn normalized_bbox(
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    image_width: f64,
    image_height: f64,
) -> Option<BoundingBox> {
    let mut width = (right - left).abs();
    let mut height = (bottom - top).abs();

    width = width.clamp(MIN_BBOX_EXTENT, image_width);
    height = height.clamp(MIN_BBOX_EXTENT, image_height);

    let min_x = left.min(right).clamp(0.0, (image_width - width).max(0.0));
    let min_y = top.min(bottom).clamp(0.0, (image_height - height).max(0.0));

    BoundingBox::from_xywh(min_x, min_y, width, height).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset() -> AssetRecord {
        AssetRecord::new_image(AssetLocation::local("test.png"), 200, 100).unwrap()
    }

    #[test]
    fn corner_resize_changes_two_dimensions() {
        let bbox = BoundingBox::from_xywh(20.0, 20.0, 40.0, 30.0).unwrap();

        let resized = resize_bbox(bbox, ResizeHandle::BottomRight, (80.0, 70.0), &asset()).unwrap();

        assert_eq!(
            resized,
            BoundingBox::from_xywh(20.0, 20.0, 60.0, 50.0).unwrap()
        );
    }

    #[test]
    fn side_resize_changes_one_dimension() {
        let bbox = BoundingBox::from_xywh(20.0, 20.0, 40.0, 30.0).unwrap();

        let resized = resize_bbox(bbox, ResizeHandle::Right, (90.0, 5.0), &asset()).unwrap();

        assert_eq!(
            resized,
            BoundingBox::from_xywh(20.0, 20.0, 70.0, 30.0).unwrap()
        );
    }

    #[test]
    fn resize_allows_flipping_across_opposite_side() {
        let bbox = BoundingBox::from_xywh(20.0, 20.0, 40.0, 30.0).unwrap();

        let resized = resize_bbox(bbox, ResizeHandle::Left, (80.0, 20.0), &asset()).unwrap();

        assert_eq!(
            resized,
            BoundingBox::from_xywh(60.0, 20.0, 20.0, 30.0).unwrap()
        );
    }

    #[test]
    fn moving_bbox_clamps_inside_image_bounds() {
        let bbox = BoundingBox::from_xywh(20.0, 20.0, 40.0, 30.0).unwrap();

        let moved = move_bbox(bbox, (30.0, 30.0), (250.0, 150.0), &asset()).unwrap();

        assert_eq!(
            moved,
            BoundingBox::from_xywh(160.0, 70.0, 40.0, 30.0).unwrap()
        );
    }

    #[test]
    fn collapsed_resize_at_image_edge_keeps_minimum_extent_inside_bounds() {
        let bbox = BoundingBox::from_xywh(150.0, 60.0, 50.0, 40.0).unwrap();

        let resized = resize_bbox(bbox, ResizeHandle::Left, (200.0, 60.0), &asset()).unwrap();

        assert_eq!(
            resized,
            BoundingBox::from_xywh(199.0, 60.0, 1.0, 40.0).unwrap()
        );
    }
}
