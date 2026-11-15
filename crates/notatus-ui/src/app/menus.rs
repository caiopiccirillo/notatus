//! The application menus: native on macOS, drawn in the title bar on Linux
//! and Windows. Every item dispatches an action, so menus, shortcuts, and the
//! command palette all run the same handlers and show the same keys.

use super::actions::*;
use gpui::{App, Menu, MenuItem};
use gpui_component::GlobalState;

fn menus() -> Vec<Menu> {
    vec![
        Menu::new("File").items([
            MenuItem::action("New project", NewProject),
            MenuItem::action("Open project…", OpenProject),
            MenuItem::separator(),
            MenuItem::action("Save", SaveProject),
            MenuItem::action("Save as…", SaveProjectAs),
            MenuItem::separator(),
            MenuItem::action("Import images…", ImportMedia),
            MenuItem::action("Import folder…", ImportFolder),
            MenuItem::separator(),
            MenuItem::action("Export annotations…", ExportAnnotations),
        ]),
        Menu::new("Edit").items([
            MenuItem::action("Undo", Undo),
            MenuItem::action("Redo", Redo),
            MenuItem::separator(),
            MenuItem::action("Delete selection", DeleteSelection),
            MenuItem::separator(),
            MenuItem::action("New label", NewLabel),
            MenuItem::action("Choose label", PickLabel),
        ]),
        Menu::new("View").items([
            MenuItem::action("Fit image", FitCanvas),
            MenuItem::action("Zoom in", ZoomIn),
            MenuItem::action("Zoom out", ZoomOut),
            MenuItem::separator(),
            MenuItem::action("Hide annotations", ToggleHideAnnotations),
            MenuItem::action("Label names", ToggleLabelNames),
            MenuItem::action("Crosshair", ToggleCrosshair),
            MenuItem::action("Focus selected", ToggleFocusSelected),
            MenuItem::separator(),
            MenuItem::action("Toggle light and dark theme", ToggleTheme),
        ]),
        Menu::new("Annotate").items([
            MenuItem::action("Select tool", SelectTool),
            MenuItem::action("Box tool", BoxTool),
            MenuItem::action("Polygon tool", PolygonTool),
            MenuItem::separator(),
            MenuItem::action("Next annotation", NextAnnotation),
            MenuItem::action("Previous annotation", PreviousAnnotation),
            MenuItem::separator(),
            MenuItem::action("Next media", NextMedia),
            MenuItem::action("Previous media", PreviousMedia),
            MenuItem::action("Next unfinished media", NextPendingMedia),
            MenuItem::action("Mark media done", ToggleMediaDone),
            MenuItem::action("Done and next", DoneAndNext),
        ]),
        Menu::new("Review").items([
            MenuItem::action("Accept", AcceptAnnotation),
            MenuItem::action("Reject", RejectAnnotation),
            MenuItem::action("Reset to draft", ResetAnnotationReview),
            MenuItem::separator(),
            MenuItem::action("Next draft", NextDraft),
            MenuItem::action("Previous draft", PreviousDraft),
        ]),
        Menu::new("Model").items([
            MenuItem::action("Suggest on this image", RunModel),
            MenuItem::action("Suggest on unfinished images", RunModelOnUnfinished),
            MenuItem::action("Stop", StopModelRun),
            MenuItem::separator(),
            MenuItem::action("Choose model runner…", ChooseModelRunner),
        ]),
        Menu::new("Help").items([
            MenuItem::action("Command palette", CommandPalette),
            MenuItem::action("Keyboard shortcuts", ShowShortcuts),
        ]),
    ]
}

/// Installs the menus with the platform, for macOS, and for the title bar's
/// menu bar elsewhere.
pub(super) fn install(cx: &mut App) {
    cx.set_menus(menus());
    GlobalState::global_mut(cx).set_app_menus(menus().into_iter().map(Menu::owned).collect());
}

/// Whether the title bar draws the menus. macOS shows them in its own bar.
pub(super) const IN_TITLE_BAR: bool = !cfg!(target_os = "macos");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::support::open_test_window;
    use gpui::{AppContext as _, OwnedMenuItem, TestAppContext};
    use gpui_component::kbd::Kbd;

    #[gpui::test]
    fn every_menu_item_reaches_a_bound_action(cx: &mut TestAppContext) {
        let (window, _) = open_test_window(cx);

        let unbound: Vec<String> = cx
            .update_window(window.into(), |_, window, _| {
                menus()
                    .into_iter()
                    .map(Menu::owned)
                    .flat_map(|menu| menu.items)
                    .filter_map(|item| match item {
                        OwnedMenuItem::Action { name, action, .. } => Some((name, action)),
                        _ => None,
                    })
                    // Menu-only commands need no key.
                    .filter(|(_, action)| {
                        !action.partial_eq(&StopModelRun)
                            && !action.partial_eq(&ChooseModelRunner)
                            && !action.partial_eq(&ToggleTheme)
                    })
                    .filter(|(_, action)| {
                        Kbd::binding_for_action(action.as_ref(), Some(CONTEXT), window).is_none()
                    })
                    .map(|(name, _)| name.to_string())
                    .collect()
            })
            .unwrap();

        assert!(unbound.is_empty(), "menu items without a key: {unbound:?}");
    }
}
