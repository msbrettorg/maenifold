use eframe::egui;
use eframe::egui::{Color32, RichText};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::ui_task_data::TaskDetail;

pub(crate) fn show_task_detail_window(
    ctx: &egui::Context,
    detail_open: &mut bool,
    detail: Option<&TaskDetail>,
    markdown_cache: &mut CommonMarkCache,
) {
    let Some(detail) = detail else {
        return;
    };
    let pipeline_label = detail
        .entry
        .pipeline
        .as_deref()
        .unwrap_or("unbound")
        .to_uppercase();
    egui::Window::new(format!("{} {}", detail.entry.task_id, detail.entry.title))
        .open(detail_open)
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
                CommonMarkViewer::new().show(ui, markdown_cache, &detail.markdown);
            });
        });
}
