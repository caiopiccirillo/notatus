use super::hit_test::{BboxHitTarget, hit_test_bbox_edit_target};
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
        .when(
            matches!(active_tool, AnnotationTool::DrawPolygon),
            |canvas| {
                attach_draw_polygon_interactions(canvas, view.clone(), shared_image_layout.clone())
            },
        )
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

fn attach_draw_polygon_interactions(
    canvas: gpui::Stateful<gpui::Div>,
    view: gpui::WeakEntity<NotatusWindow>,
    shared_image_layout: SharedImageLayout,
) -> gpui::Stateful<gpui::Div> {
    let view_down = view.clone();
    let view_move = view;
    let layout_down = shared_image_layout.clone();
    let layout_move = shared_image_layout;

    canvas
        .on_mouse_down(
            gpui::MouseButton::Left,
            move |event: &MouseDownEvent, window, cx| {
                let layout = layout_down.borrow();
                let Some(layout) = *layout else {
                    return;
                };

                let mut skipped_required_step = false;
                let mut invalid_polygon = None;
                let _ = view_down.update(cx, |notatus, cx| {
                    let Some(asset) = notatus.selected_asset().cloned() else {
                        return;
                    };
                    let image_pos = screen_to_image(layout.image_bounds, event.position, &asset);

                    if event.click_count >= 2 {
                        if notatus.tools.polygon_point_count() >= 3 {
                            match notatus.tools.finish_polygon() {
                                Some(polygon) => match notatus.state.selected_label {
                                    Some(label_id) => match notatus
                                        .state
                                        .add_human_polygon(asset.id, label_id, polygon, None)
                                    {
                                        Ok(_) => {
                                            notatus.status_message =
                                                Some("Created segmentation polygon".into());
                                        }
                                        Err(error) => {
                                            notatus.status_message = Some(error.to_string());
                                            invalid_polygon = Some(error.to_string());
                                        }
                                    },
                                    None => {
                                        notatus.left_dock = LeftDock::Dataset;
                                        notatus.status_message =
                                            Some("Select a label first".into());
                                        skipped_required_step = true;
                                    }
                                },
                                None => {
                                    invalid_polygon = Some(
                                        "Polygon needs at least three valid points".to_string(),
                                    );
                                    notatus.status_message = invalid_polygon.clone();
                                }
                            }
                        } else {
                            notatus.fit_canvas_to_view(cx);
                        }
                    } else if notatus.state.selected_label.is_some() {
                        notatus.tools.add_polygon_point(image_pos);
                    } else {
                        notatus.left_dock = LeftDock::Dataset;
                        notatus.status_message = Some("Select a label first".into());
                        skipped_required_step = true;
                    }
                    cx.notify();
                });

                if skipped_required_step {
                    window.push_notification(
                        Notification::warning("Select a label before drawing annotations.")
                            .title("Label required"),
                        cx,
                    );
                } else if let Some(message) = invalid_polygon {
                    window.push_notification(
                        Notification::warning(message).title("Invalid polygon"),
                        cx,
                    );
                }
            },
        )
        .on_mouse_move(move |event: &MouseMoveEvent, _window, cx| {
            let layout = layout_move.borrow();
            let Some(layout) = *layout else {
                return;
            };

            let _ = view_move.update(cx, |notatus, cx| {
                if notatus.tools.draw_polygon.is_none() {
                    return;
                }
                if let Some(asset) = notatus.selected_asset().cloned() {
                    let image_pos = screen_to_image(layout.image_bounds, event.position, &asset);
                    notatus.tools.update_polygon_cursor(image_pos);
                    cx.notify();
                }
            });
        })
}

fn attach_select_interactions(
    canvas: gpui::Stateful<gpui::Div>,
    view: gpui::WeakEntity<NotatusWindow>,
    shared_image_layout: SharedImageLayout,
    annotations: Vec<AnnotationOverlay>,
) -> gpui::Stateful<gpui::Div> {
    let view_down = view.clone();
    let view_move = view.clone();
    let view_up = view;
    let layout_down = shared_image_layout.clone();
    let layout_move = shared_image_layout.clone();
    let annotations_down = annotations.clone();
    let annotations_move = annotations;

    canvas
        .on_mouse_down(
            gpui::MouseButton::Left,
            move |event: &MouseDownEvent, window, cx| {
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
                            let image_pos =
                                screen_to_image(layout.image_bounds, event.position, asset);
                            let hit_target = hit_test_bbox_edit_target(
                                &annotations_down,
                                image_pos,
                                layout.image_bounds,
                                asset,
                            );
                            match hit_target {
                                Some(BboxHitTarget::Handle(annotation_id, handle)) => {
                                    if let Some(bbox) =
                                        bbox_for_annotation(&annotations_down, annotation_id)
                                    {
                                        let mode =
                                            super::super::tools::BboxEditMode::Resize(handle);
                                        notatus.select_annotation(Some(annotation_id), window, cx);
                                        notatus.tools.begin_bbox_edit(
                                            annotation_id,
                                            mode,
                                            bbox,
                                            image_pos,
                                        );
                                        notatus.set_canvas_cursor(
                                            Some(super::super::tools::cursor_for_edit_mode(mode)),
                                            cx,
                                        );
                                    }
                                }
                                Some(BboxHitTarget::Body(annotation_id)) => {
                                    if let Some(bbox) =
                                        bbox_for_annotation(&annotations_down, annotation_id)
                                    {
                                        let mode = super::super::tools::BboxEditMode::Move;
                                        notatus.select_annotation(Some(annotation_id), window, cx);
                                        notatus.tools.begin_bbox_edit(
                                            annotation_id,
                                            mode,
                                            bbox,
                                            image_pos,
                                        );
                                        notatus.set_canvas_cursor(
                                            Some(super::super::tools::cursor_for_edit_mode(mode)),
                                            cx,
                                        );
                                    } else {
                                        notatus.select_annotation(Some(annotation_id), window, cx);
                                        notatus.set_canvas_cursor(
                                            Some(gpui::CursorStyle::PointingHand),
                                            cx,
                                        );
                                    }
                                }
                                None => {
                                    notatus.select_annotation(None, window, cx);
                                    notatus.set_canvas_cursor(None, cx);
                                }
                            }
                            cx.notify();
                        }
                    });
                }
            },
        )
        .on_mouse_move(move |event: &MouseMoveEvent, window, cx| {
            let layout = layout_move.borrow();
            if let Some(layout) = *layout {
                let _ = view_move.update(cx, |notatus, cx| {
                    let Some(asset) = notatus.selected_asset().cloned() else {
                        return;
                    };
                    let image_pos = screen_to_image(layout.image_bounds, event.position, &asset);
                    if let Some((annotation_id, bbox)) =
                        notatus.tools.update_bbox_edit(image_pos, &asset)
                    {
                        let cursor = notatus.tools.bbox_edit.map(|edit| edit.cursor_style());
                        notatus.set_canvas_cursor(cursor, cx);
                        notatus.update_annotation_bbox(annotation_id, bbox, window, cx);
                    } else {
                        let cursor = match hit_test_bbox_edit_target(
                            &annotations_move,
                            image_pos,
                            layout.image_bounds,
                            &asset,
                        ) {
                            Some(BboxHitTarget::Handle(_, handle)) => {
                                Some(super::super::tools::cursor_for_resize_handle(handle))
                            }
                            Some(BboxHitTarget::Body(_)) => Some(gpui::CursorStyle::OpenHand),
                            None => None,
                        };
                        notatus.set_canvas_cursor(cursor, cx);
                    }
                });
            }
        })
        .on_mouse_up(
            gpui::MouseButton::Left,
            move |_event: &MouseUpEvent, _window, cx| {
                let _ = view_up.update(cx, |notatus, cx| {
                    notatus.tools.finish_bbox_edit();
                    notatus.set_canvas_cursor(None, cx);
                    cx.notify();
                });
            },
        )
}

fn bbox_for_annotation(
    annotations: &[AnnotationOverlay],
    annotation_id: AnnotationId,
) -> Option<BoundingBox> {
    annotations.iter().find_map(|annotation| {
        (annotation.id == annotation_id)
            .then(|| annotation.geometry.as_bbox())
            .flatten()
    })
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
