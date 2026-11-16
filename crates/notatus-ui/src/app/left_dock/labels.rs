use crate::app::helpers::{
    hex_color, label_color_button, label_swatch, plural, record_element_id, sidebar_count,
};
use crate::app::window::NotatusWindow;
use crate::app::{DEFAULT_LABEL_COLOR, LABEL_COLORS};
use gpui::prelude::*;
use gpui::{App, Context, FontWeight, IntoElement, Window, div, px};
use gpui_component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_component::color_picker::ColorPicker;
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::Input;
use gpui_component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::{Icon, IconName, Sizable as _, ThemeColor, WindowExt as _};
use notatus_core::LabelId;

#[derive(Clone)]
struct LabelMenuContext {
    label_id: LabelId,
    label_name: String,
    label_color: Option<String>,
    annotation_count: usize,
    view: gpui::WeakEntity<NotatusWindow>,
}

/// The add button, at the end of the dock's tab bar.
pub(super) fn labels_actions(view: gpui::WeakEntity<NotatusWindow>) -> impl IntoElement {
    Button::new("labels-add")
        .xsmall()
        .ghost()
        .icon(Icon::new(IconName::Plus))
        .tooltip_with_action(
            "New label",
            &crate::app::actions::NewLabel,
            Some(crate::app::actions::CONTEXT),
        )
        .on_click(move |_, _, cx| {
            let _ = view.update(cx, |notatus, cx| {
                notatus.create_label(cx);
            });
        })
}

pub(super) fn labels_content(
    window: &NotatusWindow,
    view: gpui::WeakEntity<NotatusWindow>,
    colors: ThemeColor,
) -> impl IntoElement {
    if window.state.dataset().labels.is_empty() {
        return div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .p_4()
            .text_sm()
            .text_color(colors.muted_foreground)
            .child("Add labels now, or draw a box to create one")
            .child(
                Button::new("labels-empty-add")
                    .small()
                    .icon(Icon::new(IconName::Plus))
                    .label("Add label")
                    .on_click(move |_, _, cx| {
                        let _ = view.update(cx, |notatus, cx| {
                            notatus.create_label(cx);
                        });
                    }),
            )
            .into_any_element();
    }
    div()
        .size_full()
        .overflow_y_scrollbar()
        .p_2()
        .flex()
        .flex_col()
        .gap_1()
        .children(label_rows(window, view, colors))
        .into_any_element()
}

fn label_rows(
    window: &NotatusWindow,
    view: gpui::WeakEntity<NotatusWindow>,
    colors: ThemeColor,
) -> Vec<gpui::AnyElement> {
    let index = window.state.index();
    window
        .state
        .dataset()
        .labels
        .iter()
        .map(|label| {
            label_row(
                LabelMenuContext {
                    label_id: label.id,
                    label_name: label.name.clone(),
                    label_color: label.color.clone(),
                    annotation_count: index.label_annotation_count(label.id),
                    view: view.clone(),
                },
                window.state.selected_label() == Some(label.id),
                colors,
            )
            .into_any_element()
        })
        .collect()
}

/// Swatch, name, and count, like the other label lists; the menu at the
/// end and on right click.
fn label_row(label: LabelMenuContext, selected: bool, colors: ThemeColor) -> impl IntoElement {
    let label_id = label.label_id;
    let uuid = label_id.as_uuid().as_u128();
    let select_view = label.view.clone();
    let row_menu = label.clone();
    let button_menu = label.clone();

    div()
        .id(record_element_id("label-row", uuid))
        .w_full()
        .h(px(36.0))
        .flex()
        .items_center()
        .gap_2()
        .rounded_md()
        .px_2()
        .text_sm()
        .text_color(colors.foreground)
        .hover(move |row| row.bg(colors.list_hover))
        .when(selected, |row| {
            row.bg(colors.list_active).font_weight(FontWeight::MEDIUM)
        })
        .on_click(move |_, _, cx| {
            let _ = select_view.update(cx, |notatus, cx| {
                notatus.select_label(label_id, cx);
            });
        })
        .child(label_swatch(
            label.label_color.as_deref().unwrap_or(DEFAULT_LABEL_COLOR),
            false,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(label.label_name.clone()),
        )
        .child(sidebar_count(label.annotation_count.to_string()))
        .child(
            Button::new(record_element_id("label-menu", uuid))
                .xsmall()
                .ghost()
                .icon(Icon::new(IconName::Ellipsis))
                .tooltip("Label options")
                .on_click(|_, _, cx| cx.stop_propagation())
                .dropdown_menu(move |menu, window, cx| {
                    label_menu(menu, button_menu.clone(), window, cx)
                }),
        )
        .context_menu(move |menu, window, cx| label_menu(menu, row_menu.clone(), window, cx))
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

    window.open_alert_dialog(cx, move |dialog, _, _| {
        dialog
            .width(px(360.0))
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

    window.open_alert_dialog(cx, move |dialog, _, _| {
        dialog
            .width(px(360.0))
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
            notatus.remove_label(label_id, cx);
        });
        return;
    }

    window.open_alert_dialog(cx, move |dialog, _, _| {
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
            .on_ok(move |_, _, cx| {
                let _ = remove_view.update(cx, |notatus, cx| {
                    notatus.remove_label(label_id, cx);
                });
                true
            })
    });
}
