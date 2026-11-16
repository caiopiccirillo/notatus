//! The application menus: native on macOS, drawn in the title bar on Linux
//! and Windows. Every item dispatches an action, so menus, shortcuts, and the
//! command palette all run the same handlers and show the same keys.

use crate::app::catalog::{CommandStatus, Entry, GROUPS};
use crate::app::window::NotatusWindow;
use gpui::{App, Context, Menu, MenuItem};
use gpui_component::GlobalState;

/// The menus, with `statuses` holding each command's state in catalog order.
fn menus(statuses: &[CommandStatus]) -> Vec<Menu> {
    let mut statuses = statuses.iter();
    GROUPS
        .iter()
        .map(|group| {
            Menu::new(group.title).items(group.entries.iter().map(|entry| match entry {
                Entry::Command(command) => {
                    let status = statuses.next().copied().unwrap_or_default();
                    MenuItem::Action {
                        name: command.title.into(),
                        action: command.action.boxed_clone(),
                        os_action: None,
                        checked: status.checked,
                        disabled: !status.enabled,
                    }
                }
                Entry::Separator => MenuItem::separator(),
            }))
        })
        .collect()
}

/// Installs the menus with the platform, for macOS, and for the title bar's
/// menu bar elsewhere.
fn set_menus(cx: &mut App, statuses: &[CommandStatus]) {
    cx.set_menus(menus(statuses));
    GlobalState::global_mut(cx)
        .set_app_menus(menus(statuses).into_iter().map(Menu::owned).collect());
}

/// Every command enabled, none checked: the menus before a window opens.
fn initial_statuses() -> Vec<CommandStatus> {
    let enabled = CommandStatus {
        enabled: true,
        checked: false,
    };
    vec![enabled; GROUPS.iter().flat_map(|group| group.commands()).count()]
}

/// Installs the menus before any window opens.
pub(super) fn install(cx: &mut App) {
    set_menus(cx, &initial_statuses());
}

/// Whether the title bar draws the menus. macOS shows them in its own bar.
pub(super) const IN_TITLE_BAR: bool = !cfg!(target_os = "macos");

impl NotatusWindow {
    /// Rebuilds the menus when a command's availability or check mark
    /// changed, so they grey out what cannot run and tick what is on.
    pub(super) fn sync_menus(&mut self, cx: &mut Context<Self>) {
        let statuses: Vec<_> = GROUPS
            .iter()
            .flat_map(|group| group.commands())
            .map(|command| self.command_status(command, cx))
            .collect();
        if self.menus_synced.as_ref() == Some(&statuses) {
            return;
        }
        set_menus(cx, &statuses);
        self.menus_synced = Some(statuses);
        if let Some(menu_bar) = &self.app_menu_bar {
            menu_bar.update(cx, |menu_bar, cx| menu_bar.reload(cx));
        }
    }
}

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
                menus(&initial_statuses())
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

    /// The title bar menus' item named `name`, as (checked, disabled).
    fn menu_item(cx: &mut TestAppContext, name: &str) -> (bool, bool) {
        cx.read(|cx| {
            GlobalState::global(cx)
                .app_menus()
                .iter()
                .flat_map(|menu| menu.items.iter())
                .find_map(|item| match item {
                    OwnedMenuItem::Action {
                        name: item_name,
                        checked,
                        disabled,
                        ..
                    } if *item_name == name => Some((*checked, *disabled)),
                    _ => None,
                })
                .expect("a menu item")
        })
    }

    #[gpui::test]
    fn menus_grey_out_what_cannot_run_and_tick_what_is_on(cx: &mut TestAppContext) {
        let (window, _) = open_test_window(cx);

        // A new project has no media, nothing to undo, and no model running.
        assert_eq!(menu_item(cx, "Fit image"), (false, true));
        assert_eq!(menu_item(cx, "Undo"), (false, true));
        assert_eq!(menu_item(cx, "Stop"), (false, true));
        assert_eq!(menu_item(cx, "Label names"), (true, false));
        assert_eq!(menu_item(cx, "Crosshair"), (false, false));

        cx.simulate_keystrokes(window.into(), "c");
        cx.run_until_parked();

        assert_eq!(menu_item(cx, "Crosshair"), (true, false));
    }
}
