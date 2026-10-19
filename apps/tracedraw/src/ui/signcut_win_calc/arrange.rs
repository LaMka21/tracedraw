//! The Set Size, Reference Scale and Replace windows.

use super::{picture_button, readout, tabs};
use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::signcut_docker::fmt_len;
use crate::ui::signcut_kit::{self as kit, Buttons, Pressed};
use egui::{Color32, Painter, Pos2, Rect, RichText, Stroke, Ui, Vec2};
use tracedraw_core::geometry::Shape as _;
use tracedraw_signcut::calc::arrange::{MasterPick, Sides, SizeMode};
use tracedraw_signcut::sizing::{self, Anchor, RefMeasure, ReplaceFit, SizeRule};

/// The name of a fixed point.
fn anchor_label(a: Anchor) -> String {
    tr(match a {
        Anchor::TopLeft => "signcut.wcalc_anchor_tl",
        Anchor::Top => "signcut.wcalc_anchor_t",
        Anchor::TopRight => "signcut.wcalc_anchor_tr",
        Anchor::Left => "signcut.wcalc_anchor_l",
        Anchor::Center => "signcut.wcalc_anchor_c",
        Anchor::Right => "signcut.wcalc_anchor_r",
        Anchor::BottomLeft => "signcut.wcalc_anchor_bl",
        Anchor::Bottom => "signcut.wcalc_anchor_b",
        Anchor::BottomRight => "signcut.wcalc_anchor_br",
    })
}

fn anchor_row(ui: &mut Ui, id: &str, label: &str, anchor: &mut Anchor) -> bool {
    kit::row(ui, label, |ui| {
        let opts: Vec<(Anchor, String)> =
            Anchor::ALL.iter().map(|a| (*a, anchor_label(*a))).collect();
        kit::choice(ui, id, anchor, &opts)
    })
}

/// A box with a double arrow across it (`horizontal`, vertical or
/// both): the pictures of the Set Size buttons.
fn size_picture(p: &Painter, r: Rect, c: Color32, horizontal: bool, vertical: bool) {
    let s = Stroke::new(1.0, c);
    let b = r.shrink(2.0);
    p.rect_stroke(b, 0.0, s, egui::StrokeKind::Middle);
    let arrow = |a: Pos2, z: Pos2| {
        p.line_segment([a, z], s);
        let d = (z - a).normalized() * 3.0;
        let n = Vec2::new(-d.y, d.x);
        for (tip, back) in [(z, -d), (a, d)] {
            p.line_segment([tip, tip + back + n], s);
            p.line_segment([tip, tip + back - n], s);
        }
    };
    let m = b.center();
    if horizontal {
        arrow(
            Pos2::new(b.left() + 2.0, m.y),
            Pos2::new(b.right() - 2.0, m.y),
        );
    }
    if vertical {
        arrow(
            Pos2::new(m.x, b.top() + 2.0),
            Pos2::new(m.x, b.bottom() - 2.0),
        );
    }
}

/// A grey box with a red border (a master object), with an arrow from
/// a black box when `replace`.
fn master_picture(p: &Painter, r: Rect, c: Color32, replace: bool) {
    let red = Color32::from_rgb(0xD0, 0x20, 0x20);
    if replace {
        let a = Rect::from_min_size(r.left_center() - Vec2::new(0.0, 4.0), Vec2::splat(8.0));
        p.rect_filled(a, 0.0, c);
        p.line_segment(
            [a.right_center(), a.right_center() + Vec2::new(4.0, 0.0)],
            Stroke::new(1.0, c),
        );
        let b = Rect::from_min_size(r.right_center() - Vec2::new(7.0, 4.0), Vec2::splat(8.0));
        p.rect_filled(b, 0.0, Color32::from_rgb(0xF0, 0x80, 0x80));
        p.rect_stroke(b, 0.0, Stroke::new(1.0, red), egui::StrokeKind::Inside);
    } else {
        let b = Rect::from_center_size(r.center(), Vec2::new(14.0, 9.0));
        p.rect_filled(b, 0.0, Color32::from_gray(190));
        p.rect_stroke(b, 0.0, Stroke::new(1.5, red), egui::StrokeKind::Inside);
    }
}

/// Set Size: three buttons (width only, height only, both), the fixed
/// point, the way the size is worked out, the turn and the units.
pub(super) fn set_size(app: &mut App, ui: &mut Ui) -> bool {
    let u = app.units;
    let has = !app.selection.is_empty();
    let mut sides = None;
    kit::group(ui, &tr("signcut.set_size"), |ui| {
        for (key, s, h, v) in [
            ("signcut.wcalc_width_only", Sides::Width, true, false),
            ("signcut.wcalc_height_only", Sides::Height, false, true),
            ("signcut.wcalc_both", Sides::Both, true, true),
        ] {
            if picture_button(ui, &tr(key), has, |p, r, c| size_picture(p, r, c, h, v)) {
                sides = Some(s);
            }
        }
        kit::separator(ui);
        let s = &mut app.signcut;
        anchor_row(
            ui,
            "signcut_wcalc_size_anchor",
            &tr("signcut.wcalc_reference_position"),
            &mut s.size.anchor,
        );
    });
    let s = &mut app.signcut;
    kit::group(ui, &tr("signcut.wcalc_size_by"), |ui| {
        ui.radio_value(
            &mut s.size_mode,
            SizeMode::Largest,
            tr("signcut.wcalc_mode_largest"),
        );
        ui.radio_value(
            &mut s.size_mode,
            SizeMode::Smallest,
            tr("signcut.wcalc_mode_smallest"),
        );
        ui.checkbox(&mut s.size.proportional, tr("signcut.proportional"));
        ui.radio_value(
            &mut s.size_mode,
            SizeMode::Size,
            tr("signcut.wcalc_mode_size"),
        );
        ui.add_enabled_ui(s.size_mode == SizeMode::Size, |ui| {
            kit::row(ui, &tr("signcut.width"), |ui| {
                kit::length(ui, u, &mut s.size_width, 0.001, 1_000_000.0)
            });
            kit::row(ui, &tr("signcut.height"), |ui| {
                kit::length(ui, u, &mut s.size_height, 0.001, 1_000_000.0)
            });
        });
        ui.radio_value(
            &mut s.size_mode,
            SizeMode::Change,
            tr("signcut.wcalc_mode_change"),
        );
        ui.add_enabled_ui(s.size_mode == SizeMode::Change, |ui| {
            kit::row(ui, &tr("signcut.wcalc_dx"), |ui| {
                kit::length(ui, u, &mut s.size_dx, -1_000_000.0, 1_000_000.0)
            });
            kit::row(ui, &tr("signcut.wcalc_dy"), |ui| {
                kit::length(ui, u, &mut s.size_dy, -1_000_000.0, 1_000_000.0)
            });
        });
        ui.radio_value(
            &mut s.size_mode,
            SizeMode::Scale,
            tr("signcut.wcalc_mode_scale"),
        );
        ui.add_enabled_ui(s.size_mode == SizeMode::Scale, |ui| {
            kit::row(ui, &tr("signcut.wcalc_scale"), |ui| {
                kit::number(ui, &mut s.size_scale, 0.001, 100_000.0, 3)
            });
        });
        kit::separator(ui);
        kit::row(ui, &tr("signcut.wcalc_change"), |ui| {
            let opts = [
                (SizeRule::Exact, tr("signcut.rule_exact")),
                (SizeRule::ShrinkOnly, tr("signcut.rule_shrink")),
                (SizeRule::GrowOnly, tr("signcut.rule_grow")),
            ];
            kit::choice(ui, "signcut_wcalc_size_rule", &mut s.size.rule, &opts)
        });
        ui.checkbox(&mut s.size_rotate, tr("signcut.wcalc_rotate"));
        ui.add_enabled_ui(s.size_rotate, |ui| {
            kit::row(ui, &tr("signcut.wcalc_angle"), |ui| {
                kit::number(ui, &mut s.size_angle, -360.0, 360.0, 3)
            });
        });
    });
    kit::units_row(ui, &mut app.units);
    if let Some(sides) = sides {
        app.signcut_resize(sides);
    }
    matches!(
        kit::bottom(ui, Buttons::Close, true, |_| {}),
        Some(Pressed::Close)
    )
}

/// Reference Scale: the reference object and what it measures, the
/// real size it stands for, the objects scaled and the fixed point.
/// Apply scales, OK scales and closes, Cancel closes.
pub(super) fn reference_scale(app: &mut App, ui: &mut Ui) -> bool {
    let u = app.units;
    let one = app.selection.len() == 1;
    let current = app.signcut_reference_value();
    if current.is_none() {
        app.signcut.reference = None;
    }
    // The reference on the drawing.
    if let Some((id, _)) = app.signcut.reference {
        if let Some(s) = app.doc().shape_on_page(id) {
            app.docker_preview.push(s.bounds().to_path(0.1));
        }
    }
    let mut pick = false;
    let mut remeasure = false;
    kit::group(ui, &tr("signcut.wcalc_reference"), |ui| {
        let s = &mut app.signcut;
        kit::row(ui, &tr("signcut.measure"), |ui| {
            let opts = [
                (RefMeasure::Width, tr("signcut.width")),
                (RefMeasure::Height, tr("signcut.height")),
                (RefMeasure::Length, tr("signcut.length")),
            ];
            if kit::choice(ui, "signcut_wcalc_ref_measure", &mut s.ref_measure, &opts) {
                remeasure = true;
            }
        });
        if picture_button(ui, &tr("signcut.pick_reference"), one, |p, r, c| {
            master_picture(p, r, c, false)
        }) {
            pick = true;
        }
        kit::row(ui, &tr("signcut.wcalc_reference_now"), |ui| {
            readout(
                ui,
                &current.map_or_else(|| "-".into(), |v| fmt_len(u, v)),
                false,
                false,
            )
        });
        ui.add_enabled_ui(current.is_some(), |ui| {
            kit::row(ui, &tr("signcut.wcalc_real_size"), |ui| {
                kit::length(ui, u, &mut s.ref_wanted, 0.001, 1_000_000_000.0)
            });
        });
        let factor = current.and_then(|c| sizing::reference_factor(c, s.ref_wanted));
        kit::row(ui, &tr("signcut.wcalc_factor"), |ui| {
            let text = match (current, factor) {
                (None, _) => "-".to_string(),
                (Some(_), None) => "1 (100 %)".to_string(),
                (Some(_), Some(f)) => {
                    format!("{} ({} %)", super::num(f, 4), super::num(f * 100.0, 2))
                }
            };
            readout(ui, &text, true, false)
        });
    });
    let s = &mut app.signcut;
    kit::group(ui, &tr("signcut.wcalc_mode_scale"), |ui| {
        kit::row(ui, &tr("signcut.objects"), |ui| {
            let opts = [
                (true, tr("signcut.wcalc_whole_page")),
                (false, tr("signcut.wcalc_selection_and_ref")),
            ];
            kit::choice(ui, "signcut_wcalc_ref_scope", &mut s.ref_whole_page, &opts)
        });
        anchor_row(
            ui,
            "signcut_wcalc_ref_anchor",
            &tr("signcut.wcalc_fixed_point"),
            &mut s.ref_anchor,
        );
    });
    if current.is_none() {
        ui.label(
            RichText::new(tr("signcut.no_reference"))
                .size(11.0)
                .color(Tokens::text_dim()),
        );
    }
    kit::units_row(ui, &mut app.units);
    if pick {
        app.signcut_pick_reference();
    } else if remeasure {
        if let Some(v) = app.signcut_reference_value() {
            if let Some((id, _)) = app.signcut.reference {
                app.signcut.reference = Some((id, v));
            }
            app.signcut.ref_wanted = v;
        }
    }
    let can = current
        .and_then(|c| sizing::reference_factor(c, app.signcut.ref_wanted))
        .is_some();
    match kit::bottom(ui, Buttons::ApplyOkCancel, can, |_| {}) {
        Some(Pressed::Apply) => {
            app.signcut_reference_scale();
            false
        }
        Some(Pressed::Ok) => {
            app.signcut_reference_scale();
            true
        }
        Some(_) => true,
        None => false,
    }
}

/// Replace: tabs for one master or several, the buttons that set them
/// and replace, how the copies are turned and sized.
pub(super) fn replace(app: &mut App, ui: &mut Ui) -> bool {
    let masters = app.signcut_current_masters();
    for id in &masters {
        if let Some(s) = app.doc().shape_on_page(*id) {
            app.docker_preview.push(s.bounds().to_path(0.1));
        }
    }
    let mut multi = app.signcut.replace_multi;
    tabs(
        ui,
        &mut multi,
        &[
            (false, tr("signcut.wcalc_single_master")),
            (true, tr("signcut.wcalc_several_masters")),
        ],
    );
    app.signcut.replace_multi = multi;
    let one = app.selection.len() == 1;
    let any = !app.selection.is_empty();
    let can_replace = !masters.is_empty() && app.selection.iter().any(|id| !masters.contains(id));
    let mut set = false;
    let mut run = false;
    let (set_key, run_key) = if multi {
        ("signcut.wcalc_set_masters", "signcut.wcalc_replace_masters")
    } else {
        ("signcut.wcalc_set_master", "signcut.wcalc_replace_master")
    };
    let set_text = if masters.is_empty() {
        format!("! {}", tr(set_key))
    } else {
        tr(set_key)
    };
    if picture_button(ui, &set_text, if multi { any } else { one }, |p, r, c| {
        master_picture(p, r, c, false)
    }) {
        set = true;
    }
    ui.add_space(2.0);
    if picture_button(ui, &tr(run_key), can_replace, |p, r, c| {
        master_picture(p, r, c, true)
    }) {
        run = true;
    }
    ui.add_space(4.0);
    let s = &mut app.signcut;
    if multi {
        kit::row(ui, &tr("signcut.wcalc_option"), |ui| {
            let opts = [
                (MasterPick::InTurn, tr("signcut.wcalc_pick_turn")),
                (MasterPick::ClosestShape, tr("signcut.wcalc_pick_shape")),
            ];
            kit::choice(ui, "signcut_wcalc_pick", &mut s.replace_pick, &opts)
        });
    }
    kit::row(ui, &tr("signcut.wcalc_preserve_angle"), |ui| {
        let opts = [(false, tr("signcut.nw_no")), (true, tr("signcut.nw_yes"))];
        kit::choice(ui, "signcut_wcalc_turn", &mut s.replace_turn, &opts)
    });
    kit::row(ui, &tr("signcut.wcalc_mode_size"), |ui| {
        let opts = [
            (ReplaceFit::Keep, tr("signcut.fit_keep")),
            (ReplaceFit::Fit, tr("signcut.fit_fit")),
            (ReplaceFit::Stretch, tr("signcut.fit_stretch")),
        ];
        kit::choice(ui, "signcut_wcalc_fit", &mut s.replace_fit, &opts)
    });
    ui.checkbox(&mut s.replace_keep, tr("signcut.replace_keep"));
    let note = if masters.is_empty() {
        tr("signcut.no_master")
    } else {
        trf(
            "signcut.wcalc_masters_set",
            &[("n", &masters.len().to_string())],
        )
    };
    ui.label(RichText::new(note).size(11.0).color(Tokens::text_dim()));
    if set {
        if multi {
            app.signcut_pick_masters();
        } else {
            app.signcut_pick_master();
        }
    }
    if run {
        app.signcut_replace();
    }
    let mut show = false;
    let pressed = kit::bottom(ui, Buttons::Close, true, |ui| {
        show = kit::push(ui, &tr("signcut.wcalc_select_master"), !masters.is_empty());
    });
    if show {
        app.select(masters);
    }
    matches!(pressed, Some(Pressed::Close))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{app_with_rects, frame};
    use super::*;

    #[test]
    fn the_arrange_windows_draw_in_every_mode_and_tab() {
        let ctx = crate::theme::ui_context();
        let mut app = app_with_rects();
        let rev = app.engine.revision();
        for mode in SizeMode::ALL {
            app.signcut.size_mode = mode;
            app.signcut.size_rotate = mode == SizeMode::Scale;
            frame(&ctx, |ui| {
                set_size(&mut app, ui);
            });
        }
        for multi in [false, true] {
            app.signcut.replace_multi = multi;
            frame(&ctx, |ui| {
                replace(&mut app, ui);
            });
        }
        frame(&ctx, |ui| {
            reference_scale(&mut app, ui);
        });
        // A reference: its box is marked on the drawing.
        let first = app.selection[0];
        app.select(vec![first]);
        assert!(app.signcut_pick_reference());
        app.docker_preview.clear();
        frame(&ctx, |ui| {
            reference_scale(&mut app, ui);
        });
        assert_eq!(app.docker_preview.len(), 1);
        assert_eq!(app.engine.revision(), rev);
    }

    #[test]
    fn reference_scale_apply_is_one_step() {
        let mut app = app_with_rects();
        let first = app.selection[0];
        app.select(vec![first]);
        assert!(app.signcut_pick_reference());
        app.signcut.ref_wanted = 60.0;
        assert_eq!(app.signcut_reference_scale(), Some(2.0));
        assert_eq!(app.engine.undo_label(), Some("Reference Scale"));
        // Now it measures what it stands for: nothing more to do.
        assert_eq!(
            app.signcut_reference_value()
                .and_then(|c| sizing::reference_factor(c, app.signcut.ref_wanted)),
            None
        );
    }
}
