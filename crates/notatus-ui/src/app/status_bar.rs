use crate::app::actions::{
    self, DoneAndNext, FitCanvas, NextMedia, PreviousMedia, StopModelRun, ToggleMediaDone,
};
use crate::app::helpers::plural;
use crate::app::model_run::ModelProgress;
use crate::app::window::NotatusWindow;
use gpui::prelude::*;
use gpui::{Action, Context, IntoElement, div, px};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::progress::Progress;
use gpui_component::status_bar::StatusBar;
use gpui_component::{Icon, IconName, Selectable as _, Sizable as _};
use notatus_core::AssetStatus;

impl NotatusWindow {
    /// Feedback on the left, media navigation in the middle, and the model
    /// run and zoom on the right.
    pub(super) fn status_bar(&self, _: &mut Context<Self>) -> impl IntoElement {
        // Equal sides keep the navigation centered under the canvas whether
        // or not a message or a model run is showing; StatusBar alone
        // centers its middle only while both ends have content.
        let side = || {
            div()
                .flex_1()
                .flex_basis(px(0.0))
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
        };
        let left = side().when_some(self.status_message.clone(), |side, message| {
            side.child(div().id("status-message").truncate().child(message))
        });
        let right = side()
            .justify_end()
            .when_some(self.model_run.progress(), |side, progress| {
                side.child(model_progress(progress))
            })
            .when(self.state.selected_asset().is_some(), |side| {
                side.child(
                    Button::new("status-zoom")
                        .xsmall()
                        .ghost()
                        .label(format!("{:.0}%", self.tools.viewport.zoom * 100.0))
                        .tooltip_with_action("Fit image", &FitCanvas, Some(actions::CONTEXT))
                        .on_click(|_, window, cx| {
                            window.dispatch_action(FitCanvas.boxed_clone(), cx)
                        }),
                )
            });
        StatusBar::new().h(px(36.0)).px_3().child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap_3()
                .child(left)
                .child(div().flex_none().child(self.review_controls()))
                .child(right),
        )
    }

    /// Previous/next media, progress, and the done toggle, centered in the bar.
    fn review_controls(&self) -> impl IntoElement {
        // The empty canvas guides a project without media.
        let Some(progress) = self.progress_summary() else {
            return div().into_any_element();
        };
        let done = self
            .selected_asset()
            .is_some_and(|asset| asset.status == notatus_core::AssetStatus::Done);
        let has_asset = self.state.selected_asset().is_some();

        div()
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .child(action_button(
                "status-previous",
                IconName::ChevronLeft,
                "Previous media",
                PreviousMedia,
            ))
            .child(div().px_1().child(progress))
            .child(action_button(
                "status-next",
                IconName::ChevronRight,
                "Next media",
                NextMedia,
            ))
            .when_some(self.current_annotation_summary(), |row, summary| {
                row.child(div().px_1().child(summary))
            })
            .when(has_asset, |row| {
                row.child(
                    Button::new("status-done")
                        .xsmall()
                        .icon(Icon::new(IconName::CircleCheck))
                        .label(if done { "Done" } else { "Mark done" })
                        .selected(done)
                        .tooltip_with_action(
                            "Toggle done",
                            &ToggleMediaDone,
                            Some(actions::CONTEXT),
                        )
                        .on_click(|_, window, cx| {
                            window.dispatch_action(ToggleMediaDone.boxed_clone(), cx)
                        }),
                )
                .child(
                    Button::new("status-done-next")
                        .xsmall()
                        .ghost()
                        .label("Done & next")
                        .tooltip_with_action(
                            "Mark done and go to next pending",
                            &DoneAndNext,
                            Some(actions::CONTEXT),
                        )
                        .on_click(|_, window, cx| {
                            window.dispatch_action(DoneAndNext.boxed_clone(), cx)
                        }),
                )
            })
            .into_any_element()
    }

    /// One-line summary of review progress for the status bar.
    pub(super) fn progress_summary(&self) -> Option<String> {
        let assets = &self.state.dataset().assets;
        if assets.is_empty() {
            return None;
        }
        let done = assets
            .iter()
            .filter(|asset| asset.status == AssetStatus::Done)
            .count();
        let position = self
            .state
            .selected_asset()
            .and_then(|id| assets.iter().position(|asset| asset.id == id))
            .map(|index| format!("{} of {}", index + 1, assets.len()))
            .unwrap_or_else(|| format!("{} media", assets.len()));
        let to_review = self.state.review_queue_len();
        Some(if to_review == 0 {
            format!("{position} · {done} done")
        } else {
            format!("{position} · {done} done · {to_review} to review")
        })
    }

    pub(super) fn current_annotation_summary(&self) -> Option<String> {
        let asset = self.selected_asset()?;
        let count = self.annotations_for_asset(asset).len();
        Some(format!("{count} annotation{}", plural(count)))
    }
}

/// A compact icon button that dispatches `action`, with its shortcut in the tooltip.
fn action_button<A: Action>(
    id: &'static str,
    icon: IconName,
    tooltip: &'static str,
    action: A,
) -> Button {
    Button::new(id)
        .xsmall()
        .ghost()
        .icon(Icon::new(icon))
        .tooltip_with_action(tooltip, &action, Some(actions::CONTEXT))
        .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
}

/// How far a model run is, with a button to stop it.
fn model_progress(progress: ModelProgress) -> impl IntoElement {
    let fraction = if progress.total == 0 {
        0.0
    } else {
        progress.done as f32 / progress.total as f32
    };
    div()
        .id("model-progress")
        .flex()
        .items_center()
        .gap_2()
        .child(Icon::new(gpui_kit_assets::IconName::Sparkles).xsmall())
        .child(format!(
            "Model {}/{} · {} suggestion{}",
            progress.done,
            progress.total,
            progress.added,
            plural(progress.added)
        ))
        .child(
            Progress::new("model-progress-bar")
                .w(px(96.0))
                .value(fraction * 100.0),
        )
        .child(
            Button::new("model-stop")
                .xsmall()
                .ghost()
                .icon(Icon::new(gpui_kit_assets::IconName::CircleStop))
                .tooltip("Stop after the current image")
                .on_click(|_, window, cx| window.dispatch_action(StopModelRun.boxed_clone(), cx)),
        )
}
