//! The Sign & Cut panel's CNC and Laser section.

use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::ui::field::NumField;
use crate::ui::signcut_docker::{dim, fmt_len, length_field};
use egui::{Color32, RichText, Ui};
use tracedraw_signcut::cnc::gcode::{CommentStyle, Direction, LeadIn, Machine, Origin, Side, Tool};

pub fn section(app: &mut App, ui: &mut Ui) {
    dim(
        ui,
        tr(if app.selection.is_empty() {
            "signcut.whole_page"
        } else {
            "signcut.selection_only"
        }),
    );
    sub(ui, "signcut.cnc_gcode", |ui| gcode(app, ui));
    sub(ui, "signcut.cnc_hot_wire", |ui| hot_wire(app, ui));
    sub(ui, "signcut.cnc_box", |ui| finger_box(app, ui));
    sub(ui, "signcut.cnc_joints", |ui| finger_joints(app, ui));
    sub(ui, "signcut.cnc_door", |ui| door(app, ui));
    sub(ui, "signcut.cnc_center", |ui| centre_line(app, ui));
}

fn centre_line(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    dim(ui, tr("signcut.cnc_center_hint"));
    let c = &mut app.sc_cnc.centre;
    egui::Grid::new("cnc_center_grid")
        .num_columns(2)
        .spacing([8.0, 3.0])
        .show(ui, |ui| {
            row(ui, "signcut.tolerance", |ui| {
                length_field(ui, u, &mut c.tolerance, 0.01);
            });
            row(ui, "signcut.cnc_center_prune", |ui| {
                length_field(ui, u, &mut c.prune, 0.0);
            });
        });
    if ui.button(tr("signcut.cnc_center_make")).clicked() {
        app.cnc_center_lines();
    }
}

fn door(app: &mut App, ui: &mut Ui) {
    use tracedraw_signcut::cnc::door::InnerShape;
    let u = app.units;
    dim(ui, tr("signcut.cnc_door_hint"));
    let d = &mut app.sc_cnc.door;
    egui::Grid::new("cnc_door_grid")
        .num_columns(2)
        .spacing([8.0, 3.0])
        .show(ui, |ui| {
            row(ui, "signcut.width", |ui| {
                length_field(ui, u, &mut d.width, 1.0);
            });
            row(ui, "signcut.height", |ui| {
                length_field(ui, u, &mut d.height, 1.0);
            });
            row(ui, "signcut.cnc_door_frame", |ui| {
                length_field(ui, u, &mut d.frame, 0.0);
            });
            row(ui, "signcut.cnc_door_inner", |ui| {
                ui.vertical(|ui| {
                    ui.radio_value(
                        &mut d.inner,
                        InnerShape::Square,
                        tr("signcut.cnc_door_square"),
                    );
                    ui.radio_value(
                        &mut d.inner,
                        InnerShape::Rounded,
                        tr("signcut.cnc_door_rounded"),
                    );
                    ui.radio_value(&mut d.inner, InnerShape::Arch, tr("signcut.cnc_door_arch"));
                });
            });
            match d.inner {
                InnerShape::Rounded => row(ui, "signcut.cnc_door_radius", |ui| {
                    length_field(ui, u, &mut d.radius, 0.0);
                }),
                InnerShape::Arch => row(ui, "signcut.cnc_door_rise", |ui| {
                    length_field(ui, u, &mut d.arch, 0.0);
                }),
                InnerShape::Square => {}
            }
            row(ui, "signcut.cnc_door_grooves", |ui| {
                ui.add(NumField::new(&mut d.grooves).range(0..=20));
            });
            if d.grooves > 0 {
                row(ui, "signcut.cnc_door_groove_offset", |ui| {
                    length_field(ui, u, &mut d.groove_offset, 0.0);
                });
                row(ui, "signcut.cnc_door_groove_spacing", |ui| {
                    length_field(ui, u, &mut d.groove_spacing, 0.0);
                });
            }
        });
    if ui.button(tr("signcut.cnc_door_make")).clicked() {
        app.cnc_door();
    }
}

fn finger_joints(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    dim(ui, tr("signcut.cnc_joints_hint"));
    let s = &mut app.sc_cnc;
    ui.radio_value(&mut s.joint_pair, true, tr("signcut.cnc_joint_pair"));
    ui.radio_value(&mut s.joint_pair, false, tr("signcut.cnc_joint_all"));
    let j = &mut s.joint;
    egui::Grid::new("cnc_joint_grid")
        .num_columns(2)
        .spacing([8.0, 3.0])
        .show(ui, |ui| {
            row(ui, "signcut.cnc_finger", |ui| {
                length_field(ui, u, &mut j.finger, 0.5);
            });
            row(ui, "signcut.cnc_joint_depth", |ui| {
                length_field(ui, u, &mut j.depth, 0.1);
            });
            row(ui, "signcut.cnc_joint_min", |ui| {
                length_field(ui, u, &mut j.min_length, 0.0);
            });
        });
    ui.checkbox(&mut j.flip, tr("signcut.cnc_joint_flip"));
    if ui.button(tr("signcut.cnc_joint_make")).clicked() {
        app.cnc_finger_joints();
    }
}

fn finger_box(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let b = &mut app.sc_cnc.box_spec;
    egui::Grid::new("cnc_box_grid")
        .num_columns(2)
        .spacing([8.0, 3.0])
        .show(ui, |ui| {
            row(ui, "signcut.width", |ui| {
                length_field(ui, u, &mut b.width, 1.0);
            });
            row(ui, "signcut.cnc_box_depth", |ui| {
                length_field(ui, u, &mut b.depth, 1.0);
            });
            row(ui, "signcut.height", |ui| {
                length_field(ui, u, &mut b.height, 1.0);
            });
            row(ui, "signcut.cnc_thickness", |ui| {
                length_field(ui, u, &mut b.thickness, 0.1);
            });
            row(ui, "signcut.cnc_finger", |ui| {
                length_field(ui, u, &mut b.finger, 0.5);
            });
            row(ui, "signcut.cnc_kerf", |ui| {
                length_field(ui, u, &mut b.kerf, 0.0);
            });
            row(ui, "signcut.cnc_gap", |ui| {
                length_field(ui, u, &mut b.gap, 0.0);
            });
        });
    ui.checkbox(&mut b.inside, tr("signcut.cnc_box_inside"));
    ui.checkbox(&mut b.open_top, tr("signcut.cnc_box_open"));
    if ui.button(tr("signcut.cnc_box_make")).clicked() {
        app.cnc_finger_box();
    }
}

/// A combo box over the tool list.
fn tool_combo(ui: &mut Ui, id: &str, tools: &[Tool], current: &mut usize) {
    let name = tools
        .get(*current)
        .map(|t| t.name.clone())
        .unwrap_or_default();
    egui::ComboBox::from_id_salt(id)
        .selected_text(name)
        .width(150.0)
        .show_ui(ui, |ui| {
            for (k, t) in tools.iter().enumerate() {
                ui.selectable_value(current, k, &t.name);
            }
        });
}

fn hot_wire(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    dim(ui, tr("signcut.cnc_wire_hint"));
    let s = &mut app.sc_cnc;
    egui::Grid::new("cnc_wire_grid")
        .num_columns(2)
        .spacing([8.0, 3.0])
        .show(ui, |ui| {
            row(ui, "signcut.cnc_wire_start", |ui| {
                ui.label("X");
                length_field(ui, u, &mut s.wire_start_x, -1_000_000.0);
                ui.label("Y");
                length_field(ui, u, &mut s.wire_start_y, -1_000_000.0);
            });
            row(ui, "signcut.cnc_tool", |ui| {
                tool_combo(ui, "cnc_wire_tool", &s.tools, &mut s.wire_tool);
            });
        });
    if ui.button(tr("signcut.cnc_wire_make")).clicked() {
        app.cnc_hot_wire();
    }
    if ui.button(tr("signcut.cnc_wire_export")).clicked() {
        app.cnc_export_wire_gcode();
    }
}

/// A collapsing block of the section.
fn sub(ui: &mut Ui, key: &str, add: impl FnOnce(&mut Ui)) {
    egui::CollapsingHeader::new(tr(key))
        .id_salt(("signcut_cnc", key))
        .show(ui, add);
}

/// A plain number field with a suffix.
fn number(ui: &mut Ui, v: &mut f64, min: f64, max: f64, suffix: &str) -> bool {
    ui.add_sized(
        [90.0, 22.0],
        NumField::new(v)
            .range(min..=max)
            .max_decimals(3)
            .suffix(suffix),
    )
    .changed()
}

fn row(ui: &mut Ui, key: &str, add: impl FnOnce(&mut Ui)) {
    ui.label(tr(key));
    add(ui);
    ui.end_row();
}

fn origin_label(o: Origin) -> String {
    tr(match o {
        Origin::BottomLeft => "signcut.cnc_origin_bl",
        Origin::BottomRight => "signcut.cnc_origin_br",
        Origin::TopLeft => "signcut.cnc_origin_tl",
        Origin::TopRight => "signcut.cnc_origin_tr",
        Origin::Center => "signcut.cnc_origin_c",
    })
}

fn gcode(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    machine_editor(ui, &mut app.sc_cnc.machines, &mut app.sc_cnc.machine, u);
    ui.separator();
    tool_editor(ui, &mut app.sc_cnc.tools, &mut app.sc_cnc.tool, u);
    ui.separator();
    ui.strong(tr("signcut.cnc_jobs"));
    dim(ui, tr("signcut.cnc_jobs_hint"));
    if ui.button(tr("signcut.cnc_find_colors")).clicked() {
        app.cnc_find_colors();
    }
    let tools: Vec<String> = app.sc_cnc.tools.iter().map(|t| t.name.clone()).collect();
    let mut remove = None;
    let mut swap = None;
    let n = app.sc_cnc.jobs.len();
    for (i, job) in app.sc_cnc.jobs.iter_mut().enumerate() {
        ui.push_id(("cnc_job", i), |ui| {
            ui.horizontal(|ui| {
                let [r, g, b] = job.color;
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 2.0, Color32::from_rgb(r, g, b));
                ui.checkbox(&mut job.enabled, crate::signcut_cnc::hex(job.color));
                if ui.small_button("\u{2191}").clicked() && i > 0 {
                    swap = Some(i - 1);
                }
                if ui.small_button("\u{2193}").clicked() && i + 1 < n {
                    swap = Some(i);
                }
                if ui.small_button(tr("signcut.cnc_remove")).clicked() {
                    remove = Some(i);
                }
            });
            if !job.enabled {
                return;
            }
            egui::Grid::new("cnc_job_grid")
                .num_columns(2)
                .spacing([8.0, 3.0])
                .show(ui, |ui| {
                    row(ui, "signcut.cnc_tool", |ui| {
                        let name = tools.get(job.tool).cloned().unwrap_or_default();
                        egui::ComboBox::from_id_salt("cnc_job_tool")
                            .selected_text(name)
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for (k, t) in tools.iter().enumerate() {
                                    ui.selectable_value(&mut job.tool, k, t);
                                }
                            });
                    });
                    row(ui, "signcut.cnc_depth", |ui| {
                        length_field(ui, u, &mut job.depth, 0.0);
                    });
                    row(ui, "signcut.cnc_passes", |ui| {
                        ui.add(NumField::new(&mut job.passes).range(1..=100));
                    });
                    row(ui, "signcut.cnc_side", |ui| {
                        let label = |s: Side| {
                            tr(match s {
                                Side::On => "signcut.cnc_side_on",
                                Side::Outside => "signcut.cnc_side_outside",
                                Side::Inside => "signcut.cnc_side_inside",
                            })
                        };
                        egui::ComboBox::from_id_salt("cnc_job_side")
                            .selected_text(label(job.side))
                            .show_ui(ui, |ui| {
                                for s in [Side::On, Side::Outside, Side::Inside] {
                                    ui.selectable_value(&mut job.side, s, label(s));
                                }
                            });
                    });
                    row(ui, "signcut.cnc_direction", |ui| {
                        ui.radio_value(
                            &mut job.direction,
                            Direction::Climb,
                            tr("signcut.cnc_climb"),
                        );
                        ui.radio_value(
                            &mut job.direction,
                            Direction::Conventional,
                            tr("signcut.cnc_conventional"),
                        );
                    });
                    row(ui, "signcut.cnc_lead_in", |ui| {
                        let label = |l: LeadIn| {
                            tr(match l {
                                LeadIn::None => "signcut.cnc_lead_none",
                                LeadIn::Straight => "signcut.cnc_lead_straight",
                                LeadIn::Arc => "signcut.cnc_lead_arc",
                            })
                        };
                        egui::ComboBox::from_id_salt("cnc_job_lead")
                            .selected_text(label(job.lead_in))
                            .show_ui(ui, |ui| {
                                for l in [LeadIn::None, LeadIn::Straight, LeadIn::Arc] {
                                    ui.selectable_value(&mut job.lead_in, l, label(l));
                                }
                            });
                        if job.lead_in != LeadIn::None {
                            length_field(ui, u, &mut job.lead_length, 0.0);
                        }
                    });
                    row(ui, "signcut.cnc_bridges", |ui| {
                        ui.add(NumField::new(&mut job.bridges).range(0..=100));
                    });
                    if job.bridges > 0 {
                        row(ui, "signcut.cnc_bridge_length", |ui| {
                            length_field(ui, u, &mut job.bridge_length, 0.0);
                        });
                        row(ui, "signcut.cnc_bridge_height", |ui| {
                            length_field(ui, u, &mut job.bridge_height, 0.0);
                        });
                    }
                });
            ui.checkbox(&mut job.inner_first, tr("signcut.cnc_inner_first"));
        });
    }
    if let Some(i) = swap {
        if i + 1 < app.sc_cnc.jobs.len() {
            app.sc_cnc.jobs.swap(i, i + 1);
        }
    }
    if let Some(i) = remove {
        if i < app.sc_cnc.jobs.len() {
            app.sc_cnc.jobs.remove(i);
        }
    }
    ui.separator();
    ui.checkbox(&mut app.sc_cnc.preview, tr("signcut.cnc_show_paths"));
    if app.sc_cnc.preview && !app.sc_cnc.jobs.is_empty() {
        let paths = app.cnc_preview();
        app.docker_preview.extend(paths);
    }
    if ui
        .add_enabled(
            !app.sc_cnc.jobs.is_empty(),
            egui::Button::new(tr("signcut.cnc_export")),
        )
        .clicked()
    {
        app.cnc_export_gcode();
    }
    if let Some(s) = app.sc_cnc.last {
        dim(
            ui,
            trf(
                "signcut.cnc_summary",
                &[("n", &s.cuts.to_string()), ("len", &fmt_len(u, s.length))],
            ),
        );
        if !s.on_table {
            ui.label(
                RichText::new(tr("signcut.cnc_off_table")).color(Color32::from_rgb(200, 60, 40)),
            );
        }
    }
}

fn machine_editor(ui: &mut Ui, machines: &mut Vec<Machine>, current: &mut usize, u: Units) {
    ui.horizontal(|ui| {
        ui.label(tr("signcut.cnc_machine"));
        let name = machines
            .get(*current)
            .map(|m| m.name.clone())
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("cnc_machine")
            .selected_text(name)
            .width(130.0)
            .show_ui(ui, |ui| {
                for (i, m) in machines.iter().enumerate() {
                    ui.selectable_value(current, i, &m.name);
                }
            });
        if ui
            .small_button("+")
            .on_hover_text(tr("signcut.cnc_add"))
            .clicked()
        {
            let mut m = machines.get(*current).cloned().unwrap_or_default();
            m.name = format!("{} 2", m.name);
            machines.push(m);
            *current = machines.len() - 1;
        }
        if ui
            .add_enabled(machines.len() > 1, egui::Button::new("\u{2212}").small())
            .on_hover_text(tr("signcut.cnc_remove"))
            .clicked()
            && *current < machines.len()
        {
            machines.remove(*current);
            *current = current.saturating_sub(1);
        }
    });
    let Some(m) = machines.get_mut(*current) else {
        *current = 0;
        return;
    };
    egui::CollapsingHeader::new(tr("signcut.cnc_machine_settings"))
        .id_salt("cnc_machine_settings")
        .show(ui, |ui| {
            egui::Grid::new("cnc_machine_grid")
                .num_columns(2)
                .spacing([8.0, 3.0])
                .show(ui, |ui| {
                    row(ui, "signcut.cnc_name", |ui| {
                        ui.text_edit_singleline(&mut m.name);
                    });
                    row(ui, "signcut.cnc_table", |ui| {
                        length_field(ui, u, &mut m.width, 1.0);
                        length_field(ui, u, &mut m.height, 1.0);
                    });
                    row(ui, "signcut.cnc_origin", |ui| {
                        egui::ComboBox::from_id_salt("cnc_origin")
                            .selected_text(origin_label(m.origin))
                            .show_ui(ui, |ui| {
                                for o in Origin::ALL {
                                    ui.selectable_value(&mut m.origin, o, origin_label(o));
                                }
                            });
                    });
                    row(ui, "signcut.cnc_decimals", |ui| {
                        ui.add(NumField::new(&mut m.decimals).range(0..=6));
                    });
                    row(ui, "signcut.cnc_comments", |ui| {
                        let label = |c: CommentStyle| match c {
                            CommentStyle::Parentheses => "( )".to_string(),
                            CommentStyle::Semicolon => ";".to_string(),
                            CommentStyle::Off => tr("signcut.cnc_comments_off"),
                        };
                        egui::ComboBox::from_id_salt("cnc_comments")
                            .selected_text(label(m.comments))
                            .show_ui(ui, |ui| {
                                for c in [
                                    CommentStyle::Parentheses,
                                    CommentStyle::Semicolon,
                                    CommentStyle::Off,
                                ] {
                                    ui.selectable_value(&mut m.comments, c, label(c));
                                }
                            });
                    });
                });
            ui.checkbox(&mut m.z_axis, tr("signcut.cnc_z_axis"));
            dim(ui, tr("signcut.cnc_template_hint"));
            for (key, text) in [
                ("signcut.cnc_header", &mut m.header),
                ("signcut.cnc_footer", &mut m.footer),
                ("signcut.cnc_tool_change", &mut m.tool_change),
                ("signcut.cnc_beam_on", &mut m.beam_on),
                ("signcut.cnc_beam_off", &mut m.beam_off),
            ] {
                ui.label(tr(key));
                ui.add(
                    egui::TextEdit::multiline(text)
                        .desired_rows(2)
                        .code_editor()
                        .desired_width(f32::INFINITY),
                );
            }
        });
}

fn tool_editor(ui: &mut Ui, tools: &mut Vec<Tool>, current: &mut usize, u: Units) {
    ui.horizontal(|ui| {
        ui.label(tr("signcut.cnc_tools"));
        let name = tools
            .get(*current)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("cnc_tool_edit")
            .selected_text(name)
            .width(130.0)
            .show_ui(ui, |ui| {
                for (i, t) in tools.iter().enumerate() {
                    ui.selectable_value(current, i, &t.name);
                }
            });
        if ui
            .small_button("+")
            .on_hover_text(tr("signcut.cnc_add"))
            .clicked()
        {
            let mut t = tools.get(*current).cloned().unwrap_or_default();
            t.name = format!("{} 2", t.name);
            tools.push(t);
            *current = tools.len() - 1;
        }
    });
    let Some(t) = tools.get_mut(*current) else {
        *current = 0;
        return;
    };
    egui::CollapsingHeader::new(tr("signcut.cnc_tool_settings"))
        .id_salt("cnc_tool_settings")
        .show(ui, |ui| {
            egui::Grid::new("cnc_tool_grid")
                .num_columns(2)
                .spacing([8.0, 3.0])
                .show(ui, |ui| {
                    row(ui, "signcut.cnc_name", |ui| {
                        ui.text_edit_singleline(&mut t.name);
                    });
                    row(ui, "signcut.cnc_diameter", |ui| {
                        length_field(ui, u, &mut t.diameter, 0.0);
                    });
                    row(ui, "signcut.cnc_rpm", |ui| {
                        number(ui, &mut t.rpm, 0.0, 100_000.0, "");
                    });
                    row(ui, "signcut.cnc_feed", |ui| {
                        number(ui, &mut t.feed, 1.0, 100_000.0, " mm/min");
                    });
                    row(ui, "signcut.cnc_plunge", |ui| {
                        number(ui, &mut t.plunge, 1.0, 100_000.0, " mm/min");
                    });
                    row(ui, "signcut.cnc_max_depth", |ui| {
                        length_field(ui, u, &mut t.max_depth, 0.0);
                    });
                    row(ui, "signcut.cnc_safe_z", |ui| {
                        length_field(ui, u, &mut t.safe_z, 0.0);
                    });
                });
            ui.checkbox(&mut t.park, tr("signcut.cnc_park"));
            if t.park {
                ui.horizontal(|ui| {
                    ui.label("X");
                    number(ui, &mut t.park_x, -100_000.0, 100_000.0, " mm");
                    ui.label("Y");
                    number(ui, &mut t.park_y, -100_000.0, 100_000.0, " mm");
                });
            }
        });
}
