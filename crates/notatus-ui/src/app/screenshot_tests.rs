//! Offscreen screenshots of the main window, for visual review.
//!
//! Ignored by default because they need a GPU adapter. Run with:
//!
//! ```sh
//! NOTATUS_SCREENSHOT_DIR=/tmp/shots NOTATUS_SAMPLE_IMAGE=/path/to/image.png \
//!     cargo test -p notatus-ui screenshots -- --ignored --nocapture
//! ```

use super::*;
use gpui::{AnyWindowHandle, HeadlessAppContext};
use gpui_component::ThemeMode;
use std::sync::Arc;

fn headless_cx() -> HeadlessAppContext {
    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new("Noto Sans"));
    let cx =
        HeadlessAppContext::with_platform(text_system, Arc::new(gpui_kit_assets::Assets), || {
            gpui_platform::current_headless_renderer()
        });
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
    let _ = notatus.state.select_annotation(Some(annotation));
}

fn capture(cx: &mut HeadlessAppContext, window: AnyWindowHandle, path: &Path) {
    // Let image loading and layout settle across a few frames.
    for _ in 0..6 {
        cx.run_until_parked();
        let _ = cx.update_window(window, |_, window, _| window.refresh());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
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
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |notatus, cx| notatus.open_command_palette(window, cx));
    })
    .unwrap();
    capture(&mut cx, window, &out_dir.join("palette-dark.png"));
}
