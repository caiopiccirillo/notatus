use crate::AnnotationTool;
#[derive(Clone)]
pub(in crate::app) struct CanvasToolDefinition {
    pub(in crate::app) tool: AnnotationTool,
    pub(in crate::app) id: &'static str,
    pub(in crate::app) label: &'static str,
    pub(in crate::app) tooltip: &'static str,
    /// The action that switches to the tool, whose key the tooltip shows.
    pub(in crate::app) action: &'static dyn gpui::Action,
    /// From the full Lucide set, which has shapes that read as each tool.
    pub(in crate::app) icon: gpui_kit_assets::IconName,
}

pub(in crate::app) fn canvas_tool_definitions() -> [CanvasToolDefinition; 3] {
    [
        CanvasToolDefinition {
            tool: AnnotationTool::DrawBox,
            id: "tool-draw-box",
            label: "Box",
            tooltip: "Draw boxes on empty space, drag boxes to edit (alt: always draw)",
            action: &crate::app::actions::BoxTool,
            icon: gpui_kit_assets::IconName::VectorSquare,
        },
        CanvasToolDefinition {
            tool: AnnotationTool::DrawPolygon,
            id: "tool-draw-polygon",
            label: "Polygon",
            tooltip: "Click to add points, double-click to close the shape",
            action: &crate::app::actions::PolygonTool,
            icon: gpui_kit_assets::IconName::Pentagon,
        },
        CanvasToolDefinition {
            tool: AnnotationTool::Select,
            id: "tool-select",
            label: "Select",
            tooltip: "Select, move, and resize without drawing; drag empty space to pan",
            action: &crate::app::actions::SelectTool,
            icon: gpui_kit_assets::IconName::MousePointer2,
        },
    ]
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
                AnnotationTool::DrawPolygon,
                AnnotationTool::Select,
            ]
        );
    }
}
