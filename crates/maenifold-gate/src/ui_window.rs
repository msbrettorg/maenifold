use std::path::PathBuf;

use eframe::egui;
use eframe::egui::{Color32, RichText};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use egui_extras::{Column, TableBuilder};
use serde::Deserialize;

use crate::cli::{GateContext, StatusArgs};
use crate::gate_tool_service::GateToolService;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct TaskEntry {
    task_id: String,
    title: String,
    pipeline: Option<String>,
    state: String,
    #[serde(default)]
    stages: Vec<String>,
    location: String,
    attempts: u32,
    path: PathBuf,
    #[serde(default)]
    issue: String,
}

#[derive(Debug, Clone)]
struct TaskDetail {
    entry: TaskEntry,
    markdown: String,
}

#[derive(Debug)]
pub struct GateUiWindow;

impl GateUiWindow {
    pub fn run(root: PathBuf) -> i32 {
        let entries = match load_tasks(&root) {
            Ok(entries) => entries,
            Err(err) => {
                eprintln!("gate UI failed to load tasks: {err}");
                return 1;
            }
        };
        eprintln!(
            "gate UI loaded {} tasks from {}",
            entries.len(),
            root.display()
        );

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Gate UI")
                .with_inner_size([600.0, 820.0])
                .with_min_inner_size([520.0, 640.0]),
            ..Default::default()
        };

        let app_root = root.clone();
        let result = eframe::run_native(
            "Gate UI",
            options,
            Box::new(move |_cc| Ok(Box::new(GateUiApp::new(app_root, entries)))),
        );

        match result {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("gate UI failed to start: {err}");
                1
            }
        }
    }
}

fn load_tasks(root: &PathBuf) -> Result<Vec<TaskEntry>, String> {
    let value = GateToolService::status(StatusArgs {
        context: GateContext {
            root: root.clone(),
            requested_pipeline_name: None,
            json_output: false,
        },
    })
    .map_err(|e| e.to_string())?;

    let entries: Vec<TaskEntry> =
        serde_json::from_value(value).map_err(|e| format!("failed to parse task data: {e}"))?;
    Ok(entries)
}

#[allow(dead_code)]
struct GateUiApp {
    root: PathBuf,
    tasks: Vec<TaskEntry>,
    selected: Option<TaskDetail>,
    detail_open: bool,
    markdown_cache: CommonMarkCache,
    search_text: String,
    process_filter: FilterValue,
    state_filter: FilterValue,
    location_filter: FilterValue,
    issues_only: bool,
    sort_state: SortState,
}

impl GateUiApp {
    fn new(root: PathBuf, tasks: Vec<TaskEntry>) -> Self {
        Self {
            root,
            tasks,
            selected: None,
            detail_open: false,
            markdown_cache: CommonMarkCache::default(),
            search_text: String::new(),
            process_filter: FilterValue::all(),
            state_filter: FilterValue::all(),
            location_filter: FilterValue::all(),
            issues_only: false,
            sort_state: SortState::default(),
        }
    }

    fn open_task(&mut self, task: &TaskEntry) {
        match std::fs::read_to_string(&task.path) {
            Ok(markdown) => {
                self.selected = Some(TaskDetail {
                    entry: task.clone(),
                    markdown,
                });
                self.detail_open = true;
            }
            Err(err) => {
                eprintln!(
                    "failed to open gate task detail for {}: {err}",
                    task.task_id
                );
            }
        }
    }

    fn visible_tasks(&self) -> Vec<TaskEntry> {
        let mut tasks: Vec<_> = self
            .tasks
            .iter()
            .filter(|task| task_matches_filters(task, self))
            .cloned()
            .collect();
        sort_tasks(&mut tasks, self.sort_state);
        tasks
    }
}

impl eframe::App for GateUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Gate");
            ui.label(
                RichText::new(summary_text(&self.tasks, &self.visible_tasks()))
                    .family(egui::FontFamily::Monospace)
                    .small()
                    .color(Color32::GRAY),
            );
            toolbar(ui, self);
            ui.separator();

            let tasks = self.visible_tasks();
            let mut task_to_open: Option<TaskEntry> = None;

            TableBuilder::new(ui)
                .id_salt("gate-task-table")
                .striped(true)
                .column(Column::exact(64.0))
                .column(Column::remainder().at_least(120.0).clip(true))
                .column(Column::exact(64.0))
                .column(Column::exact(180.0).clip(true))
                .header(22.0, |mut header| {
                    header.col(|ui| {
                        if ui
                            .button(header_text("ID", self.sort_state, SortColumn::Id))
                            .clicked()
                        {
                            self.sort_state.toggle(SortColumn::Id);
                        }
                    });
                    header.col(|ui| {
                        if ui
                            .button(header_text("Name", self.sort_state, SortColumn::Name))
                            .clicked()
                        {
                            self.sort_state.toggle(SortColumn::Name);
                        }
                    });
                    header.col(|ui| {
                        if ui
                            .button(header_text("Process", self.sort_state, SortColumn::Process))
                            .clicked()
                        {
                            self.sort_state.toggle(SortColumn::Process);
                        }
                    });
                    header.col(|ui| {
                        if ui
                            .button(header_text(
                                "Progress",
                                self.sort_state,
                                SortColumn::Progress,
                            ))
                            .clicked()
                        {
                            self.sort_state.toggle(SortColumn::Progress);
                        }
                    });
                })
                .body(|body| {
                    body.rows(48.0, tasks.len(), |mut row| {
                        let task = &tasks[row.index()];

                        row.col(|ui| {
                            let response = ui.selectable_label(false, &task.task_id);
                            if response.clicked() {
                                task_to_open = Some(task.clone());
                            }
                            response.on_hover_text(task.path.display().to_string());
                        });

                        row.col(|ui| {
                            let response = ui.selectable_label(false, &task.title);
                            if response.clicked() {
                                task_to_open = Some(task.clone());
                            }
                            if !task.issue.is_empty() {
                                response.on_hover_text(format!(
                                    "{}\n{}",
                                    task.path.display(),
                                    task.issue
                                ));
                            } else {
                                response.on_hover_text(task.path.display().to_string());
                            }
                        });

                        row.col(|ui| {
                            ui.label(process_text(task));
                        });

                        row.col(|ui| {
                            ui.vertical(|ui| {
                                render_progress_dots(ui, task);
                                ui.label(RichText::new(&task.state).small().color(Color32::GRAY));
                            });
                        });
                    });
                });

            if let Some(task) = task_to_open {
                self.open_task(&task);
            }
        });

        if self.detail_open {
            let mut is_open = self.detail_open;
            if let Some(detail) = self.selected.clone() {
                let pipeline_label = detail
                    .entry
                    .pipeline
                    .as_deref()
                    .unwrap_or("unbound")
                    .to_uppercase();
                egui::Window::new(format!("{} {}", detail.entry.task_id, detail.entry.title))
                    .open(&mut is_open)
                    .default_width(420.0)
                    .default_height(760.0)
                    .show(ctx, |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{}  {}  {}",
                                detail.entry.task_id,
                                pipeline_label,
                                detail.entry.state.to_uppercase(),
                            ))
                            .family(egui::FontFamily::Monospace)
                            .small()
                            .color(Color32::GRAY),
                        );
                        ui.label(RichText::new(&detail.entry.title).strong());
                        ui.label(
                            RichText::new(detail.entry.path.display().to_string())
                                .small()
                                .color(Color32::GRAY),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            CommonMarkViewer::new().show(
                                ui,
                                &mut self.markdown_cache,
                                &detail.markdown,
                            );
                        });
                    });
            }
            self.detail_open = is_open;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortColumn {
    Id,
    Name,
    Process,
    Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct SortState {
    column: Option<SortColumn>,
    direction: SortDirection,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Asc
    }
}

impl SortState {
    fn toggle(&mut self, column: SortColumn) {
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
struct FilterValue(Option<String>);

impl FilterValue {
    fn all() -> Self {
        Self(None)
    }

    fn label(&self) -> &str {
        match &self.0 {
            Some(value) => value.as_str(),
            None => "All",
        }
    }

    fn matches(&self, value: &str) -> bool {
        match &self.0 {
            Some(expected) => expected == value,
            None => true,
        }
    }
}

fn toolbar(ui: &mut egui::Ui, app: &mut GateUiApp) {
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

fn filter_combo(ui: &mut egui::Ui, label: &str, filter: &mut FilterValue, values: Vec<String>) {
    egui::ComboBox::from_label(label)
        .selected_text(filter.label())
        .show_ui(ui, |ui| {
            ui.selectable_value(filter, FilterValue::all(), "All");
            for value in values {
                ui.selectable_value(filter, FilterValue(Some(value.clone())), value);
            }
        });
}

fn process_values(tasks: &[TaskEntry]) -> Vec<String> {
    sorted_unique_values(tasks.iter().map(|task| {
        task.pipeline
            .clone()
            .unwrap_or_else(|| "unbound".to_string())
    }))
}

fn state_values(tasks: &[TaskEntry]) -> Vec<String> {
    sorted_unique_values(tasks.iter().map(|task| task.state.clone()))
}

fn location_values(tasks: &[TaskEntry]) -> Vec<String> {
    sorted_unique_values(tasks.iter().map(|task| task.location.clone()))
}

fn sorted_unique_values(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut values: Vec<_> = values.collect();
    values.sort();
    values.dedup();
    values
}

fn header_text(label: &str, sort_state: SortState, column: SortColumn) -> RichText {
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

fn task_matches_filters(task: &TaskEntry, app: &GateUiApp) -> bool {
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

fn sort_tasks(tasks: &mut [TaskEntry], sort_state: SortState) {
    match sort_state.column {
        None => {
            tasks.sort_by(default_task_order);
        }
        Some(column) => {
            tasks.sort_by(|left, right| {
                let ordering = match column {
                    SortColumn::Id => left.task_id.cmp(&right.task_id),
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

fn default_task_order(left: &TaskEntry, right: &TaskEntry) -> std::cmp::Ordering {
    location_rank(&left.location)
        .cmp(&location_rank(&right.location))
        .then(left.state.cmp(&right.state))
        .then(left.task_id.cmp(&right.task_id))
}

fn location_rank(location: &str) -> u8 {
    match location {
        "todo" => 0,
        "done" => 1,
        _ => 2,
    }
}

fn summary_text(tasks: &[TaskEntry], visible: &[TaskEntry]) -> String {
    let todo = tasks.iter().filter(|task| task.location == "todo").count();
    let done = tasks.iter().filter(|task| task.location == "done").count();
    format!(
        "{} shown  {} total  {} todo  {} done",
        visible.len(),
        tasks.len(),
        todo,
        done
    )
}

fn process_text(task: &TaskEntry) -> String {
    let pipeline = task.pipeline.as_deref().unwrap_or("unbound");
    if task.location == "done" {
        format!("{} done", pipeline.to_uppercase())
    } else {
        pipeline.to_uppercase()
    }
}

fn state_symbol(task: &TaskEntry) -> &'static str {
    match task.state.as_str() {
        "done" | "completed" => "●",
        "pending" => "○",
        "error" | "failed" => "✗",
        _ => "◉",
    }
}

fn completed_progress_color() -> Color32 {
    Color32::from_rgb(110, 251, 190)
}

fn active_progress_color() -> Color32 {
    Color32::from_rgb(255, 193, 7)
}

fn failed_progress_color() -> Color32 {
    Color32::from_rgb(255, 107, 107)
}

fn pending_progress_color() -> Color32 {
    Color32::from_gray(110)
}

fn is_failed_state(state: &str) -> bool {
    matches!(state, "error" | "failed")
}

fn progress_color(task: &TaskEntry) -> Color32 {
    if task.location == "done" {
        completed_progress_color()
    } else if is_failed_state(&task.state) {
        failed_progress_color()
    } else if task.state == "pending" {
        pending_progress_color()
    } else {
        active_progress_color()
    }
}

fn render_progress_dots(ui: &mut egui::Ui, task: &TaskEntry) {
    if task.stages.is_empty() {
        ui.label(
            RichText::new(state_symbol(task))
                .family(egui::FontFamily::Monospace)
                .color(progress_color(task)),
        );
        return;
    }

    ui.horizontal(|ui| {
        for stage in &task.stages {
            let (symbol, color) = stage_dot(stage, task);
            let response = ui.label(
                RichText::new(symbol)
                    .family(egui::FontFamily::Monospace)
                    .color(color),
            );
            response.on_hover_text(stage.replace('-', " "));
        }

        if is_failed_state(&task.state) {
            ui.label(
                RichText::new("✗")
                    .family(egui::FontFamily::Monospace)
                    .color(failed_progress_color()),
            )
            .on_hover_text(task.state.replace('-', " "));
        }
    });
}

fn stage_dot(stage: &str, task: &TaskEntry) -> (&'static str, Color32) {
    match stage_progress(stage, task) {
        StageProgress::Completed => ("●", completed_progress_color()),
        StageProgress::Active => ("◉", progress_color(task)),
        StageProgress::Pending => ("○", pending_progress_color()),
    }
}

fn stage_progress(stage: &str, task: &TaskEntry) -> StageProgress {
    if task.location == "done" {
        return StageProgress::Completed;
    }

    let Some(active_index) = task
        .stages
        .iter()
        .position(|candidate| candidate == &task.state)
    else {
        return StageProgress::Pending;
    };
    let stage_index = task
        .stages
        .iter()
        .position(|candidate| candidate == stage)
        .unwrap_or(active_index);

    if stage_index < active_index {
        StageProgress::Completed
    } else if stage_index == active_index {
        StageProgress::Active
    } else {
        StageProgress::Pending
    }
}

fn progress_rank(task: &TaskEntry) -> usize {
    if task.location == "done" {
        return task.stages.len() + 1;
    }

    task.stages
        .iter()
        .position(|stage| stage == &task.state)
        .map(|index| index + 1)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StageProgress {
    Completed,
    Active,
    Pending,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn filters_match_id_title_and_issue_toggle() {
        let mut app = GateUiApp::new(PathBuf::from("."), sample_tasks());
        app.search_text = "acp".to_string();
        assert_eq!(app.visible_tasks().len(), 1);

        app.search_text.clear();
        app.issues_only = true;
        let visible = app.visible_tasks();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].task_id, "T-2");
    }

    #[test]
    fn progress_sort_orders_todo_before_done() {
        let mut tasks = sample_tasks();
        sort_tasks(
            &mut tasks,
            SortState {
                column: Some(SortColumn::Progress),
                direction: SortDirection::Asc,
            },
        );
        let ids = tasks
            .iter()
            .map(|task| task.task_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["T-1", "T-2", "T-3"]);
    }

    #[test]
    fn stage_progress_marks_completed_active_and_pending_steps() {
        let task = TaskEntry {
            task_id: "T-4".to_string(),
            title: "Progress".to_string(),
            pipeline: Some("sdd".to_string()),
            state: "red-team".to_string(),
            stages: vec![
                "swe".to_string(),
                "red-team".to_string(),
                "blue-team".to_string(),
            ],
            location: "todo".to_string(),
            attempts: 0,
            path: PathBuf::from("./todo/T-4.md"),
            issue: String::new(),
        };

        assert_eq!(stage_progress("swe", &task), StageProgress::Completed);
        assert_eq!(stage_progress("red-team", &task), StageProgress::Active);
        assert_eq!(stage_progress("blue-team", &task), StageProgress::Pending);
    }

    #[test]
    fn progress_color_uses_stoplight_mapping() {
        let done = TaskEntry {
            location: "done".to_string(),
            state: "done".to_string(),
            ..sample_tasks()[0].clone()
        };
        let pending = TaskEntry {
            state: "pending".to_string(),
            ..sample_tasks()[0].clone()
        };
        let active = TaskEntry {
            state: "research".to_string(),
            ..sample_tasks()[1].clone()
        };
        let failed = TaskEntry {
            state: "error".to_string(),
            ..sample_tasks()[1].clone()
        };

        assert_eq!(progress_color(&done), completed_progress_color());
        assert_eq!(progress_color(&pending), pending_progress_color());
        assert_eq!(progress_color(&active), active_progress_color());
        assert_eq!(progress_color(&failed), failed_progress_color());
    }

    #[test]
    fn stage_dot_uses_stoplight_colors() {
        let task = TaskEntry {
            task_id: "T-5".to_string(),
            title: "Progress".to_string(),
            pipeline: Some("sdd".to_string()),
            state: "red-team".to_string(),
            stages: vec![
                "swe".to_string(),
                "red-team".to_string(),
                "blue-team".to_string(),
            ],
            location: "todo".to_string(),
            attempts: 0,
            path: PathBuf::from("./todo/T-5.md"),
            issue: String::new(),
        };

        assert_eq!(stage_dot("swe", &task), ("●", completed_progress_color()));
        assert_eq!(stage_dot("red-team", &task), ("◉", active_progress_color()));
        assert_eq!(
            stage_dot("blue-team", &task),
            ("○", pending_progress_color())
        );
    }

    fn sample_tasks() -> Vec<TaskEntry> {
        vec![
            TaskEntry {
                task_id: "T-1".to_string(),
                title: "ACP authentication".to_string(),
                pipeline: Some("sdd".to_string()),
                state: "pending".to_string(),
                stages: vec![
                    "swe".to_string(),
                    "red-team".to_string(),
                    "blue-team".to_string(),
                ],
                location: "todo".to_string(),
                attempts: 0,
                path: PathBuf::from("./todo/T-1.md"),
                issue: String::new(),
            },
            TaskEntry {
                task_id: "T-2".to_string(),
                title: "Gate sync issue".to_string(),
                pipeline: Some("gsd".to_string()),
                state: "research".to_string(),
                stages: vec![
                    "discover".to_string(),
                    "research".to_string(),
                    "deliver".to_string(),
                ],
                location: "todo".to_string(),
                attempts: 1,
                path: PathBuf::from("./todo/T-2.md"),
                issue: "Missing pipeline".to_string(),
            },
            TaskEntry {
                task_id: "T-3".to_string(),
                title: "Finished task".to_string(),
                pipeline: Some("sdd".to_string()),
                state: "done".to_string(),
                stages: vec![
                    "swe".to_string(),
                    "red-team".to_string(),
                    "blue-team".to_string(),
                ],
                location: "done".to_string(),
                attempts: 0,
                path: PathBuf::from("./done/T-3.md"),
                issue: String::new(),
            },
        ]
    }
}
