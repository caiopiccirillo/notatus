//! The application menus: native on macOS, drawn in the title bar on Linux
//! and Windows. Every item dispatches an action, so menus, shortcuts, and the
//! command palette all run the same handlers and show the same keys.

use super::catalog::{Entry, GROUPS};
use gpui::{App, Menu, MenuItem};
use gpui_component::GlobalState;

fn menus() -> Vec<Menu> {
    GROUPS
        .iter()
        .map(|group| {
            Menu::new(group.title).items(group.entries.iter().map(|entry| match entry {
                Entry::Command(command) => MenuItem::Action {
                    name: command.title.into(),
                    action: command.action.boxed_clone(),
                    os_action: None,
                    checked: false,
                    disabled: false,
                },
                Entry::Separator => MenuItem::separator(),
            }))
        })
        .collect()
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
    use crate::app::actions::*;
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
