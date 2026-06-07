use super::*;
use crate::app::helpers::*;

#[derive(Clone)]
struct LabelMenuContext {
    label_id: LabelId,
    label_name: String,
    label_color: Option<String>,
    annotation_count: usize,
    view: gpui::WeakEntity<NotatusWindow>,
}

pub(super) fn labels_section(
    window: &NotatusWindow,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    let add_label_view = view.clone();
    let empty_label_view = view.clone();
    let label_rows = label_rows(window, view.clone());

    div()
        .flex()
        .flex_col()
        .gap_2()
        .border_t_1()
        .border_color(rgb(0xe5e7eb))
        .pt_4()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(section_title("Labels"))
                .child(
                    Button::new("labels-add")
                        .small()
                        .icon(Icon::new(IconName::Plus))
                        .tooltip("Add label")
                        .on_click(move |_, window, cx| {
                            let _ = add_label_view.update(cx, |notatus, cx| {
                                notatus.create_label(window, cx);
                            });
                        }),
                ),
        )
        .child(if window.state.dataset.labels.is_empty() {
            div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
                .py_6()
                .text_sm()
                .text_color(rgb(0x6b7280))
                .child("Create labels before importing media")
                .child(
                    Button::new("labels-empty-add")
                        .small()
                        .icon(Icon::new(IconName::Plus))
                        .label("Add label")
                        .on_click(move |_, window, cx| {
                            let _ = empty_label_view.update(cx, |notatus, cx| {
                                notatus.create_label(window, cx);
                            });
                        }),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .children(label_rows)
                .into_any_element()
        })
}

fn label_rows(
    window: &NotatusWindow,
    view: gpui::WeakEntity<NotatusWindow>,
) -> Vec<gpui::AnyElement> {
    if window.state.dataset.labels.is_empty() {
        vec![
            div()
                .px_2()
                .py_3()
                .text_sm()
                .text_color(rgb(0x6b7280))
                .child("No labels yet")
                .into_any_element(),
        ]
    } else {
        window
            .state
            .dataset
            .labels
            .iter()
            .map(|label| {
                let label_id = label.id;
                let annotation_count = window
                    .state
                    .dataset
                    .annotations
                    .iter()
                    .filter(|annotation| annotation.label_id == label_id)
                    .count();
                let label_name = label.name.clone();
                let label_color = label.color.clone();
                let menu_view = view.clone();
                let select_view = view.clone();
                let selected = window.state.selected_label == Some(label_id);
                label_row(
                    label_id,
                    label_name,
                    label_color,
                    annotation_count,
                    selected,
                    menu_view,
                    select_view,
                )
                .into_any_element()
            })
            .collect()
    }
}

fn label_row(
    label_id: LabelId,
    label_name: String,
    label_color: Option<String>,
    annotation_count: usize,
    selected: bool,
    menu_view: gpui::WeakEntity<NotatusWindow>,
    select_view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    let row_label_name = label_name.clone();
    let row_label_color = label_color.clone();
    let action_view = menu_view.clone();

    div()
        .id(label_element_id("label-row", label_id))
        .w_full()
        .h(px(36.0))
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .rounded_md()
        .px_2()
        .text_sm()
        .text_color(rgb(0x111827))
        .hover(|row| row.bg(rgb(0xf3f4f6)))
        .when(selected, |row| {
            row.bg(rgb(0xe5e7eb)).font_weight(FontWeight::MEDIUM)
        })
        .on_click(move |_, window, cx| {
            let _ = select_view.update(cx, |notatus, cx| {
                notatus.select_label(label_id, window, cx);
            });
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .child(compact_text(&label_name, 24)),
        )
        .child(label_actions(
            label_color.as_deref(),
            annotation_count,
            label_id,
            row_label_name.clone(),
            row_label_color.clone(),
            action_view.clone(),
        ))
        .context_menu(move |menu, window, cx| {
            label_menu(
                menu,
                LabelMenuContext {
                    label_id,
                    label_name: row_label_name.clone(),
                    label_color: row_label_color.clone(),
                    annotation_count,
                    view: menu_view.clone(),
                },
                window,
                cx,
            )
        })
}

fn label_actions(
    color: Option<&str>,
    annotation_count: usize,
    label_id: LabelId,
    label_name: String,
    label_color: Option<String>,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    let menu_view = view.clone();
    let menu_label_name = label_name.clone();
    let menu_label_color = label_color.clone();

    div()
        .id(label_element_id("label-actions", label_id))
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .on_click(|_, _, cx| cx.stop_propagation())
        .child(label_asset_meta(color, annotation_count))
        .child(
            Button::new(label_element_id("label-menu", label_id))
                .xsmall()
                .ghost()
                .icon(Icon::new(IconName::Ellipsis))
                .tooltip("Label options")
                .dropdown_menu(move |menu, window, cx| {
                    label_menu(
                        menu,
                        LabelMenuContext {
                            label_id,
                            label_name: menu_label_name.clone(),
                            label_color: menu_label_color.clone(),
                            annotation_count,
                            view: menu_view.clone(),
                        },
                        window,
                        cx,
                    )
                }),
        )
        .child(
            Button::new(label_element_id("label-remove", label_id))
                .xsmall()
                .ghost()
                .danger()
                .icon(Icon::new(IconName::Delete))
                .tooltip("Remove label")
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    remove_label_or_confirm(
                        view.clone(),
                        label_id,
                        label_name.clone(),
                        annotation_count,
                        window,
                        cx,
                    );
                }),
        )
}

fn label_menu(
    menu: PopupMenu,
    context: LabelMenuContext,
    _: &mut Window,
    _: &mut Context<PopupMenu>,
) -> PopupMenu {
    let LabelMenuContext {
        label_id,
        label_name,
        label_color,
        annotation_count,
        view,
    } = context;
    let rename_view = view.clone();
    let custom_color_view = view.clone();
    let remove_view = view.clone();
    let remove_label_name = label_name.clone();
    let selected_color = label_color
        .as_deref()
        .unwrap_or(DEFAULT_LABEL_COLOR)
        .to_string();

    menu.item(
        PopupMenuItem::new("Rename")
            .icon(Icon::new(IconName::ALargeSmall))
            .on_click(move |_, window, cx| {
                open_label_rename_dialog(rename_view.clone(), label_id, window, cx);
            }),
    )
    .item(PopupMenuItem::separator())
    .item(PopupMenuItem::label("Color"))
    .item(PopupMenuItem::element(move |_, _| {
        label_color_palette(label_id, selected_color.clone(), view.clone())
    }))
    .item(
        PopupMenuItem::new("Custom color...")
            .icon(Icon::new(IconName::Palette))
            .on_click(move |_, window, cx| {
                open_label_color_dialog(custom_color_view.clone(), label_id, window, cx);
            }),
    )
    .item(PopupMenuItem::separator())
    .item(
        PopupMenuItem::new("Remove label")
            .icon(Icon::new(IconName::Delete))
            .on_click(move |_, window, cx| {
                remove_label_or_confirm(
                    remove_view.clone(),
                    label_id,
                    remove_label_name.clone(),
                    annotation_count,
                    window,
                    cx,
                );
            }),
    )
}

fn label_color_palette(
    label_id: LabelId,
    selected_color: String,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .children(LABEL_COLORS.iter().enumerate().map(|(color_ix, color)| {
            label_color_button(
                color_ix,
                color,
                color == &selected_color,
                label_id,
                view.clone(),
            )
        }))
}

fn open_label_rename_dialog(
    view: gpui::WeakEntity<NotatusWindow>,
    label_id: LabelId,
    window: &mut Window,
    cx: &mut App,
) {
    let input = view
        .update(cx, |notatus, cx| {
            notatus.prepare_label_rename(label_id, window, cx);
            notatus.label_name_input.clone()
        })
        .ok();

    let Some(input) = input else {
        return;
    };

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .alert()
            .w(px(360.0))
            .title("Rename label")
            .child(Input::new(&input).small().w_full())
            .button_props(DialogButtonProps::default().ok_text("Done"))
    });
}

fn open_label_color_dialog(
    view: gpui::WeakEntity<NotatusWindow>,
    label_id: LabelId,
    window: &mut Window,
    cx: &mut App,
) {
    let picker = view
        .update(cx, |notatus, cx| {
            notatus.prepare_label_color_picker(label_id, window, cx);
            notatus.label_color_picker.clone()
        })
        .ok();

    let Some(picker) = picker else {
        return;
    };

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .alert()
            .w(px(360.0))
            .title("Label color")
            .child(
                ColorPicker::new(&picker)
                    .featured_colors(default_label_colors())
                    .label("Custom color"),
            )
            .button_props(DialogButtonProps::default().ok_text("Done"))
    });
}

fn default_label_colors() -> Vec<gpui::Hsla> {
    LABEL_COLORS.iter().map(|color| hex_color(color)).collect()
}

fn remove_label_or_confirm(
    view: gpui::WeakEntity<NotatusWindow>,
    label_id: LabelId,
    label_name: String,
    annotation_count: usize,
    window: &mut Window,
    cx: &mut App,
) {
    if annotation_count == 0 {
        let _ = view.update(cx, |notatus, cx| {
            notatus.remove_label(label_id, window, cx);
        });
        return;
    }

    window.open_dialog(cx, move |dialog, _, _| {
        let remove_view = view.clone();
        let message = format!(
            "Removing \"{label_name}\" will also remove {annotation_count} annotation{}.",
            plural(annotation_count)
        );

        dialog
            .confirm()
            .title("Remove label?")
            .child(message)
            .button_props(
                DialogButtonProps::default()
                    .ok_text("Remove label")
                    .ok_variant(ButtonVariant::Danger)
                    .cancel_text("Cancel"),
            )
            .on_ok(move |_, window, cx| {
                let _ = remove_view.update(cx, |notatus, cx| {
                    notatus.remove_label(label_id, window, cx);
                });
                true
            })
    });
}

fn label_element_id(prefix: &'static str, label_id: LabelId) -> gpui::ElementId {
    let value = label_id.as_uuid().as_u128();
    let high = (value >> 64) as u64;
    let low = (value as u64).to_string();
    (gpui::ElementId::from((prefix, high)), low).into()
}
