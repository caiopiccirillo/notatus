//! The toolbar floating over the canvas: tools, the active label, display
//! options, the model, and fitting the image.

use super::{CanvasToolDefinition, canvas_tool_definitions};
use crate::AnnotationTool;
use crate::app::actions::{self, FitCanvas, RunModel};
use crate::app::window::NotatusWindow;
use gpui::prelude::*;
use gpui::{Action as _, Context, IntoElement, px};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::separator::Separator;
use gpui_component::toolbar::{Toolbar, ToolbarGroup};
use gpui_component::{ActiveTheme as _, Icon, IconName, Selectable as _};

impl NotatusWindow {
    pub(in crate::app) fn canvas_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let separator = || Separator::vertical().h(px(20.0));

        Toolbar::new("canvas-toolbar")
            .absolute()
            .top_3()
            .left_3()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .shadow_md()
            .child(
                ToolbarGroup::new("canvas-tools").label("Tools").children(
                    canvas_tool_definitions()
                        .into_iter()
                        .map(|definition| self.canvas_tool_button(definition, view.clone())),
                ),
            )
            .content(separator())
            .content(self.label_picker())
            .content(separator())
            .content(self.view_options_button(cx))
            .child(
                Button::new("tool-run-model")
                    .ghost()
                    .icon(Icon::new(gpui_kit_assets::IconName::Sparkles))
                    .tooltip_with_action(
                        "Suggest annotations with the model",
                        &RunModel,
                        Some(actions::CONTEXT),
                    )
                    .loading(self.model_run.is_running())
                    .on_click(|_, window, cx| window.dispatch_action(RunModel.boxed_clone(), cx)),
            )
            .child(
                Button::new("tool-fit-canvas")
                    .ghost()
                    .icon(Icon::new(IconName::Maximize))
                    .tooltip_with_action("Fit image", &FitCanvas, Some(actions::CONTEXT))
                    .on_click(|_, window, cx| window.dispatch_action(FitCanvas.boxed_clone(), cx)),
            )
    }

    fn canvas_tool_button(
        &self,
        definition: CanvasToolDefinition,
        view: gpui::WeakEntity<NotatusWindow>,
    ) -> Button {
        let tool = definition.tool;

        Button::new(definition.id)
            .ghost()
            .icon(Icon::new(definition.icon))
            .tooltip_with_action(
                format!("{}: {}", definition.label, definition.tooltip),
                definition.action,
                Some(actions::CONTEXT),
            )
            .selected(self.state.active_tool() == tool)
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |notatus, cx| {
                    notatus.set_canvas_tool(tool, cx);
                });
            })
    }

    pub(in crate::app) fn set_canvas_tool(&mut self, tool: AnnotationTool, cx: &mut Context<Self>) {
        self.tools.clear_for_tool(tool);
        self.canvas_cursor = None;
        self.state.set_tool(tool);
        self.status_message = None;
        cx.notify();
    }
}
