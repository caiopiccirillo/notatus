use super::super::helpers::compact_text;
use super::super::*;
use super::{CanvasToolDefinition, canvas_tool_definitions};

impl NotatusWindow {
    pub(in crate::app) fn canvas_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let label_view = view.clone();
        let fit_view = view.clone();

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
            .child(self.active_label_selector(label_view))
            .child(
                Button::new("tool-fit-canvas")
                    .small()
                    .icon(Icon::new(IconName::Maximize))
                    .tooltip("Fit image to canvas")
                    .on_click(move |_, _, cx| {
                        let _ = fit_view.update(cx, |notatus, cx| {
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

    fn active_label_selector(&self, view: gpui::WeakEntity<NotatusWindow>) -> impl IntoElement {
        let labels = self.state.dataset.labels.clone();
        let selected_label = self.selected_label().map(|label| label.id);
        let label_name = self
            .selected_label()
            .map(|label| compact_text(&label.name, 18))
            .unwrap_or_else(|| "Label".to_string());

        Button::new("tool-active-label")
            .small()
            .icon(Icon::new(IconName::Palette))
            .label(label_name)
            .tooltip("Active label")
            .dropdown_menu(move |menu, _, _| {
                if labels.is_empty() {
                    return menu.item(PopupMenuItem::new("Create labels in Dataset"));
                }

                let mut menu = menu;
                for label in labels.clone() {
                    let label_id = label.id;
                    let selected = selected_label == Some(label_id);
                    let view = view.clone();
                    menu = menu.item(
                        PopupMenuItem::new(label.name.clone())
                            .checked(selected)
                            .on_click(move |_, window, cx| {
                                let _ = view.update(cx, |notatus, cx| {
                                    notatus.select_label(label_id, window, cx);
                                });
                            }),
                    );
                }
                menu
            })
    }
}
