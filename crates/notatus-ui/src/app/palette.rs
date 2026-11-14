//! Ctrl+K command palette, built from the same actions as the keymap.

use super::actions::*;
use super::helpers::{asset_display_name, compact_text};
use super::*;
use gpui::Action;
use gpui_component::IndexPath;
use gpui_component::command::{Command, CommandGroup, CommandItem};
use gpui_component::kbd::Kbd;

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
        action: impl Action,
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
        self.actions.push(Box::new(action));
        self
    }

    fn command(
        self,
        label: impl Into<SharedString>,
        icon: impl Into<Lucide>,
        action: impl Action,
    ) -> Self {
        self.push(label, Vec::new(), icon, false, action)
    }
}

/// The full Lucide catalog, for icons the component set lacks.
type Lucide = gpui_kit_assets::IconName;

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

    fn palette_groups(&self) -> Vec<PaletteGroup> {
        let undo = self
            .state
            .undo_description()
            .map(|description| format!("Undo {}", description.to_lowercase()))
            .unwrap_or_else(|| "Undo".to_string());
        let redo = self
            .state
            .redo_description()
            .map(|description| format!("Redo {}", description.to_lowercase()))
            .unwrap_or_else(|| "Redo".to_string());

        let mut annotate = PaletteGroup::new("Annotate")
            .command("Select tool", Lucide::MousePointer2, SelectTool)
            .command("Box tool", Lucide::VectorSquare, BoxTool)
            .command("Polygon tool", Lucide::Pentagon, PolygonTool)
            .command(undo, IconName::Undo2, Undo)
            .command(redo, IconName::Redo2, Redo);
        if self.state.selected_annotation.is_some() {
            annotate = annotate
                .command(
                    "Delete selected annotation",
                    IconName::Delete,
                    DeleteSelection,
                )
                .command("Accept annotation", Lucide::CircleCheck, AcceptAnnotation)
                .command("Reject annotation", Lucide::CircleX, RejectAnnotation)
                .command(
                    "Reset review to draft",
                    Lucide::RotateCcw,
                    ResetAnnotationReview,
                );
        }

        let mut media = PaletteGroup::new("Media");
        if let Some(asset) = self.selected_asset() {
            let done = asset.status == notatus_core::AssetStatus::Done;
            media = media
                .command("Next media", IconName::ArrowRight, NextMedia)
                .command("Previous media", IconName::ArrowLeft, PreviousMedia)
                .command(
                    "Next pending media",
                    IconName::ChevronRight,
                    NextPendingMedia,
                )
                .command(
                    if done {
                        "Mark media pending"
                    } else {
                        "Mark media done"
                    },
                    IconName::CircleCheck,
                    ToggleMediaDone,
                )
                .command("Done and next", IconName::Check, DoneAndNext)
                .command("Fit image to canvas", IconName::Maximize, FitCanvas)
                .command("Zoom in", IconName::Plus, ZoomIn)
                .command("Zoom out", IconName::Minus, ZoomOut);
        }
        media = media
            .command("Import images", Lucide::ImagePlus, ImportMedia)
            .command("Import folder", Lucide::FolderOpen, ImportFolder);
        for asset in self.state.dataset.assets.iter().take(MAX_MEDIA_ITEMS) {
            let name = asset_display_name(asset);
            media = media.push(
                format!("Go to {}", compact_text(&name, 48)),
                vec![name, asset.location.display_path().into_owned()],
                IconName::GalleryVerticalEnd,
                self.state.selected_asset == Some(asset.id),
                GoToMedia(asset.id),
            );
        }

        let mut labels = PaletteGroup::new("Labels")
            .command("Find or create label", IconName::Search, PickLabel)
            .command("New label", IconName::Plus, NewLabel);
        let verb = if self.state.selected_annotation.is_some() {
            "Relabel as"
        } else {
            "Use label"
        };
        for (slot, label) in self.state.dataset.labels.iter().enumerate() {
            let title = format!("{verb} {}", label.name);
            let keywords = vec![label.name.clone()];
            let checked = self.state.selected_label == Some(label.id);
            // The first nine labels have number keys; show those in the row.
            labels = if slot < 9 {
                labels.push(
                    title,
                    keywords,
                    IconName::Palette,
                    checked,
                    SelectLabelSlot(slot),
                )
            } else {
                labels.push(
                    title,
                    keywords,
                    IconName::Palette,
                    checked,
                    SelectLabelById(label.id),
                )
            };
        }

        // The same icons as the Dataset panel's buttons.
        let project = PaletteGroup::new("Project")
            .command("Save project", Lucide::Save, SaveProject)
            .command("Save project as", Lucide::SaveAll, SaveProjectAs)
            .command("Open project", IconName::FolderOpen, OpenProject)
            .command("New project", Lucide::FilePlus, NewProject)
            .command("Export annotations", Lucide::Download, ExportAnnotations)
            .command("Keyboard shortcuts", Lucide::Keyboard, ShowShortcuts)
            .command("Toggle light and dark theme", IconName::Moon, ToggleTheme);

        let options = self.view_options;
        let toggle_title = |toggle: ViewToggle| {
            let verb = if toggle.get(&options) {
                "Turn off"
            } else {
                "Turn on"
            };
            format!("{verb} {}", toggle.title().to_lowercase())
        };
        let view = PaletteGroup::new("View")
            .command(
                toggle_title(ViewToggle::HideAnnotations),
                IconName::EyeOff,
                ToggleHideAnnotations,
            )
            .command(
                toggle_title(ViewToggle::Labels),
                Lucide::Tag,
                ToggleLabelNames,
            )
            .command(
                toggle_title(ViewToggle::Crosshair),
                Lucide::Crosshair,
                ToggleCrosshair,
            )
            .command(
                toggle_title(ViewToggle::FocusSelected),
                Lucide::Focus,
                ToggleFocusSelected,
            );

        let mut model = PaletteGroup::new("Model")
            .command("Run model on this image", Lucide::Sparkles, RunModel)
            .command(
                "Run model on unfinished images",
                Lucide::Sparkles,
                RunModelOnUnfinished,
            );
        if self.model_run.is_running() {
            model = model.command("Stop model run", Lucide::CircleStop, StopModelRun);
        }
        model = model.command("Choose model runner…", Lucide::FileCog, ChooseModelRunner);

        vec![annotate, media, labels, model, view, project]
    }
}
