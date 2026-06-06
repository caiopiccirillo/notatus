use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ToolAvailability {
    Enabled,
    ComingSoon,
}

impl ToolAvailability {
    pub(super) fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

#[derive(Clone)]
pub(super) struct CanvasToolDefinition {
    pub(super) tool: AnnotationTool,
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) tooltip: &'static str,
    pub(super) icon: IconName,
    pub(super) availability: ToolAvailability,
}

pub(super) fn canvas_tool_definitions() -> [CanvasToolDefinition; 3] {
    [
        CanvasToolDefinition {
            tool: AnnotationTool::DrawBox,
            id: "tool-draw-box",
            label: "Draw Box",
            tooltip: "Draw bounding boxes",
            icon: IconName::Frame,
            availability: ToolAvailability::Enabled,
        },
        CanvasToolDefinition {
            tool: AnnotationTool::Select,
            id: "tool-select",
            label: "Select",
            tooltip: "Select annotations",
            icon: IconName::Inspector,
            availability: ToolAvailability::ComingSoon,
        },
        CanvasToolDefinition {
            tool: AnnotationTool::Pan,
            id: "tool-pan",
            label: "Pan/Zoom",
            tooltip: "Pan and zoom the canvas",
            icon: IconName::Map,
            availability: ToolAvailability::ComingSoon,
        },
    ]
}

pub(super) fn tool_accepts_canvas_drag(tool: AnnotationTool) -> bool {
    canvas_tool_definitions()
        .into_iter()
        .find(|definition| definition.tool == tool)
        .is_some_and(|definition| {
            definition.availability.is_enabled()
                && matches!(definition.tool, AnnotationTool::DrawBox)
        })
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DrawingState {
    pub(super) start_image_pos: (f64, f64),
    pub(super) current_image_pos: (f64, f64),
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ToolInteractionState {
    pub(super) draw_box: Option<DrawingState>,
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
        let enabled = definition.availability.is_enabled();
        let tool = definition.tool;

        Button::new(definition.id)
            .small()
            .icon(Icon::new(definition.icon))
            .tooltip(format!("{}: {}", definition.label, definition.tooltip))
            .selected(self.state.active_tool == tool)
            .disabled(!enabled)
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
    fn draw_box_is_the_only_enabled_canvas_tool() {
        let enabled: Vec<_> = canvas_tool_definitions()
            .into_iter()
            .filter(|definition| definition.availability.is_enabled())
            .map(|definition| definition.tool)
            .collect();

        assert_eq!(enabled, vec![AnnotationTool::DrawBox]);
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
}
