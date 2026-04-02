use std::path::PathBuf;

use eframe::egui;
use eframe::egui::{Color32, RichText, Stroke};

use crate::board_data::{BoardTaskDetail, BoardTaskSummary, GateBoardData};

#[derive(Debug)]
pub struct GateUiWindow;

impl GateUiWindow {
    pub fn run(root: PathBuf) -> i32 {
        let tasks = match GateBoardData::list(&root) {
            Ok(tasks) => tasks,
            Err(err) => {
                eprintln!("gate UI failed to load tasks: {err}");
                return 1;
            }
        };
        eprintln!(
            "gate UI loaded {} tasks from {}",
            tasks.len(),
            root.display()
        );

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Gate UI")
                .with_inner_size([430.0, 820.0])
                .with_min_inner_size([360.0, 640.0]),
            ..Default::default()
        };

        let app_root = root.clone();
        let app_tasks = tasks;
        let result = eframe::run_native(
            "Gate UI",
            options,
            Box::new(move |_cc| Ok(Box::new(GateUiApp::new(app_root, app_tasks)))),
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

struct GateUiApp {
    root: PathBuf,
    tasks: Vec<BoardTaskSummary>,
    selected: Option<BoardTaskDetail>,
    detail_open: bool,
}

impl GateUiApp {
    fn new(root: PathBuf, tasks: Vec<BoardTaskSummary>) -> Self {
        Self {
            root,
            tasks,
            selected: None,
            detail_open: false,
        }
    }

    fn open_task(&mut self, task_id: &str) {
        match GateBoardData::detail(&self.root, task_id) {
            Ok(detail) => {
                self.selected = Some(detail);
                self.detail_open = true;
            }
            Err(err) => {
                eprintln!("failed to open gate task detail for {task_id}: {err}");
            }
        }
    }
}

impl eframe::App for GateUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let visuals = egui::Visuals::dark();
        ctx.set_visuals(visuals);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(surface())
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("READ ONLY PIPELINE VIEW")
                            .family(egui::FontFamily::Monospace)
                            .size(10.0)
                            .color(muted())
                            .strong(),
                    );
                    ui.add_space(2.0);
                    ui.heading(RichText::new("Gate").size(28.0).color(primary_text()));
                    render_summary_strip(ui, &self.tasks);
                    ui.add_space(12.0);

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let tasks = self.tasks.clone();
                            render_section(ui, "Todo", "todo", &tasks, self);
                            ui.add_space(14.0);
                            render_section(ui, "Done", "done", &tasks, self);
                        });
                });
            });

        if self.detail_open {
            if let Some(detail) = &mut self.selected {
                egui::Window::new(format!("{} · {}", detail.task.task_id, detail.task.title))
                    .open(&mut self.detail_open)
                    .default_width(410.0)
                    .default_height(760.0)
                    .show(ctx, |ui| {
                        ui.visuals_mut().panel_fill = panel();
                        render_task_header(ui, &detail.task);
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&detail.markdown)
                                            .monospace()
                                            .size(12.5)
                                            .color(primary_text()),
                                    )
                                    .selectable(true),
                                );
                            });
                    });
            }
        }
    }
}

fn render_summary_strip(ui: &mut egui::Ui, tasks: &[BoardTaskSummary]) {
    let todo = tasks.iter().filter(|task| task.location == "todo").count();
    let done = tasks.iter().filter(|task| task.location == "done").count();

    ui.horizontal_wrapped(|ui| {
        pill(ui, &format!("{} tasks", tasks.len()), Color32::from_rgb(160, 202, 255));
        pill(ui, &format!("{todo} todo"), Color32::from_rgb(160, 202, 255));
        pill(ui, &format!("{done} done"), Color32::from_rgb(110, 251, 190));
    });
}

fn render_section(ui: &mut egui::Ui, label: &str, location: &str, tasks: &[BoardTaskSummary], app: &mut GateUiApp) {
    let section_tasks: Vec<_> = tasks
        .iter()
        .filter(|task| task.location == location)
        .cloned()
        .collect();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .family(egui::FontFamily::Monospace)
                .size(11.0)
                .color(muted())
                .strong(),
        );
        ui.label(
            RichText::new(section_tasks.len().to_string())
                .size(11.0)
                .color(primary_text()),
        );
    });
    ui.add_space(6.0);

    for (index, task) in section_tasks.into_iter().enumerate() {
        let fill = if index % 2 == 0 {
            panel()
        } else {
            panel_alt()
        };
        let border = if task.location == "todo" {
            Color32::from_rgba_unmultiplied(160, 202, 255, 120)
        } else {
            Color32::from_rgba_unmultiplied(110, 251, 190, 90)
        };
        let response = egui::Frame::new()
            .fill(fill)
            .stroke(Stroke::new(1.0, outline()))
            .corner_radius(8.0)
            .outer_margin(egui::Margin::same(0))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                let left = ui.min_rect().left();
                let top = ui.min_rect().top();
                let bottom = ui.max_rect().bottom();
                ui.painter().line_segment(
                    [egui::pos2(left, top), egui::pos2(left, bottom)],
                    Stroke::new(2.0, border),
                );
                ui.add_space(4.0);
                render_task_row(ui, &task);
            })
            .response;

        if response.clicked() {
            app.open_task(&task.task_id);
        }
        ui.add_space(8.0);
    }
}

fn render_task_row(ui: &mut egui::Ui, task: &BoardTaskSummary) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&task.task_id)
                .family(egui::FontFamily::Monospace)
                .size(12.0)
                .color(muted()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            pill(ui, &task.phase_label, accent_for_task(task));
        });
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new(&task.title)
            .size(15.0)
            .color(primary_text())
            .strong(),
    );
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        pill(ui, &task.pipeline, pipeline_ink(task));
        ui.label(
            RichText::new(format!("{} · attempts {}", task.location, task.attempts))
                .size(12.0)
                .color(muted()),
        );
    });

    if !task.issue.is_empty() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(&task.issue)
                .size(12.0)
                .color(Color32::from_rgb(255, 180, 171)),
        );
    }

    if !task.steps.is_empty() {
        ui.add_space(8.0);
        render_progress(ui, task);
    }
}

fn render_task_header(ui: &mut egui::Ui, task: &BoardTaskSummary) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&task.task_id)
                .family(egui::FontFamily::Monospace)
                .size(12.0)
                .color(muted()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            pill(ui, &task.phase_label, accent_for_task(task));
        });
    });
    ui.add_space(4.0);
    ui.label(
        RichText::new(&task.title)
            .size(18.0)
            .color(primary_text())
            .strong(),
    );
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        pill(ui, &task.pipeline, pipeline_ink(task));
        pill(ui, &task.location, accent_for_task(task));
        ui.label(
            RichText::new(task.path.display().to_string())
                .size(11.0)
                .color(muted()),
        );
    });
    if !task.steps.is_empty() {
        ui.add_space(8.0);
        render_progress(ui, task);
    }
    if !task.issue.is_empty() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(&task.issue)
                .size(12.0)
                .color(Color32::from_rgb(255, 180, 171)),
        );
    }
}

fn pill(ui: &mut egui::Ui, text: &str, ink: Color32) {
    egui::Frame::new()
        .fill(ink.linear_multiply(0.12))
        .stroke(Stroke::new(1.0, ink.linear_multiply(0.25)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text.to_ascii_uppercase())
                    .family(egui::FontFamily::Monospace)
                    .size(10.0)
                    .color(ink)
                    .strong(),
            );
        });
}

fn render_progress(ui: &mut egui::Ui, task: &BoardTaskSummary) {
    let accent = accent_for_task(task);
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            for (index, step) in task.steps.iter().enumerate() {
                render_step_dot(ui, step, accent);
                if index + 1 < task.steps.len() {
                    render_step_connector(ui, step, accent);
                }
            }
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for step in &task.steps {
                let label_color = if step.status == "current" {
                    primary_text()
                } else {
                    muted()
                };
                ui.add_sized(
                    [60.0, 0.0],
                    egui::Label::new(
                        RichText::new(&step.label)
                            .size(9.5)
                            .color(label_color),
                    ),
                );
            }
        });
    });
}

fn render_step_dot(ui: &mut egui::Ui, step: &crate::board_data::BoardStep, accent: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    let center = rect.center();
    match step.status.as_str() {
        "complete" => {
            ui.painter().circle_filled(center, 3.5, accent);
        }
        "current" => {
            ui.painter()
                .circle_stroke(center, 5.5, Stroke::new(2.0, accent.linear_multiply(0.3)));
            ui.painter().circle_filled(center, 3.5, accent);
        }
        _ => {
            ui.painter()
                .circle_stroke(center, 3.5, Stroke::new(1.2, outline()));
        }
    }
}

fn render_step_connector(ui: &mut egui::Ui, step: &crate::board_data::BoardStep, accent: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 14.0), egui::Sense::hover());
    let connector = match step.status.as_str() {
        "complete" => accent,
        _ => outline(),
    };
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.center().y),
            egui::pos2(rect.right(), rect.center().y),
        ],
        Stroke::new(1.0, connector),
    );
}

fn surface() -> Color32 {
    Color32::from_rgb(19, 19, 19)
}

fn panel() -> Color32 {
    Color32::from_rgb(32, 31, 31)
}

fn panel_alt() -> Color32 {
    Color32::from_rgb(28, 27, 27)
}

fn outline() -> Color32 {
    Color32::from_rgba_unmultiplied(68, 71, 79, 90)
}

fn primary_text() -> Color32 {
    Color32::from_rgb(229, 226, 225)
}

fn muted() -> Color32 {
    Color32::from_rgb(142, 144, 154)
}

fn accent_for_task(task: &BoardTaskSummary) -> Color32 {
    if task.location == "done" {
        Color32::from_rgb(110, 251, 190)
    } else {
        Color32::from_rgb(160, 202, 255)
    }
}

fn pipeline_ink(task: &BoardTaskSummary) -> Color32 {
    if task.pipeline.eq_ignore_ascii_case("tdd") {
        Color32::from_rgb(208, 188, 255)
    } else {
        accent_for_task(task)
    }
}
