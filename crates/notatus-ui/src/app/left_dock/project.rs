use super::labels::labels_section;
use super::*;
use crate::app::export_commands;
use crate::app::helpers::*;
use crate::app::project_commands;

impl NotatusWindow {
    pub(super) fn dataset_dock(&self, cx: &mut Context<Self>) -> gpui::Div {
        let view = cx.weak_entity();

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(section_title("Dataset"))
                    .child(self.project_actions(view.clone())),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .child(dataset_section("Project", self.project_editor()))
                    .child(labels_section(self, view, cx.theme().colors)),
            )
    }

    fn project_actions(&self, view: gpui::WeakEntity<NotatusWindow>) -> impl IntoElement {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap_1()
            .child(project_action_button(
                "project-new",
                gpui_kit_assets::IconName::FilePlus,
                "New project (Ctrl+N)",
                project_commands::new_project,
                view.clone(),
            ))
            .child(project_action_button(
                "project-open",
                IconName::FolderOpen,
                "Open project (Ctrl+O)",
                project_commands::open_project,
                view.clone(),
            ))
            .child(project_action_button(
                "project-save",
                gpui_kit_assets::IconName::Save,
                "Save project (Ctrl+S)",
                project_commands::save_project,
                view.clone(),
            ))
            .child(project_action_button(
                "project-save-as",
                gpui_kit_assets::IconName::SaveAll,
                "Save project as (Ctrl+Shift+S)",
                project_commands::save_project_as,
                view.clone(),
            ))
            .child(project_action_button(
                "project-export",
                gpui_kit_assets::IconName::Download,
                "Export annotations (Ctrl+E)",
                export_commands::open_export_dialog,
                view,
            ))
    }

    fn project_editor(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(section_title("Name"))
                    .child(Input::new(&self.project_name_input).small().w_full()),
            )
            .child(metric(
                "Location",
                compact_text(&self.project_location.display_name(), 34),
            ))
            .child(themed({
                let created = dataset_created_label(self.state.dataset());
                move |theme| {
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(created)
                }
            }))
    }
}

fn dataset_section(title: &'static str, content: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(section_title(title))
        .child(content)
}

fn project_action_button(
    id: &'static str,
    icon: impl Into<Icon>,
    tooltip: &'static str,
    action: fn(gpui::WeakEntity<NotatusWindow>, &mut Window, &mut App),
    view: gpui::WeakEntity<NotatusWindow>,
) -> Button {
    Button::new(id)
        .small()
        .icon(Icon::new(icon))
        .tooltip(tooltip)
        .on_click(move |_, window, cx| {
            action(view.clone(), window, cx);
        })
}
