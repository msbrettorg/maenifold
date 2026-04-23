use crate::task_repository::{TaskPathSet, TaskRepository};
use crate::ui_filtering::*;
use crate::ui_progress::*;
use crate::ui_task_data::*;
use eframe::egui;
use eframe::egui::{Color32, RichText};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use egui_extras::{Column, TableBuilder};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
const READ_MODEL_POLL_INTERVAL: Duration = Duration::from_secs(2);
#[derive(Debug)]
pub struct GateUiWindow;
impl GateUiWindow {
    pub fn run(root: PathBuf) -> i32 {
        let initial_read_model_timestamp = read_model_timestamp(&root);
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
                .with_inner_size([900.0, 820.0])
                .with_min_inner_size([700.0, 640.0]),
            ..Default::default()
        };
        let app_root = root.clone();
        let result = eframe::run_native(
            "Gate UI",
            options,
            Box::new(move |_cc| {
                Ok(Box::new(GateUiApp::new(
                    app_root,
                    entries,
                    initial_read_model_timestamp,
                )))
            }),
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
#[allow(dead_code)]
pub(crate) struct GateUiApp {
    pub(crate) root: PathBuf,
    pub(crate) tasks: Vec<TaskEntry>,
    pub(crate) read_model_timestamp: Option<std::time::SystemTime>,
    pub(crate) selected: Option<TaskDetail>,
    pub(crate) detail_open: bool,
    pub(crate) markdown_cache: CommonMarkCache,
    pub(crate) search_text: String,
    pub(crate) process_filter: FilterValue,
    pub(crate) state_filter: FilterValue,
    pub(crate) location_filter: FilterValue,
    pub(crate) issues_only: bool,
    pub(crate) sort_state: SortState,
}
impl GateUiApp {
    fn new(
        root: PathBuf,
        tasks: Vec<TaskEntry>,
        read_model_timestamp: Option<std::time::SystemTime>,
    ) -> Self {
        Self {
            root,
            tasks,
            read_model_timestamp,
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
        let task_path = TaskPathSet {
            task_id: task.task_id.clone(),
            path: task.path.clone(),
            location: task.location.clone(),
        };
        match TaskRepository::canonical_task_markdown_path(&self.root, &task_path)
            .and_then(|path| fs::read_to_string(path).map_err(Into::into))
        {
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
    fn refresh_tasks_if_needed(&mut self) {
        let Some(current_timestamp) = read_model_timestamp(&self.root) else {
            return;
        };
        let should_reload = match self.read_model_timestamp {
            Some(previous_timestamp) => current_timestamp > previous_timestamp,
            None => true,
        };
        if !should_reload {
            return;
        }
        match load_tasks(&self.root) {
            Ok(tasks) => {
                self.tasks = tasks;
                self.read_model_timestamp = Some(current_timestamp);
                self.reset_stale_filters();
            }
            Err(err) => {
                eprintln!("gate UI failed to refresh tasks: {err}");
            }
        }
    }
    fn reset_stale_filters(&mut self) {
        self.process_filter
            .reset_if_missing(&process_values(&self.tasks));
        self.state_filter
            .reset_if_missing(&state_values(&self.tasks));
        self.location_filter
            .reset_if_missing(&location_values(&self.tasks));
    }
}
impl eframe::App for GateUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(READ_MODEL_POLL_INTERVAL);
        self.refresh_tasks_if_needed();
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
#[cfg(test)]
#[path = "ui_window_tests.rs"]
mod tests;
