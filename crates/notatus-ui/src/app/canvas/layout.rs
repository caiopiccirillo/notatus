use super::*;

pub(super) fn compute_image_bounds(
    canvas_bounds: Bounds<Pixels>,
    img_width: f64,
    img_height: f64,
) -> Bounds<Pixels> {
    let canvas_w: f32 = canvas_bounds.size.width.into();
    let canvas_h: f32 = canvas_bounds.size.height.into();
    let scale = (canvas_w / img_width as f32).min(canvas_h / img_height as f32);
    let display_w = img_width as f32 * scale;
    let display_h = img_height as f32 * scale;
    let offset_x = (canvas_w - display_w) / 2.0;
    let offset_y = (canvas_h - display_h) / 2.0;
    bounds(
        gpui::point(
            canvas_bounds.origin.x + px(offset_x),
            canvas_bounds.origin.y + px(offset_y),
        ),
        size(px(display_w), px(display_h)),
    )
}

pub(super) fn apply_viewport_to_bounds(
    fit_bounds: Bounds<Pixels>,
    viewport: super::super::tools::CanvasViewport,
) -> Bounds<Pixels> {
    let fit_w: f32 = fit_bounds.size.width.into();
    let fit_h: f32 = fit_bounds.size.height.into();
    let display_w = fit_w * viewport.zoom;
    let display_h = fit_h * viewport.zoom;
    let extra_w = display_w - fit_w;
    let extra_h = display_h - fit_h;
    let origin_x: f32 = fit_bounds.origin.x.into();
    let origin_y: f32 = fit_bounds.origin.y.into();

    bounds(
        gpui::point(
            px(origin_x + viewport.pan_x - extra_w / 2.0),
            px(origin_y + viewport.pan_y - extra_h / 2.0),
        ),
        size(px(display_w), px(display_h)),
    )
}

pub(super) fn canvas_image_layout(
    canvas_bounds: Bounds<Pixels>,
    fit_bounds: Bounds<Pixels>,
    image_bounds: Bounds<Pixels>,
) -> CanvasImageLayout {
    CanvasImageLayout {
        fit_bounds,
        image_bounds,
        image_bounds_in_canvas: bounds(
            gpui::point(
                image_bounds.origin.x - canvas_bounds.origin.x,
                image_bounds.origin.y - canvas_bounds.origin.y,
            ),
            image_bounds.size,
        ),
    }
}

pub(super) fn screen_to_image(
    img_bounds: Bounds<Pixels>,
    screen_pos: Point<Pixels>,
    asset: &AssetRecord,
) -> (f64, f64) {
    let rel_x: f32 = (screen_pos.x - img_bounds.origin.x).into();
    let rel_y: f32 = (screen_pos.y - img_bounds.origin.y).into();
    let img_w: f32 = img_bounds.size.width.into();
    let img_h: f32 = img_bounds.size.height.into();
    let scale_x = asset.dimensions.width as f32 / img_w;
    let scale_y = asset.dimensions.height as f32 / img_h;
    let ix = (rel_x * scale_x) as f64;
    let iy = (rel_y * scale_y) as f64;
    (
        ix.clamp(0.0, asset.dimensions.width as f64),
        iy.clamp(0.0, asset.dimensions.height as f64),
    )
}

pub(super) fn image_bbox_to_screen(
    img_bounds: Bounds<Pixels>,
    native_width: f64,
    native_height: f64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Bounds<Pixels> {
    let display_w: f32 = img_bounds.size.width.into();
    let display_h: f32 = img_bounds.size.height.into();
    let scale_x = display_w / native_width as f32;
    let scale_y = display_h / native_height as f32;
    let sx: f32 = img_bounds.origin.x.into();
    let sy: f32 = img_bounds.origin.y.into();
    let screen_x = sx + (x as f32 * scale_x);
    let screen_y = sy + (y as f32 * scale_y);
    let screen_w = w as f32 * scale_x;
    let screen_h = h as f32 * scale_y;
    bounds(
        gpui::point(px(screen_x), px(screen_y)),
        size(px(screen_w), px(screen_h)),
    )
}

pub(super) fn image_point_to_screen(
    img_bounds: Bounds<Pixels>,
    native_width: f64,
    native_height: f64,
    x: f64,
    y: f64,
) -> Point<Pixels> {
    let display_w: f32 = img_bounds.size.width.into();
    let display_h: f32 = img_bounds.size.height.into();
    let scale_x = display_w / native_width as f32;
    let scale_y = display_h / native_height as f32;
    let sx: f32 = img_bounds.origin.x.into();
    let sy: f32 = img_bounds.origin.y.into();
    gpui::point(px(sx + x as f32 * scale_x), px(sy + y as f32 * scale_y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{bounds, px, size};
    use notatus_core::{AssetLocation, AssetRecord};

    fn make_test_asset(width: u32, height: u32) -> AssetRecord {
        AssetRecord::new_image(AssetLocation::local("test.jpg"), width, height).unwrap()
    }

    #[test]
    fn screen_to_image_at_origin() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(100.0), px(50.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 0.0);
        assert_eq!(iy, 0.0);
    }

    #[test]
    fn screen_to_image_at_center() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(300.0), px(200.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 400.0);
        assert_eq!(iy, 300.0);
    }

    #[test]
    fn screen_to_image_at_bottom_right() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(500.0), px(350.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 800.0);
        assert_eq!(iy, 600.0);
    }

    #[test]
    fn screen_to_image_clamps_out_of_bounds() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(600.0), px(400.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 800.0);
        assert_eq!(iy, 600.0);
    }

    #[test]
    fn screen_to_image_clamps_negative() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(50.0), px(20.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 0.0);
        assert_eq!(iy, 0.0);
    }

    #[test]
    fn screen_to_image_with_scaling() {
        let img_bounds = bounds(gpui::point(px(0.0), px(0.0)), size(px(200.0), px(150.0)));
        let asset = make_test_asset(800, 600);
        let screen_pos = gpui::point(px(100.0), px(75.0));

        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        assert_eq!(ix, 400.0);
        assert_eq!(iy, 300.0);
    }

    #[test]
    fn image_bbox_to_screen_at_origin() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));

        let screen = image_bbox_to_screen(img_bounds, 800.0, 600.0, 0.0, 0.0, 100.0, 100.0);

        let origin_x: f32 = screen.origin.x.into();
        let origin_y: f32 = screen.origin.y.into();
        let width: f32 = screen.size.width.into();
        let height: f32 = screen.size.height.into();

        assert_eq!(origin_x, 100.0);
        assert_eq!(origin_y, 50.0);
        assert_eq!(width, 50.0);
        assert_eq!(height, 50.0);
    }

    #[test]
    fn image_bbox_to_screen_at_center() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));

        let screen = image_bbox_to_screen(img_bounds, 800.0, 600.0, 400.0, 300.0, 200.0, 200.0);

        let origin_x: f32 = screen.origin.x.into();
        let origin_y: f32 = screen.origin.y.into();
        let width: f32 = screen.size.width.into();
        let height: f32 = screen.size.height.into();

        assert_eq!(origin_x, 300.0);
        assert_eq!(origin_y, 200.0);
        assert_eq!(width, 100.0);
        assert_eq!(height, 100.0);
    }

    #[test]
    fn image_bbox_to_screen_full_image() {
        let img_bounds = bounds(gpui::point(px(0.0), px(0.0)), size(px(400.0), px(300.0)));

        let screen = image_bbox_to_screen(img_bounds, 800.0, 600.0, 0.0, 0.0, 800.0, 600.0);

        let origin_x: f32 = screen.origin.x.into();
        let origin_y: f32 = screen.origin.y.into();
        let width: f32 = screen.size.width.into();
        let height: f32 = screen.size.height.into();

        assert_eq!(origin_x, 0.0);
        assert_eq!(origin_y, 0.0);
        assert_eq!(width, 400.0);
        assert_eq!(height, 300.0);
    }

    #[test]
    fn compute_image_bounds_fits_width() {
        let canvas_bounds = bounds(gpui::point(px(0.0), px(0.0)), size(px(400.0), px(400.0)));

        let img_bounds = compute_image_bounds(canvas_bounds, 800.0, 600.0);

        let origin_x: f32 = img_bounds.origin.x.into();
        let origin_y: f32 = img_bounds.origin.y.into();
        let width: f32 = img_bounds.size.width.into();
        let height: f32 = img_bounds.size.height.into();

        assert_eq!(width, 400.0);
        assert_eq!(height, 300.0);
        assert_eq!(origin_x, 0.0);
        assert_eq!(origin_y, 50.0);
    }

    #[test]
    fn compute_image_bounds_fits_height() {
        let canvas_bounds = bounds(gpui::point(px(0.0), px(0.0)), size(px(600.0), px(300.0)));

        let img_bounds = compute_image_bounds(canvas_bounds, 800.0, 600.0);

        let origin_x: f32 = img_bounds.origin.x.into();
        let origin_y: f32 = img_bounds.origin.y.into();
        let width: f32 = img_bounds.size.width.into();
        let height: f32 = img_bounds.size.height.into();

        assert_eq!(width, 400.0);
        assert_eq!(height, 300.0);
        assert_eq!(origin_x, 100.0);
        assert_eq!(origin_y, 0.0);
    }

    #[test]
    fn compute_image_bounds_exact_fit() {
        let canvas_bounds = bounds(gpui::point(px(0.0), px(0.0)), size(px(800.0), px(600.0)));

        let img_bounds = compute_image_bounds(canvas_bounds, 800.0, 600.0);

        let origin_x: f32 = img_bounds.origin.x.into();
        let origin_y: f32 = img_bounds.origin.y.into();
        let width: f32 = img_bounds.size.width.into();
        let height: f32 = img_bounds.size.height.into();

        assert_eq!(width, 800.0);
        assert_eq!(height, 600.0);
        assert_eq!(origin_x, 0.0);
        assert_eq!(origin_y, 0.0);
    }

    #[test]
    fn apply_viewport_scales_around_fit_center_and_applies_pan() {
        let fit_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let viewport = super::super::super::tools::CanvasViewport {
            zoom: 2.0,
            pan_x: 10.0,
            pan_y: -20.0,
        };

        let transformed = apply_viewport_to_bounds(fit_bounds, viewport);

        let origin_x: f32 = transformed.origin.x.into();
        let origin_y: f32 = transformed.origin.y.into();
        let width: f32 = transformed.size.width.into();
        let height: f32 = transformed.size.height.into();

        assert_eq!(origin_x, -90.0);
        assert_eq!(origin_y, -120.0);
        assert_eq!(width, 800.0);
        assert_eq!(height, 600.0);
    }

    #[test]
    fn canvas_image_layout_keeps_absolute_and_parent_relative_bounds() {
        let canvas_bounds = bounds(gpui::point(px(40.0), px(20.0)), size(px(800.0), px(600.0)));
        let fit_bounds = bounds(gpui::point(px(140.0), px(70.0)), size(px(400.0), px(300.0)));
        let image_bounds = bounds(gpui::point(px(120.0), px(60.0)), size(px(440.0), px(330.0)));

        let layout = canvas_image_layout(canvas_bounds, fit_bounds, image_bounds);

        let rel_x: f32 = layout.image_bounds_in_canvas.origin.x.into();
        let rel_y: f32 = layout.image_bounds_in_canvas.origin.y.into();
        assert_eq!(layout.fit_bounds, fit_bounds);
        assert_eq!(layout.image_bounds, image_bounds);
        assert_eq!(rel_x, 80.0);
        assert_eq!(rel_y, 40.0);
    }

    #[test]
    fn coordinate_conversion_roundtrip() {
        let img_bounds = bounds(gpui::point(px(100.0), px(50.0)), size(px(400.0), px(300.0)));
        let asset = make_test_asset(800, 600);

        let screen_pos = gpui::point(px(250.0), px(162.5));
        let (ix, iy) = screen_to_image(img_bounds, screen_pos, &asset);

        let screen_bbox = image_bbox_to_screen(img_bounds, 800.0, 600.0, ix, iy, 1.0, 1.0);
        let result_x: f32 = screen_bbox.origin.x.into();
        let result_y: f32 = screen_bbox.origin.y.into();

        assert!((result_x - 250.0).abs() < 0.1);
        assert!((result_y - 162.5).abs() < 0.1);
    }
}
