//! Canvas gestures that end with the button released off the canvas.

use super::support::open_test_window;
use crate::app::*;
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext};

struct Canvas {
    cx: VisualTestContext,
    view: gpui::Entity<NotatusWindow>,
}

impl Canvas {
    fn open(cx: &mut TestAppContext, tool: AnnotationTool) -> Self {
        let (window, view) = open_test_window(cx);
        view.update(cx, |notatus, cx| {
            let label = notatus.state.add_label("Person");
            let asset = notatus
                .state
                .add_local_image_asset("/tmp/notatus-gesture.png", 100, 100)
                .unwrap();
            notatus.state.select_asset(asset).unwrap();
            notatus.state.select_label(label).unwrap();
            notatus.set_canvas_tool(tool, cx);
        });
        let cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        Self { cx, view }
    }

    /// The window position of an image coordinate.
    fn screen(&mut self, (x, y): (f64, f64)) -> gpui::Point<Pixels> {
        self.view.update(&mut self.cx, |notatus, _| {
            let bounds = notatus.canvas_image_layout.borrow().unwrap().image_bounds;
            gpui::point(
                bounds.origin.x + bounds.size.width * (x as f32 / 100.0),
                bounds.origin.y + bounds.size.height * (y as f32 / 100.0),
            )
        })
    }

    /// A window position left of the canvas, over the left dock, at the
    /// height of image row `y`.
    fn off_canvas_left(&mut self, y: f64) -> gpui::Point<Pixels> {
        gpui::point(px(4.0), self.screen((0.0, y)).y)
    }

    /// Presses at `from`, moves to `to` on the image, and releases at
    /// `release`, which may be anywhere in the window.
    fn drag_and_release(
        &mut self,
        button: MouseButton,
        from: (f64, f64),
        to: (f64, f64),
        release: gpui::Point<Pixels>,
    ) {
        let none = Modifiers::none();
        let start = self.screen(from);
        let end = self.screen(to);
        self.cx.simulate_mouse_move(start, None, none);
        self.cx.simulate_mouse_down(start, button, none);
        self.cx.simulate_mouse_move(end, button, none);
        self.cx.simulate_mouse_up(release, button, none);
        self.cx.run_until_parked();
    }

    fn boxes(&mut self) -> Vec<BoundingBox> {
        self.view.update(&mut self.cx, |notatus, _| {
            notatus
                .state
                .dataset
                .annotations
                .iter()
                .filter_map(|annotation| annotation.geometry.as_bbox())
                .collect()
        })
    }
}

#[gpui::test]
fn box_released_off_canvas_is_drawn_to_the_image_edge(cx: &mut TestAppContext) {
    let mut canvas = Canvas::open(cx, AnnotationTool::DrawBox);
    let release = canvas.off_canvas_left(60.0);

    canvas.drag_and_release(MouseButton::Left, (50.0, 50.0), (40.0, 60.0), release);

    let boxes = canvas.boxes();
    assert_eq!(boxes.len(), 1, "the box is created");
    let bbox = boxes[0];
    assert!(bbox.x.abs() < 1.0, "clamped to the left edge: {bbox:?}");
    assert!((bbox.max_x() - 50.0).abs() < 1.0, "{bbox:?}");
    assert!((bbox.y - 50.0).abs() < 1.0 && (bbox.max_y() - 60.0).abs() < 1.0);
    canvas.view.update(&mut canvas.cx, |notatus, _| {
        assert!(notatus.tools.draw_box.is_none(), "no drawing left behind");
    });
}

#[gpui::test]
fn move_released_off_canvas_ends_its_undo_step(cx: &mut TestAppContext) {
    let mut canvas = Canvas::open(cx, AnnotationTool::DrawBox);
    let inside = canvas.screen((40.0, 40.0));
    canvas.drag_and_release(MouseButton::Left, (20.0, 20.0), (40.0, 40.0), inside);
    let release = canvas.off_canvas_left(40.0);

    // Drag the box body left, past the canvas edge.
    canvas.drag_and_release(MouseButton::Left, (30.0, 30.0), (25.0, 30.0), release);
    canvas.view.update(&mut canvas.cx, |notatus, _| {
        assert!(notatus.tools.bbox_edit.is_none(), "the edit ended");
    });
    let moved = canvas.boxes()[0];
    assert!(
        moved.x.abs() < 1.0,
        "moved to where it was released: {moved:?}"
    );

    // A second move is its own undo step, not merged into the first.
    let inside = canvas.screen((20.0, 10.0));
    canvas.drag_and_release(MouseButton::Left, (10.0, 30.0), (20.0, 10.0), inside);
    canvas.view.update(&mut canvas.cx, |notatus, _| {
        notatus.state.undo();
    });
    let after_undo = canvas.boxes()[0];
    assert!((after_undo.x - moved.x).abs() < 0.01 && (after_undo.y - moved.y).abs() < 0.01);
}

#[gpui::test]
fn pan_released_off_canvas_stops_panning(cx: &mut TestAppContext) {
    let mut canvas = Canvas::open(cx, AnnotationTool::DrawBox);
    let release = canvas.off_canvas_left(50.0);

    canvas.drag_and_release(MouseButton::Middle, (50.0, 50.0), (30.0, 50.0), release);

    canvas.view.update(&mut canvas.cx, |notatus, _| {
        assert!(notatus.tools.pan.is_none());
    });
}

#[gpui::test]
fn clicks_elsewhere_do_not_touch_the_canvas(cx: &mut TestAppContext) {
    let mut canvas = Canvas::open(cx, AnnotationTool::DrawBox);
    let elsewhere = canvas.off_canvas_left(50.0);
    let revision = canvas
        .view
        .update(&mut canvas.cx, |notatus, _| notatus.state.revision());

    canvas
        .cx
        .simulate_mouse_down(elsewhere, MouseButton::Left, Modifiers::none());
    canvas
        .cx
        .simulate_mouse_up(elsewhere, MouseButton::Left, Modifiers::none());
    canvas.cx.run_until_parked();

    assert!(canvas.boxes().is_empty());
    canvas.view.update(&mut canvas.cx, |notatus, _| {
        assert_eq!(notatus.state.revision(), revision);
    });
}

#[gpui::test]
fn click_with_jitter_on_a_large_image_draws_nothing(cx: &mut TestAppContext) {
    let mut canvas = Canvas::open(cx, AnnotationTool::DrawBox);
    // A 4000 pixel image shrunk to fit: a two-screen-pixel wobble covers many
    // image pixels.
    canvas.view.update(&mut canvas.cx, |notatus, cx| {
        let asset = notatus
            .state
            .add_local_image_asset("/tmp/notatus-large.png", 4000, 4000)
            .unwrap();
        notatus.state.select_asset(asset).unwrap();
        cx.notify();
    });
    canvas.cx.run_until_parked();
    let start = canvas.screen((50.0, 50.0));
    let wobble = start + gpui::point(px(2.0), px(2.0));
    let none = Modifiers::none();

    canvas.cx.simulate_mouse_move(start, None, none);
    canvas
        .cx
        .simulate_mouse_down(start, MouseButton::Left, none);
    canvas
        .cx
        .simulate_mouse_move(wobble, MouseButton::Left, none);
    canvas.cx.simulate_mouse_up(wobble, MouseButton::Left, none);
    canvas.cx.run_until_parked();

    assert!(canvas.boxes().is_empty());
}
