use gpui::prelude::*;
use gpui::{
    App, Bounds, Pixels, WindowBackgroundAppearance, WindowBounds, WindowOptions, px, size,
};
use gpui_component::{Root, Theme, TitleBar};
use std::cell::RefCell;
use std::rc::Rc;
use window::NotatusWindow;

mod actions;
mod assets;
mod autosave;
mod canvas;
mod catalog;
mod dock;
mod edit_commands;
mod export_commands;
mod failures;
mod helpers;
mod label_commands;
mod label_picker;
mod left_dock;
mod media_import;
mod media_navigation;
mod menus;
mod model_run;
mod palette;
mod project_commands;
mod project_session;
mod review_queue;
mod right_dock;
mod runner_settings;
mod shortcuts;
mod status_bar;
mod titlebar;
mod tools;
mod view_options;
mod window;

#[cfg(test)]
mod tests;

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

/// Registers the components, key bindings, and menus. The app and its tests
/// both start here.
fn init(cx: &mut App) {
    gpui_component::init(cx);
    actions::bind_keys(cx);
    menus::install(cx);
}

pub fn launch_gpui() {
    gpui_platform::application()
        .with_assets(assets::AppAssets)
        .run(|cx: &mut App| {
            init(cx);

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
