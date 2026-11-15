//! Offscreen screenshots of the main window, for visual review.
//!
//! Ignored by default because they need a GPU adapter. Run with:
//!
//! ```sh
//! NOTATUS_SCREENSHOT_DIR=/tmp/shots NOTATUS_SAMPLE_IMAGE=/path/to/image.png \
//!     cargo test -p notatus-ui screenshots -- --ignored --nocapture
//! ```

use crate::app::*;
use gpui::{AnyWindowHandle, HeadlessAppContext};
use gpui_component::ThemeMode;
use std::sync::Arc;

fn headless_cx() -> HeadlessAppContext {
    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new("Noto Sans"));
    let cx = HeadlessAppContext::with_platform(
        text_system,
        Arc::new(crate::app::assets::AppAssets),
        gpui_platform::current_headless_renderer,
    );
    cx.allow_parking();
    cx
}

fn seed_project(notatus: &mut NotatusWindow, sample_image: Option<&str>) {
    let person = notatus.state.add_label("Person");
    let car = notatus.state.add_label("Car");
    let _ = notatus
        .state
        .update_label_color(person, Some(LABEL_COLORS[0].into()));
    let _ = notatus
        .state
        .update_label_color(car, Some(LABEL_COLORS[1].into()));
    let _ = notatus.state.select_label(person);

    let Some(path) = sample_image else {
        return;
    };
    let Ok((width, height)) = image::image_dimensions(path) else {
        return;
    };
    let asset_id = notatus
        .state
        .add_local_image_asset(path.to_string(), width, height)
        .unwrap();
    let _ = notatus
        .state
        .add_local_image_asset("/tmp/second.png".to_string(), 640, 480);
    let _ = notatus.state.select_asset(asset_id);
    let w = width as f64;
    let h = height as f64;
    let bbox = BoundingBox::from_xywh(w * 0.12, h * 0.25, w * 0.3, h * 0.5).unwrap();
    let annotation = notatus
        .state
        .add_human_bbox(asset_id, person, bbox, None)
        .unwrap();
    let bbox = BoundingBox::from_xywh(w * 0.55, h * 0.3, w * 0.28, h * 0.45).unwrap();
    notatus
        .state
        .add_human_bbox(asset_id, car, bbox, None)
        .unwrap();
    // A model's suggestion, still waiting for review.
    let suggestion = AnnotationRecord::new_model(
        asset_id,
        car,
        AnnotationGeometry::Bbox(
            BoundingBox::from_xywh(w * 0.62, h * 0.06, w * 0.2, h * 0.16).unwrap(),
        ),
        notatus_core::ModelProvenance {
            name: "detector".to_string(),
            version: None,
            invocation_id: None,
            metadata: Default::default(),
        },
        Some(0.87),
    );
    let _ = notatus
        .state
        .apply_model_predictions(asset_id, vec![suggestion]);
    let _ = notatus.state.select_annotation(Some(annotation));
}

fn capture(cx: &mut HeadlessAppContext, window: AnyWindowHandle, path: &Path) {
    // Let image loading and layout settle across a few frames.
    for _ in 0..6 {
        cx.run_until_parked();
        let _ = cx.update_window(window, |_, window, _| window.refresh());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    // The headless clock is simulated, so advance it past entrance
    // animations (dialogs, dropdowns) and draw once to start them.
    let _ = cx.capture_screenshot(window);
    cx.advance_clock(std::time::Duration::from_secs(1));
    let image = cx.capture_screenshot(window).expect("capture screenshot");
    image.save(path).expect("write screenshot");
    eprintln!("wrote {}", path.display());
}

#[test]
#[ignore = "needs a GPU adapter; run manually for visual review"]
fn screenshots() {
    let out_dir = PathBuf::from(
        env::var("NOTATUS_SCREENSHOT_DIR").unwrap_or_else(|_| "target/screenshots".into()),
    );
    std::fs::create_dir_all(&out_dir).unwrap();
    let sample_image = env::var("NOTATUS_SAMPLE_IMAGE").ok();

    let mut cx = headless_cx();
    cx.update(|cx| {
        gpui_component::init(cx);
        actions::bind_keys(cx);
    });

    let mut view_slot = None;
    let window = cx
        .open_window(size(px(1280.0), px(800.0)), |window, cx| {
            let view = cx.new(|cx| {
                let mut notatus = NotatusWindow::new(window, cx);
                seed_project(&mut notatus, sample_image.as_deref());
                notatus
            });
            view_slot = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        })
        .unwrap();
    let window: AnyWindowHandle = window.into();

    for (mode, name) in [(ThemeMode::Light, "light"), (ThemeMode::Dark, "dark")] {
        cx.update(|cx| Theme::change(mode, None, cx));
        capture(&mut cx, window, &out_dir.join(format!("main-{name}.png")));
    }

    let view = view_slot.expect("window view");

    // The media list, with the first item marked done.
    cx.update_window(window, |_, _, cx| {
        view.update(cx, |notatus, cx| {
            if let Some(first) = notatus.state.dataset().assets.first().map(|a| a.id) {
                let _ = notatus
                    .state
                    .set_asset_status(first, notatus_core::AssetStatus::Done);
            }
            notatus.left_dock = LeftDock::Media;
            cx.notify();
        });
    })
    .unwrap();
    capture(&mut cx, window, &out_dir.join("media-dark.png"));

    // The shortcut sheet, opened before any popover or dialog.
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |notatus, cx| notatus.open_shortcuts(window, cx));
    })
    .unwrap();
    capture(&mut cx, window, &out_dir.join("shortcuts-dark.png"));
    cx.update_window(window, |_, window, cx| window.close_all_dialogs(cx))
        .unwrap();
    cx.update_window(window, |_, _, cx| {
        view.update(cx, |notatus, cx| {
            notatus.left_dock = LeftDock::Dataset;
            cx.notify();
        });
    })
    .unwrap();

    // View popover, opened by clicking its toolbar button.
    cx.update_window(window, |_, _, cx| {
        view.update(cx, |notatus, cx| {
            let first = notatus.state.dataset().annotations.first().map(|a| a.id);
            notatus.state.select_annotation(first).unwrap();
            notatus.view_options.crosshair = true;
            notatus.view_options.focus_selected = true;
            notatus.tools.pointer_image_pos = Some((700.0, 300.0));
            cx.notify();
        });
    })
    .unwrap();
    cx.update(|cx| Theme::change(ThemeMode::Light, None, cx));
    let _ = cx.capture_screenshot(window);
    cx.update_window(window, |_, window, cx| {
        let position = gpui::point(px(614.0), px(88.0));
        for event in [
            gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                button: gpui::MouseButton::Left,
                position,
                modifiers: Default::default(),
                click_count: 1,
                first_mouse: false,
            }),
            gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                button: gpui::MouseButton::Left,
                position,
                modifiers: Default::default(),
                click_count: 1,
            }),
        ] {
            let _ = window.dispatch_event(event, cx);
        }
    })
    .unwrap();
    capture(&mut cx, window, &out_dir.join("view-options-light.png"));
    cx.update_window(window, |_, _, cx| {
        view.update(cx, |notatus, _| {
            notatus.view_options = Default::default();
            notatus.tools.pointer_image_pos = None;
        });
    })
    .unwrap();
    cx.update(|cx| Theme::change(ThemeMode::Dark, None, cx));

    cx.update_window(window, |_, window, cx| {
        window.close_all_dialogs(cx);
        view.update(cx, |notatus, cx| notatus.open_command_palette(window, cx));
    })
    .unwrap();
    capture(&mut cx, window, &out_dir.join("palette-dark.png"));

    cx.update_window(window, |_, window, cx| {
        window.close_all_dialogs(cx);
        view.update(cx, |notatus, cx| notatus.open_label_picker(window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        let picker = view.read(cx).label_picker.clone();
        picker.update(cx, |picker, cx| picker.set_query("Bi", window, cx));
    })
    .unwrap();
    capture(&mut cx, window, &out_dir.join("label-picker-dark.png"));

    cx.update_window(window, |_, window, cx| {
        window.close_all_dialogs(cx);
        view.update(cx, |notatus, cx| {
            notatus.state = UiState::new_project("Untitled project");
            cx.notify();
        });
    })
    .unwrap();
    cx.update(|cx| Theme::change(ThemeMode::Light, None, cx));
    capture(&mut cx, window, &out_dir.join("empty-light.png"));
}
