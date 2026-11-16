//! Ctrl+K command palette, built from the same actions as the keymap.

use crate::app::actions::{self, *};
use crate::app::catalog::{Command as CatalogCommand, GROUPS, Lucide};
use crate::app::helpers::{asset_display_name, compact_text};
use crate::app::window::NotatusWindow;
use gpui::prelude::*;
use gpui::{Action, App, Context, IntoElement, SharedString, Window, div, px};
use gpui_component::command::{Command, CommandGroup, CommandItem};
use gpui_component::kbd::Kbd;
use gpui_component::{ActiveTheme as _, Icon, IconName, IndexPath, Sizable as _, WindowExt as _};
use std::rc::Rc;

const MAX_MEDIA_ITEMS: usize = 500;

/// A palette section and the action behind each of its rows, in row order.
struct PaletteGroup {
    group: CommandGroup,
    actions: Vec<Box<dyn Action>>,
}

impl PaletteGroup {
    fn new(label: &'static str) -> Self {
        Self {
            group: CommandGroup::new().label(label),
            actions: Vec::new(),
        }
    }

    /// Adds a row. Actions run from `on_confirm` rather than through the
    /// item, which would dispatch them inside the dialog layer.
    fn push(
        mut self,
        label: impl Into<SharedString>,
        keywords: Vec<String>,
        icon: impl Into<Lucide>,
        checked: bool,
        action: Box<dyn Action>,
    ) -> Self {
        let label = label.into();
        let icon = icon.into();
        let row_label = label.clone();
        let hint_action = action.boxed_clone();
        let item = CommandItem::new()
            .label(label)
            .keywords(keywords)
            .child(move |window, cx| {
                palette_row(
                    icon,
                    row_label.clone(),
                    checked,
                    hint_action.as_ref(),
                    window,
                    cx,
                )
            });
        self.group = self.group.item(item);
        self.actions.push(action);
        self
    }
}

/// Icon, label, and the shortcut as bound in the main window, where the
/// palette's own text field would otherwise hide bare-key bindings.
fn palette_row(
    icon: Lucide,
    label: SharedString,
    checked: bool,
    action: &dyn Action,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let binding = Kbd::binding_for_action(action, Some(actions::CONTEXT), window);
    div()
        .flex_1()
        .flex()
        .items_center()
        .gap_2()
        .child(
            Icon::new(icon)
                .small()
                .text_color(cx.theme().muted_foreground),
        )
        .child(div().flex_1().truncate().child(label))
        .when(checked, |row| {
            row.child(Icon::new(IconName::Check).xsmall())
        })
        .when_some(binding, |row, binding| row.child(binding))
}

impl NotatusWindow {
    pub(super) fn open_command_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }

        let state = self.command_state.clone();
        state.update(cx, |state, cx| state.set_query("", window, cx));
        let (groups, actions): (Vec<_>, Vec<_>) = self
            .palette_groups(cx)
            .into_iter()
            .map(|group| (group.group, group.actions))
            .unzip();
        let actions = Rc::new(actions);
        let target = self.focus_handle.clone();
        let focus_state = state.clone();

        window.open_dialog(cx, move |dialog, _, _| {
            let actions = actions.clone();
            let target = target.clone();
            let command = groups
                .iter()
                .cloned()
                .fold(Command::new(&state), Command::group)
                .bordered(false)
                .max_h(px(380.0))
                .placeholder("Type a command, label, or media name")
                .on_confirm(move |index: IndexPath, window, cx| {
                    window.close_dialog(cx);
                    // The palette lives in Root's dialog layer, outside the
                    // window's handlers, so run the action from the window root.
                    if let Some(action) = actions
                        .get(index.section)
                        .and_then(|group| group.get(index.row))
                    {
                        target.focus(window, cx);
                        target.dispatch_action(action.as_ref(), window, cx);
                    }
                });
            dialog
                .w(px(560.0))
                .p_0()
                .close_button(false)
                .entrance(gpui_component::dialog::DialogEntrance::None)
                .child(command)
        });
        // The dialog takes focus as it opens; hand it to the search field.
        window.defer(cx, move |window, cx| {
            focus_state.update(cx, |state, cx| state.focus(window, cx));
        });
    }

    /// The catalog's commands that can run now, toggles checked when on,
    /// then a row per label and per media.
    fn palette_groups(&self, cx: &App) -> Vec<PaletteGroup> {
        // The annotation loop first; project chores last.
        const ORDER: [&str; 7] = [
            "Annotate", "Review", "Edit", "View", "Model", "File", "Help",
        ];
        let mut groups: Vec<_> = ORDER
            .iter()
            .filter_map(|title| GROUPS.iter().find(|group| group.title == *title))
            .map(|group| {
                group
                    .commands()
                    // Escape cancels in place, and the palette is already open.
                    .filter(|command| {
                        let action = command.action.as_any();
                        !action.is::<Cancel>() && !action.is::<CommandPalette>()
                    })
                    .filter_map(|command| {
                        let status = self.command_status(command, cx);
                        status.enabled.then_some((command, status.checked))
                    })
                    .fold(
                        PaletteGroup::new(group.title),
                        |palette, (command, checked)| {
                            palette.push(
                                self.palette_title(command),
                                Vec::new(),
                                command.icon,
                                checked,
                                command.action.boxed_clone(),
                            )
                        },
                    )
            })
            .collect();

        let verb = if self.state.selected_annotation().is_some() {
            "Relabel as"
        } else {
            "Use label"
        };
        let mut labels = PaletteGroup::new("Labels");
        for (slot, label) in self.state.dataset().labels.iter().enumerate() {
            // The first nine labels have number keys; their rows show them.
            let action: Box<dyn Action> = if slot < 9 {
                Box::new(SelectLabelSlot(slot))
            } else {
                Box::new(SelectLabelById(label.id))
            };
            labels = labels.push(
                format!("{verb} {}", label.name),
                vec![label.name.clone()],
                Lucide::Tag,
                self.state.selected_label() == Some(label.id),
                action,
            );
        }
        // Labels right after the annotation commands.
        groups.insert(2, labels);

        let mut media = PaletteGroup::new("Media");
        for asset in self.state.dataset().assets.iter().take(MAX_MEDIA_ITEMS) {
            let name = asset_display_name(asset);
            media = media.push(
                format!("Go to {}", compact_text(&name, 48)),
                vec![name, asset.location.display_path().into_owned()],
                Lucide::GalleryVerticalEnd,
                self.state.selected_asset() == Some(asset.id),
                Box::new(GoToMedia(asset.id)),
            );
        }
        groups.push(media);
        groups
    }

    /// The catalog title, or for Undo and Redo, what they would undo or redo.
    fn palette_title(&self, command: &CatalogCommand) -> String {
        let action = command.action.as_any();
        let description = if action.is::<Undo>() {
            self.state.undo_description()
        } else if action.is::<Redo>() {
            self.state.redo_description()
        } else {
            None
        };
        match description {
            Some(description) => format!("{} {}", command.title, description.to_lowercase()),
            None => command.title.to_string(),
        }
    }
}
