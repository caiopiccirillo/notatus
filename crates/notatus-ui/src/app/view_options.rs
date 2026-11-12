//! Canvas display options: overlay fill, label tags, crosshair, and focus.

use super::*;
use gpui_component::popover::Popover;
use gpui_component::slider::{Slider, SliderEvent, SliderState};
use gpui_component::switch::Switch;
use gpui_component::v_flex;

/// How annotations are drawn over the image. Display only, never saved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CanvasViewOptions {
    /// Fill alpha for shapes, from 0 (outline only) to 1.
    pub(super) fill_opacity: f32,
    /// Draw each shape's label name on it.
    pub(super) show_labels: bool,
    /// Draw guide lines through the pointer while drawing.
    pub(super) crosshair: bool,
    /// Fade everything except the selected annotation.
    pub(super) focus_selected: bool,
    /// Hide all annotations, to see the image underneath.
    pub(super) hide_annotations: bool,
}

impl Default for CanvasViewOptions {
    fn default() -> Self {
        Self {
            fill_opacity: 0.1,
            show_labels: true,
            crosshair: false,
            focus_selected: false,
            hide_annotations: false,
        }
    }
}

/// A boolean view option, so toggles share one code path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ViewToggle {
    Labels,
    Crosshair,
    FocusSelected,
    HideAnnotations,
}

impl ViewToggle {
    pub(super) const ALL: [Self; 4] = [
        Self::Labels,
        Self::Crosshair,
        Self::FocusSelected,
        Self::HideAnnotations,
    ];

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Labels => "Label names",
            Self::Crosshair => "Crosshair",
            Self::FocusSelected => "Focus selected",
            Self::HideAnnotations => "Hide annotations",
        }
    }

    fn slot(self, options: &mut CanvasViewOptions) -> &mut bool {
        match self {
            Self::Labels => &mut options.show_labels,
            Self::Crosshair => &mut options.crosshair,
            Self::FocusSelected => &mut options.focus_selected,
            Self::HideAnnotations => &mut options.hide_annotations,
        }
    }

    pub(super) fn get(self, options: &CanvasViewOptions) -> bool {
        let mut copy = *options;
        *self.slot(&mut copy)
    }

    pub(super) fn set(self, options: &mut CanvasViewOptions, value: bool) {
        *self.slot(options) = value;
    }
}

impl NotatusWindow {
    pub(super) fn new_opacity_slider(
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (gpui::Entity<SliderState>, Subscription) {
        let initial = CanvasViewOptions::default().fill_opacity * 100.0;
        let slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .step(5.0)
                .default_value(initial)
        });
        let subscription =
            cx.subscribe_in(&slider, window, |this, _, event: &SliderEvent, _, cx| {
                let SliderEvent::Change(value) = event else {
                    return;
                };
                this.view_options.fill_opacity = value.start() / 100.0;
                cx.notify();
            });
        (slider, subscription)
    }

    pub(super) fn toggle_view_option(&mut self, toggle: ViewToggle, cx: &mut Context<Self>) {
        let value = !toggle.get(&self.view_options);
        toggle.set(&mut self.view_options, value);
        if toggle == ViewToggle::Crosshair && !value {
            self.tools.pointer_image_pos = None;
        }
        let state = if value { "on" } else { "off" };
        self.status_message = Some(format!("{} {state}", toggle.title()));
        cx.notify();
    }

    /// Toolbar button that opens the display options.
    pub(super) fn view_options_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let slider = self.opacity_slider.clone();
        let icon = if self.view_options.hide_annotations {
            IconName::EyeOff
        } else {
            IconName::Eye
        };

        Popover::new("canvas-view-options")
            .anchor(gpui::Anchor::TopLeft)
            .trigger(
                Button::new("tool-view-options")
                    .small()
                    .icon(Icon::new(icon))
                    .tooltip("View options"),
            )
            .content(move |_, _, cx| {
                let options = view
                    .upgrade()
                    .map(|view| view.read(cx).view_options)
                    .unwrap_or_default();
                let muted = cx.theme().muted_foreground;
                let row = || div().flex().items_center().justify_between().gap_4();

                let mut panel = v_flex().w(px(240.0)).gap_3().child(
                    v_flex()
                        .gap_2()
                        .child(
                            row().child(div().text_sm().child("Fill")).child(
                                div()
                                    .text_sm()
                                    .text_color(muted)
                                    .child(format!("{:.0}%", options.fill_opacity * 100.0)),
                            ),
                        )
                        .child(Slider::new(&slider)),
                );
                for toggle in ViewToggle::ALL {
                    let view = view.clone();
                    panel = panel.child(
                        row().child(div().text_sm().child(toggle.title())).child(
                            Switch::new(SharedString::from(format!("view-{toggle:?}")))
                                .small()
                                .checked(toggle.get(&options))
                                .on_change(move |_, _, cx| {
                                    let _ = view.update(cx, |notatus, cx| {
                                        notatus.toggle_view_option(toggle, cx);
                                    });
                                }),
                        ),
                    );
                }
                panel
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_flip_only_their_own_option() {
        let mut options = CanvasViewOptions::default();
        for toggle in ViewToggle::ALL {
            let before = options;
            let value = !toggle.get(&options);
            toggle.set(&mut options, value);
            assert_eq!(toggle.get(&options), value);
            for other in ViewToggle::ALL.into_iter().filter(|other| *other != toggle) {
                assert_eq!(other.get(&options), other.get(&before));
            }
        }
    }
}
