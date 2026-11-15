//! Ctrl+K command palette, built from the same actions as the keymap.

use crate::app::actions::{self, *};
use crate::app::catalog::{Command as CatalogCommand, GROUPS, Lucide};
use crate::app::helpers::{asset_display_name, compact_text};
use crate::app::view_options::ViewToggle;
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
            .palette_groups()
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

    /// The catalog's commands that apply right now, then a row per label
    /// and per media.
    fn palette_groups(&self) -> Vec<PaletteGroup> {
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
                    .filter(|command| self.palette_offers(command.action))
                    .fold(PaletteGroup::new(group.title), |palette, command| {
                        palette.push(
                            self.palette_title(command),
                            Vec::new(),
                            command.icon,
                            false,
                            command.action.boxed_clone(),
                        )
                    })
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

    /// Whether the palette lists `action` now: commands on a selection or
    /// the current media only while there is one.
    fn palette_offers(&self, action: &dyn Action) -> bool {
        let action = action.as_any();
        let has_annotation = self.state.selected_annotation().is_some();
        let has_media = self.state.selected_asset().is_some();
        if action.is::<DeleteSelection>()
            || action.is::<AcceptAnnotation>()
            || action.is::<RejectAnnotation>()
            || action.is::<ResetAnnotationReview>()
        {
            has_annotation
        } else if action.is::<NextDraft>() || action.is::<PreviousDraft>() {
            self.state.review_queue_len() > 0
        } else if action.is::<NextMedia>()
            || action.is::<PreviousMedia>()
            || action.is::<NextPendingMedia>()
            || action.is::<ToggleMediaDone>()
            || action.is::<DoneAndNext>()
            || action.is::<NextAnnotation>()
            || action.is::<PreviousAnnotation>()
            || action.is::<FitCanvas>()
            || action.is::<ZoomIn>()
            || action.is::<ZoomOut>()
        {
            has_media
        } else if action.is::<StopModelRun>() {
            self.model_run.is_running()
        } else {
            // Escape cancels a drawing in place; the palette is already open.
            !action.is::<Cancel>() && !action.is::<CommandPalette>()
        }
    }

    /// The catalog title, or one that says what the command will do now.
    fn palette_title(&self, command: &CatalogCommand) -> String {
        let action = command.action.as_any();
        let with_description = |verb: &str, description: Option<&str>| match description {
            Some(description) => format!("{verb} {}", description.to_lowercase()),
            None => verb.to_string(),
        };
        if action.is::<Undo>() {
            return with_description("Undo", self.state.undo_description());
        }
        if action.is::<Redo>() {
            return with_description("Redo", self.state.redo_description());
        }
        if action.is::<ToggleMediaDone>()
            && self
                .selected_asset()
                .is_some_and(|asset| asset.status == notatus_core::AssetStatus::Done)
        {
            return "Mark media not done".to_string();
        }
        let toggle = if action.is::<ToggleHideAnnotations>() {
            Some(ViewToggle::HideAnnotations)
        } else if action.is::<ToggleLabelNames>() {
            Some(ViewToggle::Labels)
        } else if action.is::<ToggleCrosshair>() {
            Some(ViewToggle::Crosshair)
        } else if action.is::<ToggleFocusSelected>() {
            Some(ViewToggle::FocusSelected)
        } else {
            None
        };
        match toggle {
            Some(toggle) => {
                let verb = if toggle.get(&self.view_options) {
                    "Turn off"
                } else {
                    "Turn on"
                };
                format!("{verb} {}", toggle.title().to_lowercase())
            }
            None => command.title.to_string(),
        }
    }
}
