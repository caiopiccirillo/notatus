//! Every command the app offers, named and grouped once.
//!
//! The menus, the shortcut sheet, and the command palette are all built from
//! this list, so a command reads the same everywhere and a new one is added
//! in one place. Each command also says when it applies and what state it
//! shows, so the menus and the palette agree on both. Keys come from the
//! bindings in `actions.rs`.

use crate::app::actions::*;
use crate::app::view_options::ViewToggle;
use crate::app::window::NotatusWindow;
use gpui::{Action, App};
use gpui_component::ActiveTheme as _;
use notatus_core::AssetStatus;

/// The full Lucide catalog; icons beyond gpui-component's set are
/// registered in `assets.rs`.
pub(super) type Lucide = gpui_kit_assets::IconName;

pub(super) struct Command {
    pub(super) title: &'static str,
    pub(super) icon: Lucide,
    pub(super) action: &'static dyn Action,
    /// What must exist for the command to do anything.
    needs: Needs,
    /// The on or off state the command shows, for toggles.
    check: Option<Check>,
}

/// What a command acts on, so it is offered only while that exists.
#[derive(Clone, Copy)]
enum Needs {
    Nothing,
    /// A current media.
    Media,
    /// A selected annotation.
    Selection,
    /// Drafts in the review queue.
    Drafts,
    /// A model run in progress.
    ModelRun,
    UndoStep,
    RedoStep,
}

/// The state a toggle shows with a check mark.
#[derive(Clone, Copy)]
enum Check {
    View(ViewToggle),
    MediaDone,
    DarkTheme,
}

/// Whether a command can run now, and whether its toggle is on.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CommandStatus {
    pub(super) enabled: bool,
    pub(super) checked: bool,
}

pub(super) enum Entry {
    Command(Command),
    Separator,
}

pub(super) struct Group {
    pub(super) title: &'static str,
    pub(super) entries: &'static [Entry],
}

impl Group {
    pub(super) fn commands(&self) -> impl Iterator<Item = &Command> {
        self.entries.iter().filter_map(|entry| match entry {
            Entry::Command(command) => Some(command),
            Entry::Separator => None,
        })
    }
}

const fn command(title: &'static str, icon: Lucide, action: &'static dyn Action) -> Entry {
    Entry::Command(Command {
        title,
        icon,
        action,
        needs: Needs::Nothing,
        check: None,
    })
}

impl Entry {
    const fn needs(self, needs: Needs) -> Self {
        match self {
            Entry::Command(command) => Entry::Command(Command { needs, ..command }),
            Entry::Separator => Entry::Separator,
        }
    }

    const fn checks(self, check: Check) -> Self {
        match self {
            Entry::Command(command) => Entry::Command(Command {
                check: Some(check),
                ..command
            }),
            Entry::Separator => Entry::Separator,
        }
    }
}

const SEPARATOR: Entry = Entry::Separator;

/// The menus, in menu-bar order.
pub(super) const GROUPS: &[Group] = &[
    Group {
        title: "File",
        entries: &[
            command("New project", Lucide::FilePlus, &NewProject),
            command("Open project…", Lucide::FolderOpen, &OpenProject),
            SEPARATOR,
            command("Save", Lucide::Save, &SaveProject),
            command("Save as…", Lucide::SaveAll, &SaveProjectAs),
            SEPARATOR,
            command("Import images…", Lucide::ImagePlus, &ImportMedia),
            command("Import folder…", Lucide::FolderOpen, &ImportFolder),
            SEPARATOR,
            command("Export annotations…", Lucide::Download, &ExportAnnotations),
        ],
    },
    Group {
        title: "Edit",
        entries: &[
            command("Undo", Lucide::Undo2, &Undo).needs(Needs::UndoStep),
            command("Redo", Lucide::Redo2, &Redo).needs(Needs::RedoStep),
            SEPARATOR,
            command("Delete selection", Lucide::Trash, &DeleteSelection).needs(Needs::Selection),
            command("Cancel drawing", Lucide::X, &Cancel),
            SEPARATOR,
            command("New label", Lucide::Plus, &NewLabel),
            command("Choose label", Lucide::Tag, &PickLabel),
        ],
    },
    Group {
        title: "View",
        entries: &[
            command("Fit image", Lucide::Maximize, &FitCanvas).needs(Needs::Media),
            command("Zoom in", Lucide::ZoomIn, &ZoomIn).needs(Needs::Media),
            command("Zoom out", Lucide::ZoomOut, &ZoomOut).needs(Needs::Media),
            SEPARATOR,
            command("Hide annotations", Lucide::EyeOff, &ToggleHideAnnotations)
                .checks(Check::View(ViewToggle::HideAnnotations)),
            command("Label names", Lucide::Tag, &ToggleLabelNames)
                .checks(Check::View(ViewToggle::Labels)),
            command("Crosshair", Lucide::Crosshair, &ToggleCrosshair)
                .checks(Check::View(ViewToggle::Crosshair)),
            command("Focus selected", Lucide::Focus, &ToggleFocusSelected)
                .checks(Check::View(ViewToggle::FocusSelected)),
            SEPARATOR,
            command("Dark theme", Lucide::Moon, &ToggleTheme).checks(Check::DarkTheme),
        ],
    },
    Group {
        title: "Annotate",
        entries: &[
            command("Select tool", Lucide::MousePointer2, &SelectTool),
            command("Box tool", Lucide::VectorSquare, &BoxTool),
            command("Polygon tool", Lucide::Pentagon, &PolygonTool),
            SEPARATOR,
            command("Next annotation", Lucide::ArrowDown, &NextAnnotation).needs(Needs::Media),
            command("Previous annotation", Lucide::ArrowUp, &PreviousAnnotation)
                .needs(Needs::Media),
            SEPARATOR,
            command("Next media", Lucide::ArrowRight, &NextMedia).needs(Needs::Media),
            command("Previous media", Lucide::ArrowLeft, &PreviousMedia).needs(Needs::Media),
            command(
                "Next unfinished media",
                Lucide::ChevronsRight,
                &NextPendingMedia,
            )
            .needs(Needs::Media),
            command("Media done", Lucide::CircleCheck, &ToggleMediaDone)
                .needs(Needs::Media)
                .checks(Check::MediaDone),
            command("Done and next", Lucide::CheckCheck, &DoneAndNext).needs(Needs::Media),
        ],
    },
    Group {
        title: "Review",
        entries: &[
            command("Accept", Lucide::CircleCheck, &AcceptAnnotation).needs(Needs::Selection),
            command("Reject", Lucide::CircleX, &RejectAnnotation).needs(Needs::Selection),
            command("Reset to draft", Lucide::RotateCcw, &ResetAnnotationReview)
                .needs(Needs::Selection),
            SEPARATOR,
            command("Next draft", Lucide::ChevronRight, &NextDraft).needs(Needs::Drafts),
            command("Previous draft", Lucide::ChevronLeft, &PreviousDraft).needs(Needs::Drafts),
        ],
    },
    Group {
        title: "Model",
        entries: &[
            command("Suggest on this image", Lucide::Sparkles, &RunModel).needs(Needs::Media),
            command(
                "Suggest on unfinished images",
                Lucide::Sparkles,
                &RunModelOnUnfinished,
            ),
            command("Stop", Lucide::CircleStop, &StopModelRun).needs(Needs::ModelRun),
            SEPARATOR,
            command("Choose model runner…", Lucide::FileCog, &ChooseModelRunner),
        ],
    },
    Group {
        title: "Help",
        entries: &[
            command("Command palette", Lucide::Command, &CommandPalette),
            command("Keyboard shortcuts", Lucide::Keyboard, &ShowShortcuts),
        ],
    },
];

impl NotatusWindow {
    /// Whether `command` can run now and whether its toggle is on, read the
    /// same way by the menus and the palette.
    pub(super) fn command_status(&self, command: &Command, cx: &App) -> CommandStatus {
        let state = &self.state;
        let enabled = match command.needs {
            Needs::Nothing => true,
            Needs::Media => state.selected_asset().is_some(),
            Needs::Selection => state.selected_annotation().is_some(),
            Needs::Drafts => state.review_queue_len() > 0,
            Needs::ModelRun => self.model_run.is_running(),
            Needs::UndoStep => state.undo_description().is_some(),
            Needs::RedoStep => state.redo_description().is_some(),
        };
        let checked = command.check.is_some_and(|check| match check {
            Check::View(toggle) => toggle.get(&self.view_options),
            Check::MediaDone => self
                .selected_asset()
                .is_some_and(|asset| asset.status == AssetStatus::Done),
            Check::DarkTheme => cx.theme().is_dark(),
        });
        CommandStatus { enabled, checked }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_palette_orders_every_group() {
        // palette.rs lists the groups by title.
        let titles: Vec<_> = GROUPS.iter().map(|group| group.title).collect();
        assert_eq!(
            titles,
            [
                "File", "Edit", "View", "Annotate", "Review", "Model", "Help"
            ]
        );
    }

    #[test]
    fn every_action_appears_once() {
        let mut names: Vec<_> = GROUPS
            .iter()
            .flat_map(Group::commands)
            .map(|command| command.action.name())
            .collect();
        let count = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), count, "an action is listed twice");
    }
}
