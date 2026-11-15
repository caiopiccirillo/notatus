//! Every command the app offers, named and grouped once.
//!
//! The menus, the shortcut sheet, and the command palette are all built from
//! this list, so a command reads the same everywhere and a new one is added
//! in one place. Keys come from the bindings in `actions.rs`.

use crate::app::actions::*;
use gpui::Action;

/// The full Lucide catalog; icons beyond gpui-component's set are
/// registered in `assets.rs`.
pub(super) type Lucide = gpui_kit_assets::IconName;

pub(super) struct Command {
    pub(super) title: &'static str,
    pub(super) icon: Lucide,
    pub(super) action: &'static dyn Action,
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
    })
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
            command("Undo", Lucide::Undo2, &Undo),
            command("Redo", Lucide::Redo2, &Redo),
            SEPARATOR,
            command("Delete selection", Lucide::Trash, &DeleteSelection),
            command("Cancel drawing", Lucide::X, &Cancel),
            SEPARATOR,
            command("New label", Lucide::Plus, &NewLabel),
            command("Choose label", Lucide::Tag, &PickLabel),
        ],
    },
    Group {
        title: "View",
        entries: &[
            command("Fit image", Lucide::Maximize, &FitCanvas),
            command("Zoom in", Lucide::ZoomIn, &ZoomIn),
            command("Zoom out", Lucide::ZoomOut, &ZoomOut),
            SEPARATOR,
            command("Hide annotations", Lucide::EyeOff, &ToggleHideAnnotations),
            command("Label names", Lucide::Tag, &ToggleLabelNames),
            command("Crosshair", Lucide::Crosshair, &ToggleCrosshair),
            command("Focus selected", Lucide::Focus, &ToggleFocusSelected),
            SEPARATOR,
            command("Light and dark theme", Lucide::Moon, &ToggleTheme),
        ],
    },
    Group {
        title: "Annotate",
        entries: &[
            command("Select tool", Lucide::MousePointer2, &SelectTool),
            command("Box tool", Lucide::VectorSquare, &BoxTool),
            command("Polygon tool", Lucide::Pentagon, &PolygonTool),
            SEPARATOR,
            command("Next annotation", Lucide::ArrowDown, &NextAnnotation),
            command("Previous annotation", Lucide::ArrowUp, &PreviousAnnotation),
            SEPARATOR,
            command("Next media", Lucide::ArrowRight, &NextMedia),
            command("Previous media", Lucide::ArrowLeft, &PreviousMedia),
            command(
                "Next unfinished media",
                Lucide::ChevronsRight,
                &NextPendingMedia,
            ),
            command("Mark media done", Lucide::CircleCheck, &ToggleMediaDone),
            command("Done and next", Lucide::CheckCheck, &DoneAndNext),
        ],
    },
    Group {
        title: "Review",
        entries: &[
            command("Accept", Lucide::CircleCheck, &AcceptAnnotation),
            command("Reject", Lucide::CircleX, &RejectAnnotation),
            command("Reset to draft", Lucide::RotateCcw, &ResetAnnotationReview),
            SEPARATOR,
            command("Next draft", Lucide::ChevronRight, &NextDraft),
            command("Previous draft", Lucide::ChevronLeft, &PreviousDraft),
        ],
    },
    Group {
        title: "Model",
        entries: &[
            command("Suggest on this image", Lucide::Sparkles, &RunModel),
            command(
                "Suggest on unfinished images",
                Lucide::Sparkles,
                &RunModelOnUnfinished,
            ),
            command("Stop", Lucide::CircleStop, &StopModelRun),
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
