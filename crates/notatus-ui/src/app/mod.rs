use super::{AnnotationTool, UiState};
use gpui::prelude::*;
use gpui::{
    App, Bounds, Context, FontWeight, IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ObjectFit, PathPromptOptions, Pixels, Point, Render, ScrollEvent, SharedString, Subscription,
    Window, WindowBackgroundAppearance, WindowBounds, WindowOptions, bounds, div, fill, img,
    outline, px, rgb, size,
};
use gpui_component::{
    ActiveTheme as _, Colorize as _, Icon, IconName, Root, Selectable as _, Sizable as _, Theme,
    ThemeColor, TitleBar, WindowExt,
    button::{Button, ButtonVariant, ButtonVariants as _},
    color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState},
    dialog::DialogButtonProps,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt as _, DropdownMenu, PopupMenu, PopupMenuItem},
    notification::Notification,
    resizable::{h_resizable, resizable_panel},
    scroll::ScrollableElement as _,
    sidebar::{SidebarItem as _, SidebarMenu, SidebarMenuItem},
};
use notatus_core::{
    AnnotationGeometry, AnnotationId, AnnotationRecord, AssetId, AssetKind, AssetLocation,
    AssetRecord, BoundingBox, ClassificationId, ClassificationRecord, Label, LabelId, Polygon,
};
use notatus_storage::{LocalProjectStore, MANIFEST_FILE, ProjectStore};
use std::cell::RefCell;
use std::env;
use std::path::{Path, PathBuf};
use std::rc::Rc;

mod bottom_bar;
mod canvas;
mod commands;
mod export_commands;
mod helpers;
mod left_dock;
mod media_import;
mod project_commands;
mod project_session;
mod right_dock;
mod titlebar;
mod tools;
mod window;

#[cfg(test)]
mod gpui_tests;
#[cfg(test)]
mod screenshot_tests;

use project_session::ProjectLocation;
use tools::ToolInteractionState;
use window::{LeftDock, NotatusWindow, RightDock};

const DEFAULT_LABEL_COLOR: &str = "#2563eb";
const LABEL_COLORS: [&str; 8] = [
    "#2563eb", "#16a34a", "#dc2626", "#d97706", "#7c3aed", "#0891b2", "#db2777", "#4b5563",
];

#[derive(Clone, Copy, Debug, PartialEq)]
struct CanvasImageLayout {
    fit_bounds: Bounds<Pixels>,
    image_bounds: Bounds<Pixels>,
    image_bounds_in_canvas: Bounds<Pixels>,
}

type SharedImageLayout = Rc<RefCell<Option<CanvasImageLayout>>>;

pub fn launch_gpui() {
    gpui_platform::application()
        .with_assets(gpui_kit_assets::Assets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);

            cx.on_window_closed(|cx, _window_id| {
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
                    window_min_size: Some(size(px(720.0), px(460.0))),
                    ..Default::default()
                },
                |window, cx| {
                    Theme::sync_system_appearance(Some(window), cx);
                    let view = cx.new(|cx| {
                        cx.observe_window_appearance(window, |_, window, cx| {
                            Theme::sync_system_appearance(Some(window), cx);
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
