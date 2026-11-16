//! The review queue: stepping through drafts across all media, and the
//! filter that narrows it to one label or to unsure model suggestions.

use crate::app::actions::{self, NextDraft, PreviousDraft};
use crate::app::helpers::{counted_section_title, plural};
use crate::app::right_dock::Labels;
use crate::app::window::NotatusWindow;
use crate::{Direction, ReviewFilter};
use gpui::prelude::*;
use gpui::{Context, div};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::{Disableable as _, Icon, IconName, Sizable as _};
use notatus_core::{LabelId, ReviewState};

/// Confidences the filter offers, from 0 to 1.
const CONFIDENCE_THRESHOLDS: [f32; 4] = [0.9, 0.75, 0.5, 0.25];

impl NotatusWindow {
    /// Selects the next or previous draft in the review queue, moving to its
    /// media when it is on another one.
    pub(super) fn step_review_queue(&mut self, direction: Direction, cx: &mut Context<Self>) {
        let target = self
            .state
            .adjacent_review_draft(direction)
            .and_then(|annotation_id| {
                self.state
                    .dataset()
                    .annotations
                    .iter()
                    .find(|annotation| annotation.id == annotation_id)
                    .map(|annotation| (annotation_id, annotation.asset_id))
            });
        match target {
            Some((annotation_id, asset_id)) => {
                self.go_to_media(asset_id, cx);
                self.select_annotation(Some(annotation_id), cx);
                self.status_message = Some(self.review_queue_summary());
            }
            None if self.state.review_queue_len() == 0 => {
                self.status_message = Some(self.review_queue_summary());
            }
            None => self.status_message = Some("No other draft to review".to_string()),
        }
        cx.notify();
    }

    pub(super) fn set_review_filter(&mut self, filter: ReviewFilter, cx: &mut Context<Self>) {
        self.state.set_review_filter(filter);
        self.status_message = Some(self.review_queue_summary());
        cx.notify();
    }

    /// How much the queue has left, noting when the filter hides drafts.
    pub(super) fn review_queue_summary(&self) -> String {
        let count = self.state.review_queue_len();
        let filtered = if self.state.review_filter().is_active() {
            " matching the filter"
        } else {
            ""
        };
        match count {
            0 => format!("Nothing left to review{filtered}"),
            _ => format!("{count} draft{} to review{filtered}", plural(count)),
        }
    }

    /// The queue's count, its previous and next buttons, and its filter.
    pub(super) fn review_queue_section(
        &self,
        labels: &Labels,
        view: gpui::WeakEntity<NotatusWindow>,
    ) -> impl IntoElement {
        let filter = self.state.review_filter();
        let count = self.state.review_queue_len();
        let label_name = filter
            .label
            .and_then(|id| labels.iter().find(|label| label.id == id))
            .map(|label| label.name.clone())
            .unwrap_or_else(|| "All labels".to_string());

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(counted_section_title("To review", count))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(queue_button(
                                "review-previous",
                                IconName::ChevronLeft,
                                "Previous draft to review",
                                &PreviousDraft,
                                count == 0,
                            ))
                            .child(queue_button(
                                "review-next",
                                IconName::ChevronRight,
                                "Next draft to review",
                                &NextDraft,
                                count == 0,
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(label_filter_menu(
                        label_name,
                        labels.clone(),
                        filter.label,
                        view.clone(),
                    ))
                    .child(confidence_filter_menu(filter.below_confidence, view)),
            )
    }

    /// Reviews the selected annotation, or the first one on this media when
    /// none is selected. Accepting or rejecting then selects the next draft
    /// in the review queue on the same media, so a review pass is one key per
    /// shape. It stays on the media when none is left there, so moving on
    /// is a choice: N goes to the next draft on other media.
    pub(super) fn review_selection(&mut self, review_state: ReviewState, cx: &mut Context<Self>) {
        let target = self
            .state
            .selected_annotation()
            .or_else(|| self.state.adjacent_annotation(Direction::Next));
        let Some(annotation_id) = target else {
            self.status_message = Some("No annotation to review".to_string());
            cx.notify();
            return;
        };
        if let Err(error) = self
            .state
            .set_annotation_review_state(annotation_id, review_state)
        {
            self.status_message = Some(error.to_string());
            cx.notify();
            return;
        }
        let done = match review_state {
            ReviewState::Accepted => "Accepted annotation",
            ReviewState::Rejected => "Rejected annotation",
            ReviewState::Reviewed => "Marked reviewed",
            ReviewState::Draft => "Reset to draft",
        };
        let next = (review_state != ReviewState::Draft)
            .then(|| self.state.next_draft_annotation(annotation_id))
            .flatten();
        if let Some(next) = next {
            self.select_annotation(Some(next), cx);
        }
        self.status_message = Some(match (review_state, next) {
            (ReviewState::Draft, _) | (_, Some(_)) => done.to_string(),
            (_, None) => match self.state.review_queue_len() {
                0 => format!("{done}. {}", self.review_queue_summary()),
                left => format!(
                    "{done}. No drafts left on this media; press N for the next one ({left} left)"
                ),
            },
        });
        cx.notify();
    }
}

fn label_filter_menu(
    button_label: String,
    labels: Labels,
    current: Option<LabelId>,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    Button::new("review-label-filter")
        .xsmall()
        .outline()
        .label(button_label)
        .dropdown_caret(true)
        .tooltip("Review only one label")
        .dropdown_menu(move |menu, _, _| {
            let choices = std::iter::once((None, "All labels".to_string())).chain(
                labels
                    .iter()
                    .map(|label| (Some(label.id), label.name.clone())),
            );
            let mut menu = menu;
            for (label, name) in choices {
                let view = view.clone();
                menu = menu.item(PopupMenuItem::new(name).checked(label == current).on_click(
                    move |_, _, cx| {
                        let _ = view.update(cx, |notatus, cx| {
                            let filter = ReviewFilter {
                                label,
                                ..notatus.state.review_filter()
                            };
                            notatus.set_review_filter(filter, cx);
                        });
                    },
                ));
            }
            menu
        })
}

fn confidence_filter_menu(
    current: Option<f32>,
    view: gpui::WeakEntity<NotatusWindow>,
) -> impl IntoElement {
    Button::new("review-confidence-filter")
        .xsmall()
        .outline()
        .label(confidence_filter_label(current))
        .dropdown_caret(true)
        .tooltip("Review only model suggestions the model was unsure of")
        .dropdown_menu(move |menu, _, _| {
            let choices = std::iter::once(None).chain(CONFIDENCE_THRESHOLDS.into_iter().map(Some));
            let mut menu = menu;
            for threshold in choices {
                let view = view.clone();
                menu = menu.item(
                    PopupMenuItem::new(confidence_filter_label(threshold))
                        .checked(threshold == current)
                        .on_click(move |_, _, cx| {
                            let _ = view.update(cx, |notatus, cx| {
                                let filter = ReviewFilter {
                                    below_confidence: threshold,
                                    ..notatus.state.review_filter()
                                };
                                notatus.set_review_filter(filter, cx);
                            });
                        }),
                );
            }
            menu
        })
}

fn queue_button(
    id: &'static str,
    icon: IconName,
    tooltip: &'static str,
    action: &'static dyn gpui::Action,
    disabled: bool,
) -> Button {
    Button::new(id)
        .xsmall()
        .ghost()
        .icon(Icon::new(icon))
        .disabled(disabled)
        .tooltip_with_action(tooltip, action, Some(actions::CONTEXT))
        .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
}

fn confidence_filter_label(threshold: Option<f32>) -> String {
    match threshold {
        None => "Any confidence".to_string(),
        Some(threshold) => format!("Below {:.0}%", threshold * 100.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_choices_read_as_percentages() {
        assert_eq!(confidence_filter_label(None), "Any confidence");
        assert_eq!(confidence_filter_label(Some(0.75)), "Below 75%");
    }
}
