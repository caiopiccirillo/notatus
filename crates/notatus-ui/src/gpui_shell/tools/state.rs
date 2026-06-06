use super::super::*;

#[derive(Clone, Copy, Debug)]
pub(in crate::gpui_shell) struct DrawingState {
    pub(in crate::gpui_shell) start_image_pos: (f64, f64),
    pub(in crate::gpui_shell) current_image_pos: (f64, f64),
}

#[derive(Clone, Copy, Debug)]
pub(in crate::gpui_shell) struct CanvasViewport {
    pub(in crate::gpui_shell) zoom: f32,
    pub(in crate::gpui_shell) pan_x: f32,
    pub(in crate::gpui_shell) pan_y: f32,
}

impl Default for CanvasViewport {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
}

impl CanvasViewport {
    const MIN_ZOOM: f32 = 0.25;
    const MAX_ZOOM: f32 = 8.0;

    pub(in crate::gpui_shell) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(in crate::gpui_shell) fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    pub(in crate::gpui_shell) fn zoom_at(
        &mut self,
        screen_pos: Point<Pixels>,
        fit_bounds: Bounds<Pixels>,
        factor: f32,
    ) {
        let old_zoom = self.zoom;
        self.zoom_by(factor);
        let zoom_ratio = self.zoom / old_zoom;
        if (zoom_ratio - 1.0).abs() < f32::EPSILON {
            return;
        }

        let fit_x: f32 = fit_bounds.origin.x.into();
        let fit_y: f32 = fit_bounds.origin.y.into();
        let fit_w: f32 = fit_bounds.size.width.into();
        let fit_h: f32 = fit_bounds.size.height.into();
        let cursor_x: f32 = screen_pos.x.into();
        let cursor_y: f32 = screen_pos.y.into();

        let old_w = fit_w * old_zoom;
        let old_h = fit_h * old_zoom;
        let old_origin_x = fit_x + self.pan_x - (old_w - fit_w) / 2.0;
        let old_origin_y = fit_y + self.pan_y - (old_h - fit_h) / 2.0;
        let old_rel_x = cursor_x - old_origin_x;
        let old_rel_y = cursor_y - old_origin_y;

        let new_w = fit_w * self.zoom;
        let new_h = fit_h * self.zoom;
        let new_origin_x = cursor_x - old_rel_x * zoom_ratio;
        let new_origin_y = cursor_y - old_rel_y * zoom_ratio;

        self.pan_x = new_origin_x - fit_x + (new_w - fit_w) / 2.0;
        self.pan_y = new_origin_y - fit_y + (new_h - fit_h) / 2.0;
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::gpui_shell) struct PanState {
    start_screen_pos: Point<Pixels>,
    start_pan_x: f32,
    start_pan_y: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(in crate::gpui_shell) struct ToolInteractionState {
    pub(in crate::gpui_shell) draw_box: Option<DrawingState>,
    pub(in crate::gpui_shell) pan: Option<PanState>,
    pub(in crate::gpui_shell) viewport: CanvasViewport,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::gpui_shell) struct DrawBoxCompletion {
    pub(in crate::gpui_shell) bbox: Option<BoundingBox>,
}

impl ToolInteractionState {
    pub(in crate::gpui_shell) fn fit_canvas_to_view(&mut self) {
        self.draw_box = None;
        self.pan = None;
        self.viewport.reset();
    }

    pub(in crate::gpui_shell) fn clear_for_tool(&mut self, tool: AnnotationTool) {
        if !matches!(tool, AnnotationTool::DrawBox) {
            self.draw_box = None;
        }
        if !matches!(tool, AnnotationTool::Pan) {
            self.pan = None;
        }
    }

    pub(in crate::gpui_shell) fn begin_draw_box(&mut self, image_pos: (f64, f64)) {
        self.draw_box = Some(DrawingState {
            start_image_pos: image_pos,
            current_image_pos: image_pos,
        });
    }

    pub(in crate::gpui_shell) fn update_draw_box(&mut self, image_pos: (f64, f64)) {
        if let Some(ref mut drawing) = self.draw_box {
            drawing.current_image_pos = image_pos;
        }
    }

    pub(in crate::gpui_shell) fn finish_draw_box(&mut self) -> Option<DrawBoxCompletion> {
        let drawing = self.draw_box.take()?;
        let (x1, y1) = drawing.start_image_pos;
        let (x2, y2) = drawing.current_image_pos;
        let min_x = x1.min(x2);
        let min_y = y1.min(y2);
        let w = (x2 - x1).abs();
        let h = (y2 - y1).abs();
        let bbox = if w > 1.0 && h > 1.0 {
            BoundingBox::from_xywh(min_x, min_y, w, h).ok()
        } else {
            None
        };

        Some(DrawBoxCompletion { bbox })
    }

    pub(in crate::gpui_shell) fn begin_pan(&mut self, screen_pos: Point<Pixels>) {
        self.pan = Some(PanState {
            start_screen_pos: screen_pos,
            start_pan_x: self.viewport.pan_x,
            start_pan_y: self.viewport.pan_y,
        });
    }

    pub(in crate::gpui_shell) fn update_pan(&mut self, screen_pos: Point<Pixels>) {
        let Some(pan) = self.pan else {
            return;
        };
        let dx: f32 = (screen_pos.x - pan.start_screen_pos.x).into();
        let dy: f32 = (screen_pos.y - pan.start_screen_pos.y).into();
        self.viewport.pan_x = pan.start_pan_x + dx;
        self.viewport.pan_y = pan.start_pan_y + dy;
    }

    pub(in crate::gpui_shell) fn finish_pan(&mut self) {
        self.pan = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finishing_draw_box_returns_bbox_for_large_drag() {
        let mut tools = ToolInteractionState::default();

        tools.begin_draw_box((30.0, 40.0));
        tools.update_draw_box((10.0, 20.0));
        let completion = tools.finish_draw_box().unwrap();

        assert_eq!(
            completion.bbox,
            Some(BoundingBox::from_xywh(10.0, 20.0, 20.0, 20.0).unwrap())
        );
        assert!(tools.draw_box.is_none());
    }

    #[test]
    fn finishing_draw_box_ignores_tiny_drag() {
        let mut tools = ToolInteractionState::default();

        tools.begin_draw_box((10.0, 10.0));
        tools.update_draw_box((10.5, 12.0));
        let completion = tools.finish_draw_box().unwrap();

        assert_eq!(completion.bbox, None);
    }

    #[test]
    fn pan_updates_viewport_from_drag_delta() {
        let mut tools = ToolInteractionState::default();

        tools.begin_pan(gpui::point(px(10.0), px(20.0)));
        tools.update_pan(gpui::point(px(25.0), px(5.0)));
        tools.finish_pan();

        assert_eq!(tools.viewport.pan_x, 15.0);
        assert_eq!(tools.viewport.pan_y, -15.0);
        assert!(tools.pan.is_none());
    }

    #[test]
    fn zoom_is_clamped() {
        let mut viewport = CanvasViewport::default();

        viewport.zoom_by(100.0);
        assert_eq!(viewport.zoom, CanvasViewport::MAX_ZOOM);

        viewport.zoom_by(0.001);
        assert_eq!(viewport.zoom, CanvasViewport::MIN_ZOOM);
    }

    #[test]
    fn fit_canvas_to_view_resets_interaction_state() {
        let mut tools = ToolInteractionState::default();
        tools.begin_draw_box((10.0, 10.0));
        tools.begin_pan(gpui::point(px(10.0), px(20.0)));
        tools.viewport.zoom_by(2.0);

        tools.fit_canvas_to_view();

        assert!(tools.draw_box.is_none());
        assert!(tools.pan.is_none());
        assert_eq!(tools.viewport.zoom, 1.0);
        assert_eq!(tools.viewport.pan_x, 0.0);
        assert_eq!(tools.viewport.pan_y, 0.0);
    }

    #[test]
    fn zoom_at_adjusts_pan_to_anchor_cursor() {
        let mut viewport = CanvasViewport::default();
        let fit_bounds = gpui::bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));

        viewport.zoom_at(gpui::point(px(300.0), px(200.0)), fit_bounds, 2.0);

        assert_eq!(viewport.zoom, 2.0);
        assert_eq!(viewport.pan_x, 0.0);
        assert_eq!(viewport.pan_y, 0.0);

        viewport.zoom_at(gpui::point(px(100.0), px(50.0)), fit_bounds, 2.0);

        assert_eq!(viewport.zoom, 4.0);
        assert_eq!(viewport.pan_x, 200.0);
        assert_eq!(viewport.pan_y, 150.0);
    }
}
