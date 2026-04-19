use eframe::egui;
use eframe::egui::RichText;

use crate::task_id;
use crate::ui_progress::progress_rank;
use crate::ui_task_data::TaskEntry;
use crate::ui_window::GateUiApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortColumn {
    Id,
    Name,
    Process,
    Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SortState {
    pub(crate) column: Option<SortColumn>,
    pub(crate) direction: SortDirection,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Asc
    }
}

impl SortState {
    pub(crate) fn toggle(&mut self, column: SortColumn) {
        match self.column {
            Some(current) if current == column => {
                self.direction = match self.direction {
                    SortDirection::Asc => SortDirection::Desc,
                    SortDirection::Desc => SortDirection::Asc,
                };
            }
            _ => {
                self.column = Some(column);
                self.direction = SortDirection::Asc;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FilterValue(pub(crate) Option<String>);

impl FilterValue {
    pub(crate) fn all() -> Self {
        Self(None)
    }

    pub(crate) fn label(&self) -> &str {
        match &self.0 {
            Some(value) => value.as_str(),
            None => "All",
        }
    }

    pub(crate) fn matches(&self, value: &str) -> bool {
        match &self.0 {
            Some(expected) => expected == value,
            None => true,
        }
    }

    pub(crate) fn reset_if_missing(&mut self, values: &[String]) {
        if let Some(selected) = &self.0
            && !values.iter().any(|value| value == selected)
        {
            *self = Self::all();
        }
    }
}

pub(crate) fn toolbar(ui: &mut egui::Ui, app: &mut GateUiApp) {
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.search_text)
                .hint_text("Filter tasks")
                .desired_width(140.0),
        );
        filter_combo(
            ui,
            "Process",
            &mut app.process_filter,
            process_values(&app.tasks),
        );
        filter_combo(ui, "State", &mut app.state_filter, state_values(&app.tasks));
        filter_combo(
            ui,
            "Location",
            &mut app.location_filter,
            location_values(&app.tasks),
        );
        ui.checkbox(&mut app.issues_only, "Issues only");
    });
}

pub(crate) fn filter_combo(
    ui: &mut egui::Ui,
    label: &str,
    filter: &mut FilterValue,
    values: Vec<String>,
) {
    egui::ComboBox::from_label(label)
        .selected_text(filter.label())
        .show_ui(ui, |ui| {
            ui.selectable_value(filter, FilterValue::all(), "All");
            for value in values {
                ui.selectable_value(filter, FilterValue(Some(value.clone())), value);
            }
        });
}

pub(crate) fn process_values(tasks: &[TaskEntry]) -> Vec<String> {
    sorted_unique_values(tasks.iter().map(|task| {
        task.pipeline
            .clone()
            .unwrap_or_else(|| "unbound".to_string())
    }))
}

pub(crate) fn state_values(tasks: &[TaskEntry]) -> Vec<String> {
    sorted_unique_values(tasks.iter().map(|task| task.state.clone()))
}

pub(crate) fn location_values(tasks: &[TaskEntry]) -> Vec<String> {
    sorted_unique_values(tasks.iter().map(|task| task.location.clone()))
}

pub(crate) fn sorted_unique_values(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut values: Vec<_> = values.collect();
    values.sort();
    values.dedup();
    values
}

pub(crate) fn header_text(label: &str, sort_state: SortState, column: SortColumn) -> RichText {
    let suffix = match (sort_state.column, sort_state.direction) {
        (Some(current), SortDirection::Asc) if current == column => " ↑",
        (Some(current), SortDirection::Desc) if current == column => " ↓",
        _ => "",
    };
    RichText::new(format!("{label}{suffix}"))
        .family(egui::FontFamily::Monospace)
        .small()
        .strong()
}

pub(crate) fn task_matches_filters(task: &TaskEntry, app: &GateUiApp) -> bool {
    let search = app.search_text.trim().to_ascii_lowercase();
    let matches_search = search.is_empty()
        || task.task_id.to_ascii_lowercase().contains(&search)
        || task.title.to_ascii_lowercase().contains(&search)
        || task
            .path
            .display()
            .to_string()
            .to_ascii_lowercase()
            .contains(&search);

    let pipeline_str = task.pipeline.as_deref().unwrap_or("unbound");

    matches_search
        && app.process_filter.matches(pipeline_str)
        && app.state_filter.matches(&task.state)
        && app.location_filter.matches(&task.location)
        && (!app.issues_only || !task.issue.is_empty())
}

pub(crate) fn sort_tasks(tasks: &mut [TaskEntry], sort_state: SortState) {
    match sort_state.column {
        None => {
            tasks.sort_by(default_task_order);
        }
        Some(column) => {
            tasks.sort_by(|left, right| {
                let ordering = match column {
                    SortColumn::Id => task_id::natural_cmp(&left.task_id, &right.task_id),
                    SortColumn::Name => left.title.cmp(&right.title),
                    SortColumn::Process => left.pipeline.cmp(&right.pipeline),
                    SortColumn::Progress => progress_rank(left)
                        .cmp(&progress_rank(right))
                        .then(left.state.cmp(&right.state)),
                };

                let ordering = if ordering.is_eq() {
                    default_task_order(left, right)
                } else {
                    ordering
                };

                match sort_state.direction {
                    SortDirection::Asc => ordering,
                    SortDirection::Desc => ordering.reverse(),
                }
            });
        }
    }
}

pub(crate) fn default_task_order(left: &TaskEntry, right: &TaskEntry) -> std::cmp::Ordering {
    location_rank(&left.location)
        .cmp(&location_rank(&right.location))
        .then(left.state.cmp(&right.state))
        .then_with(|| task_id::natural_cmp(&left.task_id, &right.task_id))
}

pub(crate) fn location_rank(location: &str) -> u8 {
    match location {
        "todo" => 0,
        "done" => 1,
        _ => 2,
    }
}
