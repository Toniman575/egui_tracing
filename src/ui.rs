use std::sync::{Arc, Mutex};

use egui::text::LayoutJob;
use egui::{Align, Button, Color32, Label, Layout, Rect, RichText, TextFormat, TextStyle, Ui};
use egui_extras::{Column, TableBuilder};
use globset::{Error, Glob};
use ringbuffer::RingBuffer;
use tracing::Level;

use crate::time::DurationExt;
use crate::{EguiTracing, State};

impl EguiTracing {
    pub fn ui(&mut self, ui: &mut Ui) {
        let id = ui.id();

        let state = ui.memory_mut(|memory| {
            memory
                .data
                .get_persisted_mut_or_default::<Arc<Mutex<State>>>(id)
                .clone()
        });
        let mut state = state.lock().unwrap();

        if self.globset.is_none() {
            self.update_globset(&state.target_filter);
        }

        // https://github.com/emilk/egui/blob/9478e50d012c5138551c38cbee16b07bc1fcf283/crates/egui/src/widgets/separator.rs#L24C13-L24C20
        const SEPARATOR_SPACING: f32 = 6.0;
        let row_height =
            SEPARATOR_SPACING + ui.style().text_styles.get(&TextStyle::Small).unwrap().size;

        TableBuilder::new(ui)
            .striped(true)
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .max_scroll_height(f32::INFINITY)
            .column(Column::auto())
            .column(Column::exact(40.))
            .column(Column::initial(120.).at_least(50.).resizable(true))
            .column(Column::remainder().at_least(200.).clip(true))
            .header(row_height, |mut header| {
                header.col(|ui| {
                    ui.label("Time");
                });
                header.col(|ui| {
                    ui.set_clip_rect(Rect::EVERYTHING);
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
                            match Glob::new(&state.target_filter.input) {
                                Ok(target) => {
                                    state.target_filter.targets.push(target);
                                    state.target_filter.input = "".to_owned();
                                    self.update_globset(&state.target_filter);
                                    ui.memory_mut(|memory| {
                                        memory.data.remove::<Arc<Error>>(id);
                                    });
                                }
                                Err(error) => ui.memory_mut(|memory| {
                                    memory.data.insert_temp(id, Arc::new(error));
                                }),
                            }
                        }

                        if let Some(error) =
                            ui.memory(|memory| memory.data.get_temp::<Arc<Error>>(id))
                        {
                            // TODO: Maybe add a seperator here.
                            // TODO: Maybe replace with tooltip.
                            ui.colored_label(Color32::RED, error.to_string());
                        }

                        for (i, target) in state.target_filter.targets.clone().iter().enumerate() {
                            ui.separator();
                            let pattern = target.glob().to_owned();
                            ui.horizontal(|ui| {
                                let mut job = LayoutJob::single_section(
                                    pattern.clone(),
                                    TextFormat {
                                        font_id: ui
                                            .style()
                                            .text_styles
                                            .get(&TextStyle::Body)
                                            .unwrap()
                                            .clone(),
                                        ..Default::default()
                                    },
                                );
                                job.wrap.max_rows = 1;

                                ui.with_layout(Layout::default().with_main_wrap(true), |ui| {
                                    // TODO: Fix that the "delete" button keeps expanding the layout.
                                    ui.label(job).on_hover_text(pattern);
                                    if ui.button("Delete").clicked() {
                                        state.target_filter.targets.remove(i);
                                        self.update_globset(&state.target_filter);
                                    }
                                });
                            });
                        }
                    });
                });
                header.col(|ui| {
                    ui.horizontal_top(|ui| {
                        ui.set_clip_rect(Rect::EVERYTHING);
                        ui.label("Message");

                        ui.horizontal_top(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                if ui.add(Button::new("To Bottom")).clicked() {
                                    ui.scroll_to_rect(
                                        Rect {
                                            min: egui::Pos2 { x: 0.0, y: 0.0 },
                                            max: egui::Pos2 {
                                                x: f32::MAX,
                                                y: f32::MAX,
                                            },
                                        },
                                        Some(Align::Max),
                                    );
                                }
                            });
                        });
                    });
                });
            })
            .body(|body| {
                self.fetch_tracings();

                let filtered_events = self
                    .events
                    .iter()
                    .enumerate()
                    .filter_map(|(index, event)| {
                        (state.level_filter.matches(event.level)
                            && (self.globset.as_ref().unwrap().is_empty()
                                || self.globset.as_ref().unwrap().is_match(&event.target)))
                        .then_some(index)
                    })
                    .collect::<Vec<_>>();

                body.rows(row_height, filtered_events.len(), |row_index, mut row| {
                    let index = *filtered_events.get(row_index).unwrap();
                    let event = self.events.get(index).unwrap();

                    row.col(|ui| {
                        ui.add(
                            Label::new(
                                RichText::new(event.time.display_ext()).color(Color32::GRAY),
                            )
                            .wrap(false),
                        );
                    });
                    row.col(|ui| {
                        ui.style_mut().wrap = Some(false);
                        ui.colored_label(event.level.to_color32(), event.level.as_str());
                    });
                    row.col(|ui| {
                        let mut job = LayoutJob::single_section(
                            event.target.clone(),
                            TextFormat {
                                font_id: ui
                                    .style()
                                    .text_styles
                                    .get(&TextStyle::Body)
                                    .unwrap()
                                    .clone(),
                                color: Color32::GRAY,
                                ..Default::default()
                            },
                        );
                        job.wrap.max_rows = 1;

                        ui.label(job).on_hover_text(&event.target);
                    });
                    row.col(|ui| {
                        let mut job = LayoutJob::single_section(
                            event.message.clone(),
                            TextFormat {
                                font_id: ui
                                    .style()
                                    .text_styles
                                    .get(&TextStyle::Body)
                                    .unwrap()
                                    .clone(),
                                color: Color32::WHITE,
                                ..Default::default()
                            },
                        );
                        job.wrap.max_rows = 1;
                        job.break_on_newline = false;

                        ui.add(Label::new(job)).on_hover_text(&event.message);
                    });
                })
            });
    }
}

const TRACE_COLOR: Color32 = Color32::from_rgb(117, 80, 123);
const DEBUG_COLOR: Color32 = Color32::from_rgb(114, 159, 207);
const INFO_COLOR: Color32 = Color32::from_rgb(78, 154, 6);
const WARN_COLOR: Color32 = Color32::from_rgb(196, 160, 0);
const ERROR_COLOR: Color32 = Color32::from_rgb(204, 0, 0);

pub trait ToColor32 {
    fn to_color32(self) -> Color32;
}

impl ToColor32 for Level {
    fn to_color32(self) -> Color32 {
        match self {
            Self::TRACE => TRACE_COLOR,
            Self::DEBUG => DEBUG_COLOR,
            Self::INFO => INFO_COLOR,
            Self::WARN => WARN_COLOR,
            Self::ERROR => ERROR_COLOR,
        }
    }
}
