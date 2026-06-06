use super::super::*;

#[derive(Clone)]
pub(in crate::gpui_shell) struct CanvasToolDefinition {
    pub(in crate::gpui_shell) tool: AnnotationTool,
    pub(in crate::gpui_shell) id: &'static str,
    pub(in crate::gpui_shell) label: &'static str,
    pub(in crate::gpui_shell) tooltip: &'static str,
    pub(in crate::gpui_shell) icon: IconName,
}

pub(in crate::gpui_shell) fn canvas_tool_definitions() -> [CanvasToolDefinition; 3] {
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
}
