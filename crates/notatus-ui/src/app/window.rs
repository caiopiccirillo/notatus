use crate::app::autosave::AutosaveState;
use crate::app::catalog::CommandStatus;
use crate::app::export_commands::ExportOptions;
use crate::app::helpers::hex_color;
use crate::app::label_picker::LabelPickerState;
use crate::app::left_dock::MediaListState;
use crate::app::menus;
use crate::app::model_run::ModelRunState;
use crate::app::project_session::ProjectLocation;
use crate::app::runner_settings::RunnerSettings;
use crate::app::tools::ToolInteractionState;
use crate::app::view_options::CanvasViewOptions;
use crate::app::{DEFAULT_LABEL_COLOR, SharedImageLayout};
use crate::{AnnotationTool, UiState};
use gpui::prelude::*;
use gpui::{
    AnyWindowHandle, Context, CursorStyle, Entity, FocusHandle, IntoElement, Pixels, Render,
    Subscription, Window, div, px,
};
use gpui_component::color_picker::{ColorPickerEvent, ColorPickerState};
use gpui_component::command::CommandState;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::menu::AppMenuBar;
use gpui_component::resizable::{h_resizable, resizable_panel};
use gpui_component::slider::SliderState;
use gpui_component::{ActiveTheme as _, Colorize as _};
use notatus_core::{AnnotationId, AssetId, AssetRecord, Label, LabelId};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LeftDock {
    Media,
    Labels,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RightDock {
    Annotations,
    Details,
}

pub(super) struct NotatusWindow {
    pub(super) state: UiState,
    pub(super) left_dock: LeftDock,
    pub(super) right_dock: RightDock,
    pub(super) status_message: Option<String>,
    pub(super) project_location: ProjectLocation,
    pub(super) project_name_input: Entity<InputState>,
    pub(super) label_name_input: Entity<InputState>,
    pub(super) label_color_picker: Entity<ColorPickerState>,
    pub(super) label_color_target: Option<LabelId>,
    pub(super) tools: ToolInteractionState,
    pub(super) hovered_annotation: Option<AnnotationId>,
    pub(super) canvas_cursor: Option<CursorStyle>,
    pub(super) export: ExportOptions,
    pub(super) canvas_image_layout: SharedImageLayout,
    pub(super) focus_handle: FocusHandle,
    pub(super) command_state: Entity<CommandState>,
    pub(super) autosave: AutosaveState,
    pub(super) window_handle: AnyWindowHandle,
    pub(super) model_run: ModelRunState,
    pub(super) runner_settings: RunnerSettings,
    pub(super) label_picker: Entity<LabelPickerState>,
    pub(super) label_picker_synced: Option<(u64, Option<LabelId>)>,
    /// The revision and label the name fields last showed.
    pub(super) name_inputs_synced: Option<(u64, Option<LabelId>)>,
    pub(super) media_list: Entity<MediaListState>,
    /// The revision and media the list last showed.
    pub(super) media_list_synced: Option<(u64, Option<AssetId>)>,
    pub(super) view_options: CanvasViewOptions,
    pub(super) opacity_slider: Entity<SliderState>,
    /// Each command's state when the menus were last built.
    pub(super) menus_synced: Option<Vec<CommandStatus>>,
    /// The menus drawn in the title bar, where the platform has no menu bar.
    pub(super) app_menu_bar: Option<Entity<AppMenuBar>>,
    pub(super) _subscriptions: Vec<Subscription>,
}

impl NotatusWindow {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut state = UiState::new_project("Untitled project");
        state.set_tool(AnnotationTool::DrawBox);
        let project_name_input = cx.new(|cx| InputState::new(window, cx));
        let label_name_input = cx.new(|cx| InputState::new(window, cx));
        let label_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(hex_color(DEFAULT_LABEL_COLOR))
        });
        let command_state = cx.new(|cx| CommandState::new(window, cx));
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);
        let (label_picker, label_picker_subscription) = Self::new_label_picker(window, cx);
        let (opacity_slider, opacity_subscription) = Self::new_opacity_slider(window, cx);
        let (media_list, media_list_subscription) = Self::new_media_list(window, cx);
        project_name_input.update(cx, |input, cx| {
            input.set_value(state.dataset().manifest.project.name.clone(), window, cx);
        });
        let close_view = cx.weak_entity();
        window.on_window_should_close(cx, move |window, cx| {
            close_view
                .update(cx, |notatus, cx| notatus.should_close(window, cx))
                .unwrap_or(true)
        });
        let _subscriptions = vec![
            label_picker_subscription,
            opacity_subscription,
            media_list_subscription,
            // Every state change notifies, so this sees each edit.
            cx.observe_self(|this, cx| this.schedule_autosave(cx)),
            cx.subscribe_in(
                &project_name_input,
                window,
                |this, input, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        let value = input.read(cx).value().to_string();
                        match this.state.rename_project(value) {
                            Ok(()) => this.status_message = None,
                            Err(error) => this.status_message = Some(error.to_string()),
                        }
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(
                &label_name_input,
                window,
                |this, input, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change)
                        && let Some(label_id) = this.state.selected_label()
                    {
                        let value = input.read(cx).value().to_string();
                        match this.state.update_label_name(label_id, value) {
                            Ok(()) => this.status_message = None,
                            Err(error) => this.status_message = Some(error.to_string()),
                        }
                        cx.notify();
                    }
                },
            ),
            cx.subscribe_in(
                &label_color_picker,
                window,
                |this, _, event: &ColorPickerEvent, _window, cx| {
                    let ColorPickerEvent::Change(Some(color)) = event else {
                        return;
                    };

                    let Some(label_id) = this.label_color_target else {
                        return;
                    };

                    match this
                        .state
                        .update_label_color(label_id, Some(color.to_hex()))
                    {
                        Ok(()) => this.status_message = None,
                        Err(error) => this.status_message = Some(error.to_string()),
                    }
                    cx.notify();
                },
            ),
        ];

        Self {
            state,
            left_dock: LeftDock::Media,
            // The current media's annotations are what annotating needs;
            // project details stay one click away.
            right_dock: RightDock::Annotations,
            status_message: None,
            project_location: ProjectLocation::default(),
            project_name_input,
            label_name_input,
            label_color_picker,
            label_color_target: None,
            tools: ToolInteractionState::default(),
            hovered_annotation: None,
            canvas_cursor: None,
            export: Default::default(),
            canvas_image_layout: Rc::new(RefCell::new(None)),
            focus_handle,
            command_state,
            autosave: AutosaveState::new(),
            window_handle: window.window_handle(),
            model_run: Default::default(),
            runner_settings: RunnerSettings::for_user(),
            label_picker,
            label_picker_synced: None,
            name_inputs_synced: None,
            media_list,
            media_list_synced: None,
            view_options: Default::default(),
            opacity_slider,
            menus_synced: None,
            app_menu_bar: menus::IN_TITLE_BAR.then(|| AppMenuBar::new(cx)),
            _subscriptions,
        }
    }

    pub(super) fn selected_asset(&self) -> Option<&AssetRecord> {
        self.state
            .selected_asset()
            .and_then(|asset_id| self.state.dataset().asset_by_id(asset_id))
    }

    pub(super) fn selected_label(&self) -> Option<&Label> {
        self.state
            .selected_label()
            .and_then(|label_id| self.state.dataset().label_by_id(label_id))
    }

    fn app_frame(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_color(colors.foreground)
            .bg(colors.background)
            .child(self.app_titlebar(cx))
            .child(
                div().flex_1().overflow_hidden().child(
                    h_resizable("notatus-annotation-panels")
                        .child(
                            resizable_panel()
                                .size(px(304.0))
                                .size_range(px(228.0)..px(420.0))
                                .child(self.left_panel(cx)),
                        )
                        .child(
                            resizable_panel()
                                .size_range(px(320.0)..Pixels::MAX)
                                .child(self.canvas_area(cx)),
                        )
                        .child(
                            resizable_panel()
                                .size(px(304.0))
                                .size_range(px(220.0)..px(420.0))
                                .child(self.right_panel(cx)),
                        ),
                ),
            )
            .child(self.status_bar(cx))
    }

    /// Keeps the project and label name fields showing the names they edit,
    /// after undo, opening a project, or choosing another label. Runs when
    /// the dataset or the active label changed, and rewrites a field only when
    /// it shows something else, so typing is never interrupted and a name
    /// rejected mid-edit (an empty one) stays in the field.
    pub(super) fn sync_name_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = (self.state.revision(), self.state.selected_label());
        if self.name_inputs_synced == Some(key) {
            return;
        }
        self.name_inputs_synced = Some(key);
        let project = self.state.dataset().manifest.project.name.clone();
        let label = self
            .selected_label()
            .map(|label| label.name.clone())
            .unwrap_or_default();
        for (input, name) in [
            (self.project_name_input.clone(), project),
            (self.label_name_input.clone(), label),
        ] {
            input.update(cx, |input, cx| {
                // set_value emits no change event, so this edits nothing.
                if input.value() != name.as_str() {
                    input.set_value(name, window, cx);
                }
            });
        }
    }
}

impl Render for NotatusWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_name_inputs(window, cx);
        self.sync_label_picker(window, cx);
        self.sync_media_list(window, cx);
        self.sync_menus(cx);
        // gpui-component's Root mounts the dialog and notification layers itself.
        let root = div()
            .id("notatus-window")
            .relative()
            .size_full()
            .bg(cx.theme().background);
        self.action_handlers(root, cx).child(self.app_frame(cx))
    }
}
