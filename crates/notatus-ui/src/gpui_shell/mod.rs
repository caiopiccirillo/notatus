use super::{AnnotationTool, UiState};
use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, Context, FontWeight, IntoElement, ObjectFit, PathPromptOptions,
    Pixels, Render, SharedString, Subscription, Window, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowOptions, div, img, px, rgb, size,
};
use gpui_component::{
    Icon, IconName, Root, Selectable as _, Sizable as _, TitleBar,
    button::Button,
    input::{Input, InputEvent, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    resizable::{h_resizable, resizable_panel},
    sidebar::{SidebarMenu, SidebarMenuItem},
};
use notatus_core::{
    AnnotationGeometry, AnnotationRecord, AssetKind, AssetLocation, AssetRecord, Label, LabelId,
};
use std::path::{Path, PathBuf};

mod bottom_bar;
mod canvas;
mod helpers;
mod left_dock;
mod media_import;
mod right_dock;
mod titlebar;

use helpers::{annotation_count_label, label_count_label, media_count_label};

const DEFAULT_LABEL_COLOR: &str = "#2563eb";
const LABEL_COLORS: [&str; 8] = [
    "#2563eb", "#16a34a", "#dc2626", "#d97706", "#7c3aed", "#0891b2", "#db2777", "#4b5563",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LeftDock {
    Project,
    Media,
    Labels,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RightDock {
    Annotations,
    MediaInfo,
}

struct NotatusWindow {
    state: UiState,
    left_dock: LeftDock,
    right_dock: RightDock,
    status_message: Option<String>,
    label_name_input: gpui::Entity<InputState>,
    syncing_label_input: bool,
    _subscriptions: Vec<Subscription>,
}

impl NotatusWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut state = UiState::new_project("Untitled dataset");
        state.set_tool(AnnotationTool::DrawBox);
        let label_name_input = cx.new(|cx| InputState::new(window, cx));
        let _subscriptions = vec![cx.subscribe_in(
            &label_name_input,
            window,
            |this, input, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change)
                    && !this.syncing_label_input
                    && let Some(label_id) = this.state.selected_label
                {
                    let value = input.read(cx).value().to_string();
                    match this.state.update_label_name(label_id, value) {
                        Ok(()) => this.status_message = None,
                        Err(error) => this.status_message = Some(error.to_string()),
                    }
                    cx.notify();
                }
            },
        )];

        Self {
            state,
            left_dock: LeftDock::Media,
            right_dock: RightDock::Annotations,
            status_message: None,
            label_name_input,
            syncing_label_input: false,
            _subscriptions,
        }
    }

    fn selected_asset(&self) -> Option<&AssetRecord> {
        self.state
            .selected_asset
            .and_then(|asset_id| self.state.dataset.asset_by_id(asset_id))
    }

    fn selected_label(&self) -> Option<&notatus_core::Label> {
        self.state
            .selected_label
            .and_then(|label_id| self.state.dataset.label_by_id(label_id))
    }

    fn annotations_for_asset(&self, asset: &AssetRecord) -> Vec<&AnnotationRecord> {
        self.state
            .dataset
            .annotations
            .iter()
            .filter(|annotation| annotation.asset_id == asset.id)
            .collect()
    }

    fn project_summary(&self) -> String {
        format!(
            "{} · {} · {}",
            media_count_label(self.state.dataset.assets.len()),
            annotation_count_label(self.state.dataset.annotations.len()),
            label_count_label(self.state.dataset.labels.len())
        )
    }

    fn create_label(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let label_number = self.state.dataset.labels.len() + 1;
        let label_id = self.state.add_label(format!("Label {label_number}"));
        let color = LABEL_COLORS[(label_number - 1) % LABEL_COLORS.len()].to_string();
        self.left_dock = LeftDock::Labels;
        if let Err(error) = self.state.update_label_color(label_id, Some(color)) {
            self.status_message = Some(error.to_string());
        } else {
            self.status_message = Some("Created label".to_string());
        }
        self.sync_label_name_input(window, cx);
        cx.notify();
    }

    fn select_label(&mut self, label_id: LabelId, window: &mut Window, cx: &mut Context<Self>) {
        match self.state.select_label(label_id) {
            Ok(()) => {
                self.left_dock = LeftDock::Labels;
                self.status_message = None;
                self.sync_label_name_input(window, cx);
            }
            Err(error) => self.status_message = Some(error.to_string()),
        }
        cx.notify();
    }

    fn update_selected_label_color(&mut self, color: &'static str, cx: &mut Context<Self>) {
        if let Some(label_id) = self.state.selected_label {
            match self
                .state
                .update_label_color(label_id, Some(color.to_string()))
            {
                Ok(()) => self.status_message = None,
                Err(error) => self.status_message = Some(error.to_string()),
            }
            cx.notify();
        }
    }

    fn sync_label_name_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .selected_label()
            .map(|label| label.name.clone())
            .unwrap_or_default();
        self.syncing_label_input = true;
        self.label_name_input.update(cx, |input, cx| {
            input.set_value(name, window, cx);
        });
        self.syncing_label_input = false;
    }

    fn app_frame(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_color(rgb(0x111827))
            .bg(rgb(0xf3f4f6))
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
                                .child(self.canvas_placeholder()),
                        )
                        .child(
                            resizable_panel()
                                .size(px(304.0))
                                .size_range(px(220.0)..px(420.0))
                                .child(self.right_panel(cx)),
                        ),
                ),
            )
            .child(self.bottom_bar(cx))
    }
}

impl Render for NotatusWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("notatus-window")
            .size_full()
            .bg(rgb(0xf3f4f6))
            .child(self.app_frame(cx))
    }
}

fn requested_window_decorations() -> Option<WindowDecorations> {
    if cfg!(target_os = "linux") {
        Some(WindowDecorations::Client)
    } else {
        None
    }
}

pub fn launch_gpui() {
    Application::new()
        .with_assets(gpui_component_assets::Assets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);

            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1200.0), px(760.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitleBar::title_bar_options()),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: requested_window_decorations(),
                    window_min_size: Some(size(px(720.0), px(460.0))),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| {
                        cx.observe_window_appearance(window, |_, window, _| {
                            window.refresh();
                        })
                        .detach();
                        NotatusWindow::new(window, cx)
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .unwrap();
            cx.activate(true);
        });
}
