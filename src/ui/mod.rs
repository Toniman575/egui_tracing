mod color;
mod state;

use std::sync::{Arc, Mutex};

use egui::epaint::text::TextWrapping;
use egui::text::LayoutJob;
use egui::{Align, Color32, Label, Layout, RichText, TextFormat, TextStyle};
use egui_extras::{Column, TableBuilder};
use globset::{Glob, GlobSetBuilder};
use itertools::Itertools;

use self::color::*;
use self::state::LogsState;
use crate::string::Ellipse;
use crate::time::DateTimeFormatExt;
use crate::tracing::collector::EventCollector;

// https://github.com/emilk/egui/blob/9478e50d012c5138551c38cbee16b07bc1fcf283/crates/egui/src/widgets/separator.rs#L24C13-L24C20
pub const SEPARATOR_SPACING: f32 = 6.0;

pub struct Logs {
    collector: EventCollector,
}

impl Logs {
    #[must_use]
    pub const fn new(collector: EventCollector) -> Self {
        Self { collector }
    }
}

impl Logs {
    pub fn ui(self, ui: &mut egui::Ui) {
        let state = ui.memory_mut(|mem| {
            let state_mem_id = ui.id();
            mem.data
                .get_temp_mut_or_insert_with(state_mem_id, || {
                    Arc::new(Mutex::new(LogsState::default()))
                })
                .clone()
        });
        let mut state = state.lock().unwrap();

        // TODO: cache the globset
        let glob = {
            let mut glob = GlobSetBuilder::new();
            for target in state.target_filter.targets.clone() {
                glob.add(target);
            }
            glob.build().unwrap()
        };

        let events = self.collector.events();
        let filtered_events = events
            .iter()
            .filter(|event| state.level_filter.get(event.level) && !glob.is_match(&event.target))
            .collect::<Vec<_>>();

        let row_height =
            SEPARATOR_SPACING + ui.style().text_styles.get(&TextStyle::Small).unwrap().size;

        TableBuilder::new(ui)
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .max_scroll_height(f32::INFINITY)
            .column(Column::initial(100.))
            .column(Column::initial(80.))
            .column(Column::initial(120.))
            .column(Column::remainder().at_least(120.).clip(true))
            .header(row_height, |mut header| {
                header.col(|ui| {
                    ui.label("Time");
                });
                header.col(|ui| {
                    ui.menu_button("Level", |ui| {
                        ui.label("Level Filter");
                        ui.add(egui::Checkbox::new(
                            &mut state.level_filter.trace,
                            RichText::new("TRACE").color(TRACE_COLOR),
                        ));
                        ui.add(egui::Checkbox::new(
                            &mut state.level_filter.debug,
                            RichText::new("DEBUG").color(DEBUG_COLOR),
                        ));
                        ui.add(egui::Checkbox::new(
                            &mut state.level_filter.info,
                            RichText::new("INFO").color(INFO_COLOR),
                        ));
                        ui.add(egui::Checkbox::new(
                            &mut state.level_filter.warn,
                            RichText::new("WARN").color(WARN_COLOR),
                        ));
                        ui.add(egui::Checkbox::new(
                            &mut state.level_filter.error,
                            RichText::new("ERROR").color(ERROR_COLOR),
                        ));
                    });
                });
                header.col(|ui| {
                    ui.menu_button("Target", |ui| {
                        ui.label("Target Filter");

                        let (input, add_button) = ui
                            .horizontal(|ui| {
                                let input = ui
                                    .text_edit_singleline(&mut state.target_filter.input)
                                    .on_hover_text("example: eframe::*");
                                let button = ui.button("Add");
                                (input, button)
                            })
                            .inner;

                        if add_button.clicked()
                            || (input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            let target = Glob::new(&state.target_filter.input).unwrap();
                            state.target_filter.targets.push(target);
                            state.target_filter.input = "".to_owned();
                        }

                        for (i, target) in state.target_filter.targets.clone().iter().enumerate() {
                            ui.separator();
                            let pattern = target.glob().to_owned();
                            ui.horizontal(|ui| {
                                let mut job = LayoutJob::single_section(
                                    pattern.clone(),
                                    TextFormat::default(),
                                );
                                job.wrap.max_rows = 1;

                                ui.label(job).on_hover_text(pattern);
                                ui.add_space(ui.available_width() - 43.0);
                                if ui.button("Delete").clicked() {
                                    state.target_filter.targets.remove(i);
                                }
                            });
                        }
                    });
                });
                header.col(|ui| {
                    ui.horizontal(|ui| {
                        ui.set_clip_rect(egui::Rect::EVERYTHING);
                        ui.label("Message");

                        ui.horizontal_top(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                if ui.add(egui::Button::new("To Bottom")).clicked() {
                                    ui.scroll_to_rect(
                                        egui::Rect {
                                            min: egui::Pos2 { x: 0.0, y: 0.0 },
                                            max: egui::Pos2 {
                                                x: f32::MAX,
                                                y: f32::MAX,
                                            },
                                        },
                                        Some(egui::Align::Max),
                                    );
                                }

                                if ui.add(egui::Button::new("Clear")).clicked() {
                                    self.collector.clear();
                                }
                            });
                        });
                    });
                });
            })
            .body(|body| {
                body.rows(row_height, filtered_events.len(), |row_index, mut row| {
                    let event = filtered_events.get(row_index).unwrap();

                    row.col(|ui| {
                        ui.colored_label(Color32::GRAY, event.time.format_short())
                            .on_hover_text(event.time.format_detailed());
                    });
                    row.col(|ui| {
                        ui.colored_label(event.level.to_color32(), event.level.as_str());
                    });
                    row.col(|ui| {
                        let mut job = LayoutJob::single_section(
                            event.target.clone(),
                            TextFormat {
                                color: Color32::GRAY,
                                ..Default::default()
                            },
                        );
                        job.wrap.max_rows = 1;

                        ui.label(job).on_hover_text(&event.target);
                    });
                    row.col(|ui| {
                        let message = event.fields.get("message").unwrap();

                        ui.style_mut().visuals.override_text_color = Some(Color32::WHITE);

                        ui.add(
                            Label::new(
                                Itertools::intersperse(message.lines(), " ").collect::<String>(),
                            )
                            .wrap(false),
                        )
                        .on_hover_text(message);
                    });
                })
            });
    }
}
