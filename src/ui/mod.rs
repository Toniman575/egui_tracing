mod color;
mod components;
mod state;

use std::sync::{Arc, Mutex};

use egui::{Color32, Context, Label, RichText, TextStyle, TopBottomPanel};
use egui_extras::{Column, TableBuilder};
use globset::{Glob, GlobSetBuilder};

use self::color::*;
use self::components::constants::SEPARATOR_SPACING;
use self::components::target_menu_item::TargetMenuItem;
use self::state::LogsState;
use crate::string::Ellipse;
use crate::time::DateTimeFormatExt;
use crate::tracing::collector::EventCollector;

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
            .column(Column::initial(100.).resizable(false))
            .column(Column::initial(80.).resizable(false))
            .column(Column::initial(120.).resizable(false))
            .column(Column::remainder().resizable(false))
            .header(row_height, |mut header| {
                header.col(|ui| {
                    ui.heading("Time");
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
                            TargetMenuItem::default()
                                .on_clicked(|| {
                                    state.target_filter.targets.remove(i);
                                })
                                .target(target)
                                .show(ui);
                        }
                    });
                });
                header.col(|ui| {
                    ui.heading("Message");
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
                        ui.colored_label(Color32::GRAY, event.target.truncate_graphemes(18))
                            .on_hover_text(&event.target);
                    });
                    row.col(|ui| {
                        let message = event.fields.get("message").unwrap();

                        ui.style_mut().visuals.override_text_color = Some(Color32::WHITE);
                        ui.add(Label::new(message.lines().collect::<String>()).wrap(false))
                            .on_hover_text(message);
                        ui.set_clip_rect(ui.available_rect_before_wrap());
                    });
                })
            });
    }
}
