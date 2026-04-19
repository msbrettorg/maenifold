use eframe::egui;
use eframe::egui::{Color32, RichText};

use crate::ui_task_data::TaskEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StageProgress {
    Completed,
    Active,
    Failed,
    Pending,
}

pub(crate) fn summary_text(tasks: &[TaskEntry], visible: &[TaskEntry]) -> String {
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

pub(crate) fn process_text(task: &TaskEntry) -> String {
    let pipeline = task.pipeline.as_deref().unwrap_or("unbound");
    if task.location == "done" {
        format!("{} done", pipeline.to_uppercase())
    } else {
        pipeline.to_uppercase()
    }
}

pub(crate) fn state_symbol(task: &TaskEntry) -> &'static str {
    match task.state.as_str() {
        "done" | "completed" => "●",
        "pending" => "○",
        "error" | "failed" => "✗",
        _ => "◉",
    }
}

pub(crate) fn completed_progress_color() -> Color32 {
    Color32::from_rgb(110, 251, 190)
}

pub(crate) fn active_progress_color() -> Color32 {
    Color32::from_rgb(255, 193, 7)
}

pub(crate) fn failed_progress_color() -> Color32 {
    Color32::from_rgb(255, 107, 107)
}

pub(crate) fn pending_progress_color() -> Color32 {
    Color32::from_gray(110)
}

pub(crate) fn is_failed_state(state: &str) -> bool {
    matches!(state, "error" | "failed")
}

pub(crate) fn progress_color(task: &TaskEntry) -> Color32 {
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

pub(crate) fn render_progress_dots(ui: &mut egui::Ui, task: &TaskEntry) {
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

pub(crate) fn stage_dot(stage: &str, task: &TaskEntry) -> (&'static str, Color32) {
    match stage_progress(stage, task) {
        StageProgress::Completed => ("●", completed_progress_color()),
        StageProgress::Active => ("◉", progress_color(task)),
        StageProgress::Failed => ("●", failed_progress_color()),
        StageProgress::Pending => ("○", pending_progress_color()),
    }
}

pub(crate) fn stage_progress(stage: &str, task: &TaskEntry) -> StageProgress {
    if task.location == "done" {
        return StageProgress::Completed;
    }

    let stage_index = task.stages.iter().position(|candidate| candidate == stage);

    if is_failed_state(&task.state) {
        let Some(failed_index) = task
            .stages
            .iter()
            .rposition(|candidate| !matches!(candidate.as_str(), "done" | "completed"))
        else {
            return StageProgress::Pending;
        };

        return match stage_index.unwrap_or(failed_index) {
            index if index < failed_index => StageProgress::Completed,
            index if index == failed_index => StageProgress::Failed,
            _ => StageProgress::Pending,
        };
    }

    let Some(active_index) = task.stages.iter().position(|candidate| candidate == &task.state)
    else {
        return StageProgress::Pending;
    };
    let stage_index = stage_index.unwrap_or(active_index);

    if stage_index < active_index {
        StageProgress::Completed
    } else if stage_index == active_index {
        StageProgress::Active
    } else {
        StageProgress::Pending
    }
}

pub(crate) fn progress_rank(task: &TaskEntry) -> usize {
    if task.location == "done" {
        return task.stages.len() + 1;
    }

    task.stages
        .iter()
        .position(|stage| stage == &task.state)
        .map(|index| index + 1)
        .unwrap_or(0)
}
