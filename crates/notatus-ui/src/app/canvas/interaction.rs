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
    let zoom_view = view.clone();
    let canvas = attach_middle_pan(canvas, view.clone());
    let canvas = attach_pointer_tracking(canvas, view.clone(), shared_image_layout.clone());
    canvas
        .when(matches!(active_tool, AnnotationTool::DrawBox), |canvas| {
            attach_draw_box_interactions(
                canvas,
                view.clone(),
                shared_image_layout.clone(),
                annotations.clone(),
            )
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
        .on_scroll(move |event: &ScrollEvent, window, cx| {
            apply_canvas_zoom(&zoom_view, event, window, cx);
        })
}

/// The box tool doubles as the select tool: pressing on a shape selects it
/// and drags or resizes it, pressing on empty image draws a new box, and
/// holding alt always draws so boxes can start inside other boxes.
fn attach_draw_box_interactions(
    canvas: gpui::Stateful<gpui::Div>,
    view: gpui::WeakEntity<NotatusWindow>,
    shared_image_layout: SharedImageLayout,
    annotations: Vec<AnnotationOverlay>,
) -> gpui::Stateful<gpui::Div> {
    let view_down = view.clone();
    let view_move = view.clone();
    let layout_down = shared_image_layout.clone();
    let layout_move = shared_image_layout.clone();
    let annotations_down = annotations.clone();
    let annotations_move = annotations;
    let up_inside = release_handler(
        view.clone(),
        shared_image_layout.clone(),
        finish_box_gesture,
    );
    let up_outside = release_handler(view, shared_image_layout, finish_box_gesture);

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
                        let Some(asset) = notatus.selected_asset() else {
                            return;
                        };
                        let image_pos = screen_to_image(layout.image_bounds, event.position, asset);
                        let target = (!event.modifiers.alt)
                            .then(|| {
                                hit_test_bbox_edit_target(
                                    &annotations_down,
                                    image_pos,
                                    layout.image_bounds,
                                    asset,
                                )
                            })
                            .flatten();
                        match target {
                            Some(target) => begin_annotation_edit(
                                notatus,
                                target,
                                &annotations_down,
                                image_pos,
                                window,
                                cx,
                            ),
                            None => {
                                if notatus.state.selected_annotation.is_some() {
                                    notatus.select_annotation(None, window, cx);
                                }
                                notatus.tools.begin_draw_box(image_pos);
                            }
                        }
                        cx.notify();
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
                    if notatus.tools.draw_box.is_some() {
                        notatus.tools.update_draw_box(image_pos);
                        cx.notify();
                    } else if let Some((annotation_id, bbox)) =
                        notatus.tools.update_bbox_edit(image_pos, &asset)
                    {
                        notatus.update_annotation_bbox(annotation_id, bbox, window, cx);
                    } else if notatus.tools.bbox_edit.is_none() && notatus.tools.pan.is_none() {
                        let cursor = if event.modifiers.alt {
                            gpui::CursorStyle::Crosshair
                        } else {
                            hover_cursor(
                                &annotations_move,
                                image_pos,
                                layout.image_bounds,
                                &asset,
                                gpui::CursorStyle::Crosshair,
                            )
                        };
                        notatus.set_canvas_cursor(Some(cursor), cx);
                    }
                });
            }
        })
        .on_mouse_up(gpui::MouseButton::Left, up_inside)
        .on_mouse_up_out(gpui::MouseButton::Left, up_outside)
}

/// Ends a gesture given the release point in image coordinates, if the
/// pointer was over a laid-out image.
type FinishGesture =
    fn(&mut NotatusWindow, Option<(f64, f64)>, &mut Window, &mut Context<NotatusWindow>);

/// A release listener that finishes the current gesture at the release
/// point. Registered for releases both on and off the canvas: once the
/// pointer leaves the canvas it stops reporting moves, so the release is
/// the only place a gesture dragged past the edge can end.
fn release_handler(
    view: gpui::WeakEntity<NotatusWindow>,
    shared_image_layout: SharedImageLayout,
    finish: FinishGesture,
) -> impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static {
    move |event, window, cx| {
        let layout = *shared_image_layout.borrow();
        let _ = view.update(cx, |notatus, cx| {
            let image_pos = layout
                .zip(notatus.selected_asset())
                .map(|(layout, asset)| screen_to_image(layout.image_bounds, event.position, asset));
            finish(notatus, image_pos, window, cx);
        });
    }
}

/// Ends a box-tool gesture: commits the box being drawn, or ends a move or
/// resize. `image_pos` is where the button was released, clamped to the
/// image.
fn finish_box_gesture(
    notatus: &mut NotatusWindow,
    image_pos: Option<(f64, f64)>,
    window: &mut Window,
    cx: &mut Context<NotatusWindow>,
) {
    if notatus.tools.bbox_edit.is_some() {
        finish_bbox_edit(notatus, image_pos, window, cx);
        return;
    }
    if notatus.tools.draw_box.is_none() {
        return;
    }
    if let Some(image_pos) = image_pos {
        notatus.tools.update_draw_box(image_pos);
    }
    if let Some(completion) = notatus.tools.finish_draw_box(min_box_extent(notatus))
        && let Some(bbox) = completion.bbox
        && let Some(asset_id) = notatus.state.selected_asset
    {
        let result = notatus.state.batch("Draw box", |state| {
            let (label_id, created) = super::super::commands::drawing_label(state);
            state
                .add_human_bbox(asset_id, label_id, bbox, None)
                .map(|_| created)
        });
        notatus.status_message = Some(match result {
            Ok(true) => format!(
                "Created annotation with new label \"{}\"",
                super::super::commands::FALLBACK_LABEL_NAME
            ),
            Ok(false) => "Created annotation".into(),
            Err(error) => error.to_string(),
        });
    }
    cx.notify();
}

/// Smallest box side, in screen pixels, that counts as a drag. Anything
/// smaller is a click with a little hand movement, not a box.
const MIN_BOX_SCREEN_PX: f64 = 4.0;

/// [`MIN_BOX_SCREEN_PX`] in image pixels at the current zoom, and never
/// below one image pixel.
fn min_box_extent(notatus: &NotatusWindow) -> f64 {
    let layout = *notatus.canvas_image_layout.borrow();
    layout
        .zip(notatus.selected_asset())
        .map(|(layout, asset)| {
            let shown_width = f64::from(f32::from(layout.image_bounds.size.width));
            f64::from(asset.dimensions.width) / shown_width.max(1.0)
        })
        .map_or(1.0, |image_px_per_screen_px| {
            (MIN_BOX_SCREEN_PX * image_px_per_screen_px).max(1.0)
        })
}

/// Ends a select-tool gesture: a move, resize, or pan.
fn finish_select_gesture(
    notatus: &mut NotatusWindow,
    image_pos: Option<(f64, f64)>,
    window: &mut Window,
    cx: &mut Context<NotatusWindow>,
) {
    if notatus.tools.bbox_edit.is_some() {
        finish_bbox_edit(notatus, image_pos, window, cx);
    } else if notatus.tools.pan.is_some() {
        notatus.tools.cancel_interactions();
        notatus.set_canvas_cursor(None, cx);
        cx.notify();
    }
}

/// Applies the release point to the box being moved or resized, then ends
/// the edit so the next one starts a new undo step.
fn finish_bbox_edit(
    notatus: &mut NotatusWindow,
    image_pos: Option<(f64, f64)>,
    window: &mut Window,
    cx: &mut Context<NotatusWindow>,
) {
    if let Some(image_pos) = image_pos
        && let Some(asset) = notatus.selected_asset().cloned()
        && let Some((annotation_id, bbox)) = notatus.tools.update_bbox_edit(image_pos, &asset)
    {
        notatus.update_annotation_bbox(annotation_id, bbox, window, cx);
    }
    notatus.tools.cancel_interactions();
    notatus.state.break_history_coalescing();
    notatus.set_canvas_cursor(None, cx);
    cx.notify();
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

                let mut invalid_polygon = None;
                let _ = view_down.update(cx, |notatus, cx| {
                    let Some(asset) = notatus.selected_asset().cloned() else {
                        return;
                    };
                    let image_pos = screen_to_image(layout.image_bounds, event.position, &asset);

                    if event.click_count >= 2 {
                        if notatus.tools.polygon_point_count() >= 3 {
                            match notatus.tools.finish_polygon() {
                                Some(polygon) => {
                                    // A label made on the fly undoes with its shape.
                                    let result = notatus.state.batch("Draw polygon", |state| {
                                        let (label_id, _) =
                                            super::super::commands::drawing_label(state);
                                        state.add_human_polygon(asset.id, label_id, polygon, None)
                                    });
                                    match result {
                                        Ok(_) => {
                                            notatus.status_message =
                                                Some("Created segmentation polygon".into());
                                        }
                                        Err(error) => {
                                            notatus.status_message = Some(error.to_string());
                                            invalid_polygon = Some(error.to_string());
                                        }
                                    }
                                }
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
                    } else {
                        notatus.tools.add_polygon_point(image_pos);
                    }
                    cx.notify();
                });

                if let Some(message) = invalid_polygon {
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
    let layout_down = shared_image_layout.clone();
    let layout_move = shared_image_layout.clone();
    let annotations_down = annotations.clone();
    let annotations_move = annotations;
    let up_inside = release_handler(
        view.clone(),
        shared_image_layout.clone(),
        finish_select_gesture,
    );
    let up_outside = release_handler(view, shared_image_layout, finish_select_gesture);

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
                                Some(target) => begin_annotation_edit(
                                    notatus,
                                    target,
                                    &annotations_down,
                                    image_pos,
                                    window,
                                    cx,
                                ),
                                None => {
                                    notatus.select_annotation(None, window, cx);
                                    notatus
                                        .set_canvas_cursor(Some(gpui::CursorStyle::ClosedHand), cx);
                                    notatus.tools.begin_pan(event.position);
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
                    } else if notatus.tools.pan.is_some() {
                        notatus.tools.update_pan(event.position);
                        cx.notify();
                    } else {
                        let cursor = hover_cursor(
                            &annotations_move,
                            image_pos,
                            layout.image_bounds,
                            &asset,
                            gpui::CursorStyle::OpenHand,
                        );
                        notatus.set_canvas_cursor(Some(cursor), cx);
                    }
                });
            }
        })
        .on_mouse_up(gpui::MouseButton::Left, up_inside)
        .on_mouse_up_out(gpui::MouseButton::Left, up_outside)
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

/// Selects the annotation under the pointer and, for boxes, starts moving or
/// resizing it.
fn begin_annotation_edit(
    notatus: &mut NotatusWindow,
    target: BboxHitTarget,
    annotations: &[AnnotationOverlay],
    image_pos: (f64, f64),
    window: &mut Window,
    cx: &mut Context<NotatusWindow>,
) {
    use super::super::tools::{BboxEditMode, cursor_for_edit_mode};

    let (annotation_id, mode) = match target {
        BboxHitTarget::Handle(id, handle) => (id, BboxEditMode::Resize(handle)),
        BboxHitTarget::Body(id) => (id, BboxEditMode::Move),
    };
    notatus.select_annotation(Some(annotation_id), window, cx);
    match bbox_for_annotation(annotations, annotation_id) {
        Some(bbox) => {
            notatus
                .tools
                .begin_bbox_edit(annotation_id, mode, bbox, image_pos);
            notatus.set_canvas_cursor(Some(cursor_for_edit_mode(mode)), cx);
        }
        // Polygons can be selected but not dragged yet.
        None => notatus.set_canvas_cursor(Some(gpui::CursorStyle::PointingHand), cx),
    }
}

/// Cursor for hovering: resize arrows on handles, a move hand over shapes,
/// and `idle` elsewhere.
fn hover_cursor(
    annotations: &[AnnotationOverlay],
    image_pos: (f64, f64),
    image_bounds: Bounds<Pixels>,
    asset: &AssetRecord,
    idle: gpui::CursorStyle,
) -> gpui::CursorStyle {
    match hit_test_bbox_edit_target(annotations, image_pos, image_bounds, asset) {
        Some(BboxHitTarget::Handle(_, handle)) => {
            super::super::tools::cursor_for_resize_handle(handle)
        }
        Some(BboxHitTarget::Body(_)) => gpui::CursorStyle::OpenHand,
        None => idle,
    }
}

/// Records the pointer's image position while the crosshair is shown.
fn attach_pointer_tracking(
    canvas: gpui::Stateful<gpui::Div>,
    view: gpui::WeakEntity<NotatusWindow>,
    shared_image_layout: SharedImageLayout,
) -> gpui::Stateful<gpui::Div> {
    let hover_view = view.clone();
    canvas
        .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
            let Some(layout) = *shared_image_layout.borrow() else {
                return;
            };
            let _ = view.update(cx, |notatus, cx| {
                if !notatus.view_options.crosshair {
                    return;
                }
                if let Some(asset) = notatus.selected_asset() {
                    let pos = screen_to_image(layout.image_bounds, event.position, asset);
                    notatus.tools.pointer_image_pos = Some(pos);
                    cx.notify();
                }
            });
        })
        .on_hover(move |hovered, _, cx| {
            if !hovered {
                let _ = hover_view.update(cx, |notatus, cx| {
                    if notatus.tools.pointer_image_pos.take().is_some() {
                        cx.notify();
                    }
                });
            }
        })
}

/// Middle-drag pans the view in every tool.
fn attach_middle_pan(
    canvas: gpui::Stateful<gpui::Div>,
    view: gpui::WeakEntity<NotatusWindow>,
) -> gpui::Stateful<gpui::Div> {
    let view_down = view.clone();
    let view_move = view.clone();
    let view_up = view;
    canvas
        .on_mouse_down(
            gpui::MouseButton::Middle,
            move |event: &MouseDownEvent, _, cx| {
                let _ = view_down.update(cx, |notatus, cx| {
                    notatus.tools.begin_pan(event.position);
                    notatus.set_canvas_cursor(Some(gpui::CursorStyle::ClosedHand), cx);
                });
            },
        )
        .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
            if event.pressed_button != Some(gpui::MouseButton::Middle) {
                return;
            }
            let _ = view_move.update(cx, |notatus, cx| {
                if notatus.tools.pan.is_some() {
                    notatus.tools.update_pan(event.position);
                    cx.notify();
                }
            });
        })
        .on_mouse_up(gpui::MouseButton::Middle, end_pan(view_up.clone()))
        .on_mouse_up_out(gpui::MouseButton::Middle, end_pan(view_up))
}

fn end_pan(
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static {
    move |_, _, cx| {
        let _ = view.update(cx, |notatus, cx| {
            if notatus.tools.pan.take().is_some() {
                notatus.set_canvas_cursor(None, cx);
            }
        });
    }
}

fn apply_canvas_zoom(
    view: &gpui::WeakEntity<NotatusWindow>,
    event: &ScrollEvent,
    window: &mut Window,
    cx: &mut App,
) {
    let delta = event.delta.pixel_delta(window.line_height());
    let dy: f32 = delta.y.into();
    if dy.abs() < f32::EPSILON {
        return;
    }
    let factor = if dy < 0.0 { 1.1 } else { 1.0 / 1.1 };
    let _ = view.update(cx, |notatus, cx| {
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
}
