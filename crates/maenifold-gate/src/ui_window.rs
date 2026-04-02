use std::path::PathBuf;

use eframe::egui;
use eframe::egui::{Color32, RichText};

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
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(Color32::from_rgb(243, 239, 230))
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Read Only Pipeline View")
                            .size(11.0)
                            .color(Color32::from_rgb(111, 103, 93))
                            .strong(),
                    );
                    ui.add_space(2.0);
                    ui.heading(RichText::new("Gate UI").size(28.0));
                    ui.label(
                        RichText::new(format!("{} tasks", self.tasks.len()))
                            .size(13.0)
                            .color(Color32::from_rgb(111, 103, 93)),
                    );
                    ui.add_space(10.0);

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let tasks = self.tasks.clone();
                            for task in tasks {
                                let response = egui::Frame::group(ui.style())
                                    .fill(Color32::from_rgba_unmultiplied(255, 252, 246, 245))
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        Color32::from_rgba_unmultiplied(29, 26, 22, 28),
                                    ))
                                    .corner_radius(18.0)
                                    .inner_margin(egui::Margin::same(12))
                                    .show(ui, |ui| {
                                        render_task_card(ui, &task);
                                    })
                                    .response;

                                if response.clicked() {
                                    self.open_task(&task.task_id);
                                }
                                ui.add_space(10.0);
                            }
                        });
                });
            });

        if self.detail_open {
            if let Some(detail) = &mut self.selected {
                egui::Window::new(format!("{} · {}", detail.task.task_id, detail.task.title))
                    .open(&mut self.detail_open)
                    .default_width(420.0)
                    .default_height(700.0)
                    .show(ctx, |ui| {
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
                                            .size(13.0),
                                    )
                                    .selectable(true),
                                );
                            });
                    });
            }
        }
    }
}

fn render_task_card(ui: &mut egui::Ui, task: &BoardTaskSummary) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&task.task_id)
                .family(egui::FontFamily::Monospace)
                .size(12.0)
                .color(Color32::from_rgb(111, 103, 93)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            pill(ui, &task.phase_label, Color32::from_rgb(207, 92, 54));
        });
    });

    ui.add_space(4.0);
    ui.label(RichText::new(&task.title).size(16.0).strong());
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        pill(ui, &task.pipeline, Color32::from_rgb(29, 26, 22));
        ui.label(
            RichText::new(format!("{} · attempts {}", task.location, task.attempts))
                .size(12.0)
                .color(Color32::from_rgb(111, 103, 93)),
        );
    });

    if !task.issue.is_empty() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(&task.issue)
                .size(12.0)
                .color(Color32::from_rgb(170, 49, 31)),
        );
    }

    if !task.steps.is_empty() {
        ui.add_space(8.0);
        ui.horizontal_top(|ui| {
            for step in &task.steps {
                ui.vertical(|ui| {
                    let color = match step.status.as_str() {
                        "complete" => Color32::from_rgb(39, 103, 73),
                        "current" => Color32::from_rgb(207, 92, 54),
                        _ => Color32::from_rgba_unmultiplied(29, 26, 22, 45),
                    };
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 5.0, color);
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(&step.label)
                            .size(10.5)
                            .color(if step.status == "current" {
                                Color32::from_rgb(29, 26, 22)
                            } else {
                                Color32::from_rgb(111, 103, 93)
                            }),
                    );
                });
                ui.add_space(8.0);
            }
        });
    }
}

fn render_task_header(ui: &mut egui::Ui, task: &BoardTaskSummary) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&task.task_id)
                .family(egui::FontFamily::Monospace)
                .size(12.0)
                .color(Color32::from_rgb(111, 103, 93)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            pill(ui, &task.phase_label, Color32::from_rgb(207, 92, 54));
        });
    });
    ui.add_space(4.0);
    ui.label(RichText::new(&task.title).size(18.0).strong());
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        pill(ui, &task.pipeline, Color32::from_rgb(29, 26, 22));
        pill(ui, &task.location, Color32::from_rgb(29, 26, 22));
        ui.label(
            RichText::new(task.path.display().to_string())
                .size(11.0)
                .color(Color32::from_rgb(111, 103, 93)),
        );
    });
    if !task.issue.is_empty() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(&task.issue)
                .size(12.0)
                .color(Color32::from_rgb(170, 49, 31)),
        );
    }
}

fn pill(ui: &mut egui::Ui, text: &str, ink: Color32) {
    egui::Frame::new()
        .fill(ink.linear_multiply(0.12))
        .corner_radius(999.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(ink).strong());
        });
}
