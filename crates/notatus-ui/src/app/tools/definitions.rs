use super::super::*;

#[derive(Clone)]
pub(in crate::app) struct CanvasToolDefinition {
    pub(in crate::app) tool: AnnotationTool,
    pub(in crate::app) id: &'static str,
    pub(in crate::app) label: &'static str,
    pub(in crate::app) tooltip: &'static str,
    pub(in crate::app) icon: IconName,
}

pub(in crate::app) fn canvas_tool_definitions() -> [CanvasToolDefinition; 4] {
    [
        CanvasToolDefinition {
            tool: AnnotationTool::DrawBox,
            id: "tool-draw-box",
            label: "Draw Box",
            tooltip: "Draw bounding boxes",
            icon: IconName::Frame,
        },
        CanvasToolDefinition {
            tool: AnnotationTool::DrawPolygon,
            id: "tool-draw-polygon",
            label: "Draw Polygon",
            tooltip: "Draw segmentation polygons",
            icon: IconName::Asterisk,
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
                AnnotationTool::Pan
            ]
        );
    }
}
