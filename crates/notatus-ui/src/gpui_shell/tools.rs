use super::*;

#[derive(Clone)]
pub(super) struct CanvasToolDefinition {
    pub(super) tool: AnnotationTool,
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) tooltip: &'static str,
    pub(super) icon: IconName,
}

pub(super) fn canvas_tool_definitions() -> [CanvasToolDefinition; 3] {
    [
        CanvasToolDefinition {
            tool: AnnotationTool::DrawBox,
            id: "tool-draw-box",
            label: "Draw Box",
            tooltip: "Draw bounding boxes",
            icon: IconName::Frame,
        },
        CanvasToolDefinition {
            tool: AnnotationTool::Select,
            id: "tool-select",
            label: "Select",
            tooltip: "Select annotations",
            icon: IconName::Inspector,
        },
        CanvasToolDefinition {
            tool: AnnotationTool::Pan,
            id: "tool-pan",
            label: "Pan/Zoom",
            tooltip: "Pan and zoom the canvas",
            icon: IconName::Map,
        },
    ]
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DrawingState {
    pub(super) start_image_pos: (f64, f64),
    pub(super) current_image_pos: (f64, f64),
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CanvasViewport {
    pub(super) zoom: f32,
    pub(super) pan_x: f32,
    pub(super) pan_y: f32,
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

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(super) fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    pub(super) fn zoom_at(
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
pub(super) struct PanState {
    start_screen_pos: Point<Pixels>,
    start_pan_x: f32,
    start_pan_y: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ToolInteractionState {
    pub(super) draw_box: Option<DrawingState>,
    pub(super) pan: Option<PanState>,
    pub(super) viewport: CanvasViewport,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DrawBoxCompletion {
    pub(super) bbox: Option<BoundingBox>,
}

impl ToolInteractionState {
    pub(super) fn clear_for_tool(&mut self, tool: AnnotationTool) {
        if !matches!(tool, AnnotationTool::DrawBox) {
            self.draw_box = None;
        }
        if !matches!(tool, AnnotationTool::Pan) {
            self.pan = None;
        }
    }

    pub(super) fn begin_draw_box(&mut self, image_pos: (f64, f64)) {
        self.draw_box = Some(DrawingState {
            start_image_pos: image_pos,
            current_image_pos: image_pos,
        });
    }

    pub(super) fn update_draw_box(&mut self, image_pos: (f64, f64)) {
        if let Some(ref mut drawing) = self.draw_box {
            drawing.current_image_pos = image_pos;
        }
    }

    pub(super) fn finish_draw_box(&mut self) -> Option<DrawBoxCompletion> {
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

    pub(super) fn begin_pan(&mut self, screen_pos: Point<Pixels>) {
        self.pan = Some(PanState {
            start_screen_pos: screen_pos,
            start_pan_x: self.viewport.pan_x,
            start_pan_y: self.viewport.pan_y,
        });
    }

    pub(super) fn update_pan(&mut self, screen_pos: Point<Pixels>) {
        let Some(pan) = self.pan else {
            return;
        };
        let dx: f32 = (screen_pos.x - pan.start_screen_pos.x).into();
        let dy: f32 = (screen_pos.y - pan.start_screen_pos.y).into();
        self.viewport.pan_x = pan.start_pan_x + dx;
        self.viewport.pan_y = pan.start_pan_y + dy;
    }

    pub(super) fn finish_pan(&mut self) {
        self.pan = None;
    }
}

impl NotatusWindow {
    pub(super) fn set_canvas_tool(&mut self, tool: AnnotationTool, cx: &mut Context<Self>) {
        self.tools.clear_for_tool(tool);
        self.state.set_tool(tool);
        self.status_message = None;
        cx.notify();
    }

    pub(super) fn canvas_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();

        div()
            .absolute()
            .top_3()
            .left_3()
            .flex()
            .items_center()
            .gap_1()
            .rounded_sm()
            .border_1()
            .border_color(rgb(0xd1d5db))
            .bg(rgb(0xffffff))
            .p_1()
            .children(
                canvas_tool_definitions()
                    .into_iter()
                    .map(|definition| self.canvas_tool_button(definition, view.clone())),
            )
    }

    fn canvas_tool_button(
        &self,
        definition: CanvasToolDefinition,
        view: gpui::WeakEntity<NotatusWindow>,
    ) -> Button {
        let tool = definition.tool;

        Button::new(definition.id)
            .small()
            .icon(Icon::new(definition.icon))
            .tooltip(format!("{}: {}", definition.label, definition.tooltip))
            .selected(self.state.active_tool == tool)
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |notatus, cx| {
                    notatus.set_canvas_tool(tool, cx);
                });
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_all_initial_canvas_tools() {
        let tools: Vec<_> = canvas_tool_definitions()
            .into_iter()
            .map(|definition| definition.tool)
            .collect();

        assert_eq!(
            tools,
            vec![
                AnnotationTool::DrawBox,
                AnnotationTool::Select,
                AnnotationTool::Pan
            ]
        );
    }

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
