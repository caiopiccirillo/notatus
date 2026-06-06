use super::super::*;
use super::{CanvasToolDefinition, canvas_tool_definitions};

impl NotatusWindow {
    pub(in crate::app) fn canvas_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(
                Button::new("tool-fit-canvas")
                    .small()
                    .icon(Icon::new(IconName::Maximize))
                    .tooltip("Fit image to canvas")
                    .on_click(move |_, _, cx| {
                        let _ = view.update(cx, |notatus, cx| {
                            notatus.fit_canvas_to_view(cx);
                        });
                    }),
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
