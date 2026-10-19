//! The windows of Print and Cut Marks and Crop Marks: a preview of the
//! marks at the left (always current: the marks are quick to work out),
//! the settings at the right, OK (add them in one step) and Cancel.

use super::{add_all, col, columns, rect_path, scope};
use crate::app::App;
use crate::i18n::{tr, trf};
use crate::signcut_cut::Contour;
use crate::ui::signcut_kit::{self as kit, colors, Buttons, Pressed, Scene};
use egui::Ui;
use tracedraw_signcut::cut::marks::MarkStyle;

/// Print and Cut Marks.
pub fn print_and_cut(app: &mut App, ui: &mut Ui) -> bool {
    let basis = app.sc_cut_registration_basis();
    if !matches!(&app.sc_cut.win.reg, Some((b, _)) if *b == basis) {
        let r = app.sc_cut_registration_compute();
        app.sc_cut.win.reg = Some((basis, r));
    }
    let page = app.page_rect();
    let reg_page = app.sc_cut.reg_page;
    let (scene, strip, has) = match &app.sc_cut.win.reg {
        Some((_, r)) => {
            let mut scene = Scene::default();
            if reg_page {
                scene.add(rect_path(page), colors::SHEET, 1.0);
            } else if let Some(area) = r.area {
                scene.add(rect_path(area), col::GUIDE, 1.0);
            }
            add_all(&mut scene, &r.objects, colors::ORIGINAL, 1.0);
            add_all(&mut scene, &r.contour, col::CONTOUR, 1.5);
            add_all(&mut scene, &r.marks, colors::PART, 2.0);
            let strip = if r.marks.is_empty() {
                tr("signcut.cut_nothing")
            } else {
                trf(
                    "signcut.wcut_marks_strip",
                    &[
                        ("n", &r.marks.len().to_string()),
                        ("c", &r.contour.len().to_string()),
                    ],
                )
            };
            (scene, strip, !r.marks.is_empty())
        }
        None => (Scene::default(), tr("signcut.cut_nothing"), false),
    };
    columns(app, ui, "print_and_cut", &scene, &strip, reg_settings);
    match kit::bottom(ui, Buttons::OkCancel, has, |_| {}) {
        Some(Pressed::Ok) => {
            let basis = app.sc_cut_registration_basis();
            let r = match app.sc_cut.win.reg.take() {
                Some((b, r)) if b == basis => r,
                _ => app.sc_cut_registration_compute(),
            };
            app.sc_cut_registration_commit(r);
            true
        }
        Some(_) => {
            app.sc_cut.win.reg = None;
            true
        }
        None => false,
    }
}

fn reg_settings(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_cut;
    kit::group(ui, &tr("signcut.wcut_marks"), |ui| {
        kit::row(ui, &tr("signcut.cut_around"), |ui| {
            let opts = [
                (false, tr("signcut.cut_around_objects")),
                (true, tr("signcut.cut_around_page")),
            ];
            kit::choice(ui, "wcut_reg_around", &mut s.reg_page, &opts)
        });
        kit::row(ui, &tr("signcut.cut_style"), |ui| {
            let opts = [
                (MarkStyle::Square, tr("signcut.cut_style_square")),
                (MarkStyle::Circle, tr("signcut.cut_style_circle")),
                (MarkStyle::Corner, tr("signcut.cut_style_corner")),
            ];
            kit::choice(ui, "wcut_reg_style", &mut s.reg.style, &opts)
        });
        if s.reg.style == MarkStyle::Corner {
            kit::row(ui, &tr("signcut.cut_corner_length"), |ui| {
                kit::length(ui, u, &mut s.reg.corner_length, 0.1, 1000.0)
            });
            kit::row(ui, &tr("signcut.cut_corner_width"), |ui| {
                kit::length(ui, u, &mut s.reg.corner_width, 0.01, 100.0)
            });
        }
        kit::row(ui, &tr("signcut.cut_mark_size"), |ui| {
            kit::length(ui, u, &mut s.reg.size, 0.1, 1000.0)
        });
        kit::row(ui, &tr("signcut.cut_offset"), |ui| {
            kit::length(ui, u, &mut s.reg.offset, 0.0, 1000.0)
        });
        kit::separator(ui);
        kit::row(ui, &tr("signcut.cut_mark_spacing"), |ui| {
            kit::length(ui, u, &mut s.reg.spacing, 0.0, 100_000.0)
        });
    });
    kit::group(ui, &tr("signcut.cut_contour"), |ui| {
        kit::row(ui, &tr("signcut.wcut_contour_from"), |ui| {
            let opts = [
                (Contour::Outlines, tr("signcut.cut_contour_outlines")),
                (Contour::BoundingBox, tr("signcut.cut_contour_box")),
                (Contour::None, tr("signcut.cut_contour_none")),
            ];
            kit::choice(ui, "wcut_reg_contour", &mut s.contour, &opts)
        });
    });
    scope(app, ui);
}

/// Crop Marks.
pub fn crop(app: &mut App, ui: &mut Ui) -> bool {
    let basis = app.sc_cut_crop_basis();
    if !matches!(&app.sc_cut.win.crop, Some((b, _)) if *b == basis) {
        let r = app.sc_cut_crop_compute();
        app.sc_cut.win.crop = Some((basis, r));
    }
    let (scene, strip, has) = match &app.sc_cut.win.crop {
        Some((_, r)) => {
            let mut scene = Scene::default();
            add_all(&mut scene, &r.objects, colors::ORIGINAL, 1.0);
            for b in &r.boxes {
                scene.add(rect_path(*b), col::GUIDE, 1.0);
            }
            add_all(&mut scene, &r.marks, colors::PART, 1.5);
            let strip = if r.marks.is_empty() {
                tr("signcut.cut_nothing")
            } else {
                trf(
                    "signcut.wcut_crop_strip",
                    &[("n", &r.marks.len().to_string())],
                )
            };
            (scene, strip, !r.marks.is_empty())
        }
        None => (Scene::default(), tr("signcut.cut_nothing"), false),
    };
    columns(app, ui, "crop_marks", &scene, &strip, crop_settings);
    match kit::bottom(ui, Buttons::OkCancel, has, |_| {}) {
        Some(Pressed::Ok) => {
            let basis = app.sc_cut_crop_basis();
            let marks = match app.sc_cut.win.crop.take() {
                Some((b, r)) if b == basis => r.marks,
                _ => app.sc_cut_crop_compute().marks,
            };
            app.sc_cut_crop_commit(marks);
            true
        }
        Some(_) => {
            app.sc_cut.win.crop = None;
            true
        }
        None => false,
    }
}

fn crop_settings(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let s = &mut app.sc_cut;
    kit::group(ui, &tr("signcut.cut_crop"), |ui| {
        kit::row(ui, &tr("signcut.length"), |ui| {
            kit::length(ui, u, &mut s.crop.length, 0.1, 1000.0)
        });
        kit::row(ui, &tr("signcut.cut_offset"), |ui| {
            kit::length(ui, u, &mut s.crop.offset, 0.0, 1000.0)
        });
        kit::row(ui, &tr("signcut.cut_crop_width"), |ui| {
            kit::length(ui, u, &mut s.crop_width, 0.01, 100.0)
        });
        ui.checkbox(&mut s.crop_each, tr("signcut.cut_crop_each"));
    });
    scope(app, ui);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signcut_tools::ToolId;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::ShapeKind;

    fn app_with_parts() -> App {
        let mut app = App::headless();
        let mut ids = Vec::new();
        for x in [0.0, 50.0] {
            ids.push(
                app.new_shape(ShapeKind::Rect {
                    rect: Rect::new(x, 50.0, x + 20.0, 70.0),
                    radius: 0.0,
                    corners: None,
                })
                .expect("layer"),
            );
        }
        app.select(ids);
        app
    }

    #[test]
    fn print_and_cut_window_draws_every_style_and_previews_live() {
        let mut app = app_with_parts();
        let rev = app.engine.revision();
        for style in [MarkStyle::Square, MarkStyle::Circle, MarkStyle::Corner] {
            for page in [false, true] {
                app.sc_cut.reg.style = style;
                app.sc_cut.reg_page = page;
                super::super::draw_headless(&mut app, ToolId::PrintAndCut);
                let (b, r) = app.sc_cut.win.reg.as_ref().expect("worked out");
                assert_eq!(*b, app.sc_cut_registration_basis());
                assert_eq!(r.marks.len(), 4, "{style:?} {page}");
            }
        }
        assert_eq!(app.engine.revision(), rev, "the preview changes nothing");
        let r = app.sc_cut.win.reg.take().expect("kept").1;
        assert_eq!(app.sc_cut_registration_commit(r), 4);
        assert_eq!(app.engine.undo_label(), Some("Registration Marks"));
    }

    #[test]
    fn crop_window_previews_each_object_or_all() {
        let mut app = app_with_parts();
        let rev = app.engine.revision();
        for each in [false, true] {
            app.sc_cut.crop_each = each;
            super::super::draw_headless(&mut app, ToolId::CropMarks);
            let r = &app.sc_cut.win.crop.as_ref().expect("worked out").1;
            assert_eq!(r.boxes.len(), if each { 2 } else { 1 });
        }
        assert_eq!(app.engine.revision(), rev, "the preview changes nothing");
        let r = app.sc_cut.win.crop.take().expect("kept").1;
        assert_eq!(app.sc_cut_crop_commit(r.marks), 8);
        assert_eq!(app.engine.undo_label(), Some("Crop Marks"));
    }
}
