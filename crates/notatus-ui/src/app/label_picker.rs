//! Searchable active-label picker that creates labels from the typed name.

use super::*;
use gpui::Task;
use gpui_component::IndexPath;
use gpui_component::combobox::{Caret, Combobox, ComboboxEvent, ComboboxState};
use gpui_component::searchable_list::{SearchableListDelegate, SearchableListItem};

/// What choosing a row does: use an existing label, or create one by name.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum LabelChoice {
    Existing(LabelId),
    Create(SharedString),
}

#[derive(Clone)]
pub(super) struct LabelOption {
    choice: LabelChoice,
    title: SharedString,
    color: SharedString,
}

impl SearchableListItem for LabelOption {
    type Value = LabelChoice;

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn value(&self) -> &LabelChoice {
        &self.choice
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let swatch = match self.choice {
            LabelChoice::Existing(_) => {
                super::helpers::label_swatch(&self.color, false).into_any_element()
            }
            LabelChoice::Create(_) => Icon::new(IconName::Plus).xsmall().into_any_element(),
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(swatch)
            .child(self.title.clone())
    }

    fn matches(&self, _query: &str) -> bool {
        // Filtering happens in `perform_search`, which also adds the create row.
        true
    }
}

/// Labels filtered by the query, plus a create row for unknown names.
#[derive(Default)]
pub(super) struct LabelOptions {
    all: Vec<LabelOption>,
    visible: Vec<LabelOption>,
}

impl LabelOptions {
    pub(super) fn new(labels: &[Label]) -> Self {
        let all: Vec<_> = labels
            .iter()
            .map(|label| LabelOption {
                choice: LabelChoice::Existing(label.id),
                title: label.name.clone().into(),
                color: label
                    .color
                    .clone()
                    .unwrap_or_else(|| DEFAULT_LABEL_COLOR.to_string())
                    .into(),
            })
            .collect();
        Self {
            visible: all.clone(),
            all,
        }
    }

    fn filter(&mut self, query: &str) {
        let query = query.trim();
        let needle = query.to_lowercase();
        self.visible = self
            .all
            .iter()
            .filter(|option| option.title.to_lowercase().contains(&needle))
            .cloned()
            .collect();
        let exact = self
            .all
            .iter()
            .any(|option| option.title.to_lowercase() == needle);
        if !query.is_empty() && !exact {
            let name: SharedString = query.to_string().into();
            self.visible.push(LabelOption {
                title: format!("Create \"{name}\"").into(),
                choice: LabelChoice::Create(name),
                color: DEFAULT_LABEL_COLOR.into(),
            });
        }
    }
}

impl SearchableListDelegate for LabelOptions {
    type Item = LabelOption;

    fn items_count(&self, _: usize) -> usize {
        self.visible.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&LabelOption> {
        self.visible.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        LabelOption: SearchableListItem<Value = V>,
        V: PartialEq,
    {
        self.visible
            .iter()
            .position(|option| option.value() == value)
            .map(|row| IndexPath::default().row(row))
    }

    fn perform_search(&mut self, query: &str, _: &mut Window, _: &mut App) -> Task<()> {
        self.filter(query);
        Task::ready(())
    }
}

pub(super) type LabelPickerState = ComboboxState<LabelOptions>;

impl NotatusWindow {
    pub(super) fn new_label_picker(
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (gpui::Entity<LabelPickerState>, Subscription) {
        let picker = cx.new(|cx| {
            ComboboxState::new(LabelOptions::default(), Vec::new(), window, cx).searchable(true)
        });
        let subscription = cx.subscribe_in(
            &picker,
            window,
            |this, _, event: &ComboboxEvent<LabelOptions>, window, cx| {
                let values = match event {
                    ComboboxEvent::Change(values) => values,
                    // Closing may leave a stale query or a create row behind.
                    ComboboxEvent::Confirm(_) => {
                        this.label_picker_synced = None;
                        this.sync_label_picker(window, cx);
                        return;
                    }
                };
                match values.first() {
                    Some(LabelChoice::Existing(label_id)) => {
                        this.choose_label(*label_id, window, cx)
                    }
                    Some(LabelChoice::Create(name)) => {
                        this.create_label_named(name.to_string(), window, cx)
                    }
                    None => {}
                }
            },
        );
        (picker, subscription)
    }

    /// Creates a label from a typed name and makes it active, relabeling the
    /// selected annotation like choosing an existing label does.
    fn create_label_named(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let label_id = commands::add_colored_label(&mut self.state, Some(name));
        self.status_message = Some("Created label".to_string());
        self.choose_label(label_id, window, cx);
    }

    /// Refreshes the picker's options and selection from the dataset. Runs
    /// after changes, not every frame, since it resets the search text.
    pub(super) fn sync_label_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = (self.state.revision(), self.state.selected_label);
        if self.label_picker_synced == Some(key) {
            return;
        }
        self.label_picker_synced = Some(key);
        let options = LabelOptions::new(&self.state.dataset.labels);
        let selected = self.state.selected_label.map(LabelChoice::Existing);
        self.label_picker.update(cx, |picker, cx| {
            picker.set_items(options, window, cx);
            match selected {
                Some(choice) => picker.set_selected_values(&[choice], window, cx),
                None => picker.clear_selection(cx),
            }
        });
    }

    /// Focuses the picker and opens its list, so typing searches labels.
    pub(super) fn open_label_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.label_picker
            .update(cx, |picker, cx| picker.focus(window, cx));
        // gpui-base's `ui::SelectDown` opens a closed combobox. It isn't
        // re-exported, so build it by name.
        if let Ok(open) = cx.build_action("ui::SelectDown", None) {
            window.dispatch_action(open, cx);
        }
    }

    pub(super) fn label_picker(&self) -> impl IntoElement {
        Combobox::new(&self.label_picker)
            .small()
            .w(px(168.0))
            .menu_width(px(220.0))
            .placeholder("Label")
            .search_placeholder("Find or create a label")
            .render_trigger(|trigger, _, cx| {
                let selected = trigger.selection().first().map(|(_, option)| option);
                let muted = cx.theme().muted_foreground;
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap_2()
                    .map(|row| match selected {
                        Some(option) => row
                            .child(super::helpers::label_swatch(&option.color, false))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .child(option.title.clone()),
                            ),
                        None => row.child(div().flex_1().text_color(muted).child("Label")),
                    })
                    .child(Caret::new(trigger.size()).text_color(muted))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> LabelOptions {
        let mut dataset = notatus_core::Dataset::new("demo");
        dataset.add_label("Person");
        dataset.add_label("Car");
        LabelOptions::new(&dataset.labels)
    }

    fn titles(options: &LabelOptions) -> Vec<String> {
        options
            .visible
            .iter()
            .map(|o| o.title.to_string())
            .collect()
    }

    #[test]
    fn filters_labels_and_offers_to_create_unknown_names() {
        let mut options = options();

        options.filter("ca");
        assert_eq!(titles(&options), ["Car", "Create \"ca\""]);

        options.filter("car");
        assert_eq!(titles(&options), ["Car"]);

        options.filter("  Truck ");
        assert_eq!(titles(&options), ["Create \"Truck\""]);
        assert_eq!(
            options.visible[0].choice,
            LabelChoice::Create("Truck".into())
        );

        options.filter("");
        assert_eq!(titles(&options), ["Person", "Car"]);
    }
}
