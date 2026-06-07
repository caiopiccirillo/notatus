use super::hit_test::hit_test_bbox_annotation;
use super::layout::screen_to_image;
use super::overlay::AnnotationOverlay;
use super::*;

pub(super) fn attach_canvas_interactions(
    canvas: gpui::Stateful<gpui::Div>,
    active_tool: AnnotationTool,
    view: gpui::WeakEntity<NotatusWindow>,
    shared_image_layout: SharedImageLayout,
    annotations: Vec<AnnotationOverlay>,
) -> gpui::Stateful<gpui::Div> {
    canvas
        .when(matches!(active_tool, AnnotationTool::DrawBox), |canvas| {
            attach_draw_box_interactions(canvas, view.clone(), shared_image_layout.clone())
        })
        .when(matches!(active_tool, AnnotationTool::Select), |canvas| {
            attach_select_interactions(
                canvas,
                view.clone(),
                shared_image_layout.clone(),
                annotations.clone(),
            )
        })
        .when(matches!(active_tool, AnnotationTool::Pan), |canvas| {
            attach_pan_interactions(canvas, view)
        })
}

fn attach_draw_box_interactions(
    canvas: gpui::Stateful<gpui::Div>,
    view: gpui::WeakEntity<NotatusWindow>,
    shared_image_layout: SharedImageLayout,
) -> gpui::Stateful<gpui::Div> {
    let view_down = view.clone();
    let view_move = view.clone();
    let view_up = view;
    let layout_down = shared_image_layout.clone();
    let layout_move = shared_image_layout.clone();
    let layout_up = shared_image_layout;

    canvas
        .on_mouse_down(
            gpui::MouseButton::Left,
            move |event: &MouseDownEvent, _window, cx| {
                if event.click_count >= 2 {
                    let _ = view_down.update(cx, |notatus, cx| {
                        notatus.fit_canvas_to_view(cx);
                    });
                    return;
                }

                let layout = layout_down.borrow();
                if let Some(layout) = *layout {
                    let _ = view_down.update(cx, |notatus, cx| {
                        if let Some(asset) = notatus.selected_asset() {
                            let (ix, iy) =
                                screen_to_image(layout.image_bounds, event.position, asset);
                            notatus.tools.begin_draw_box((ix, iy));
                            cx.notify();
                        }
                    });
                }
            },
        )
        .on_mouse_move(move |event: &MouseMoveEvent, _window, cx| {
            let layout = layout_move.borrow();
            if let Some(layout) = *layout {
                let _ = view_move.update(cx, |notatus, cx| {
                    if notatus.tools.draw_box.is_some()
                        && let Some(asset) = notatus.selected_asset()
                    {
                        let (ix, iy) = screen_to_image(layout.image_bounds, event.position, asset);
                        notatus.tools.update_draw_box((ix, iy));
                        cx.notify();
                    }
                });
            }
        })
        .on_mouse_up(
            gpui::MouseButton::Left,
            move |_event: &MouseUpEvent, window, cx| {
                let layout = layout_up.borrow();
                let skipped_required_step = if let Some(layout) = *layout {
                    view_up
                        .update(cx, |notatus, cx| {
                            let mut skipped_required_step = false;

                            if let Some(completion) = notatus.tools.finish_draw_box()
                                && let Some(asset) = notatus.selected_asset()
                            {
                                if let Some(label_id) = notatus.state.selected_label {
                                    if let Some(bbox) = completion.bbox {
                                        let _ = layout;
                                        match notatus
                                            .state
                                            .add_human_bbox(asset.id, label_id, bbox, None)
                                        {
                                            Ok(_) => {
                                                notatus.status_message =
                                                    Some("Created annotation".into());
                                            }
                                            Err(e) => {
                                                notatus.status_message = Some(e.to_string());
                                            }
                                        }
                                    }
                                } else {
                                    notatus.left_dock = LeftDock::Dataset;
                                    notatus.status_message = Some("Select a label first".into());
                                    skipped_required_step = true;
                                }
                            }

                            cx.notify();
                            skipped_required_step
                        })
                        .unwrap_or(false)
                } else {
                    false
                };

                if skipped_required_step {
                    window.push_notification(
                        Notification::warning("Select a label before drawing annotations.")
                            .title("Label required"),
                        cx,
                    );
                }
            },
        )
}

fn attach_select_interactions(
    canvas: gpui::Stateful<gpui::Div>,
    view: gpui::WeakEntity<NotatusWindow>,
    shared_image_layout: SharedImageLayout,
    annotations: Vec<AnnotationOverlay>,
) -> gpui::Stateful<gpui::Div> {
    canvas.on_mouse_down(
        gpui::MouseButton::Left,
        move |event: &MouseDownEvent, window, cx| {
            if event.click_count >= 2 {
                let _ = view.update(cx, |notatus, cx| {
                    notatus.fit_canvas_to_view(cx);
                });
                return;
            }

            let layout = shared_image_layout.borrow();
            if let Some(layout) = *layout {
                let _ = view.update(cx, |notatus, cx| {
                    if let Some(asset) = notatus.selected_asset() {
                        let image_pos = screen_to_image(layout.image_bounds, event.position, asset);
                        let selected = hit_test_bbox_annotation(&annotations, image_pos);
                        notatus.select_annotation(selected, window, cx);
                    }
                });
            }
        },
    )
}

fn attach_pan_interactions(
    canvas: gpui::Stateful<gpui::Div>,
    view: gpui::WeakEntity<NotatusWindow>,
) -> gpui::Stateful<gpui::Div> {
    let view_down = view.clone();
    let view_move = view.clone();
    let view_up = view.clone();
    let view_scroll = view;

    canvas
        .on_mouse_down(
            gpui::MouseButton::Left,
            move |event: &MouseDownEvent, _window, cx| {
                let _ = view_down.update(cx, |notatus, cx| {
                    if event.click_count >= 2 {
                        notatus.fit_canvas_to_view(cx);
                    } else {
                        notatus.tools.begin_pan(event.position);
                        cx.notify();
                    }
                });
            },
        )
        .on_mouse_move(move |event: &MouseMoveEvent, _window, cx| {
            let _ = view_move.update(cx, |notatus, cx| {
                if notatus.tools.pan.is_some() {
                    notatus.tools.update_pan(event.position);
                    cx.notify();
                }
            });
        })
        .on_mouse_up(
            gpui::MouseButton::Left,
            move |_event: &MouseUpEvent, _window, cx| {
                let _ = view_up.update(cx, |notatus, cx| {
                    notatus.tools.finish_pan();
                    cx.notify();
                });
            },
        )
        .on_scroll_wheel(move |event: &ScrollWheelEvent, window, cx| {
            let delta = event.delta.pixel_delta(window.line_height());
            let dy: f32 = delta.y.into();
            if dy.abs() < f32::EPSILON {
                return;
            }
            let factor = if dy < 0.0 { 1.1 } else { 1.0 / 1.1 };
            let _ = view_scroll.update(cx, |notatus, cx| {
                if let Some(layout) = *notatus.canvas_image_layout.borrow() {
                    notatus
                        .tools
                        .viewport
                        .zoom_at(event.position, layout.fit_bounds, factor);
                } else {
                    notatus.tools.viewport.zoom_by(factor);
                }
                cx.notify();
            });
        })
}
