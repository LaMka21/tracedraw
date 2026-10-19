//! The Sign & Cut toolbar: nine buttons, each with a small arrow. Eight
//! open a menu of their tools; the last shows or hides the Sign & Cut
//! panel. Window > Toolbars > Sign & Cut shows the bar.

use crate::app::{App, DockerTab};
use crate::i18n::tr;
use crate::signcut_tools::Group;
use crate::theme::Tokens;
use egui::{Color32, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// Size of a button (icon and arrow).
const BUTTON: Vec2 = Vec2::new(40.0, 30.0);

pub fn toolbar(app: &mut App, ui: &mut Ui) {
    let doc = app.has_document() && !app.show_welcome;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (i, g) in Group::ALL.into_iter().enumerate() {
            if i > 0 {
                let (r, _) = ui.allocate_exact_size(Vec2::new(5.0, BUTTON.y), Sense::hover());
                ui.painter().vline(
                    r.center().x,
                    (r.top() + 5.0)..=(r.bottom() - 5.0),
                    Stroke::new(1.0, Tokens::border()),
                );
            }
            let panel_on = app.show_dockers && app.docker_tab == DockerTab::SignCut;
            let on = g == Group::Panel && panel_on;
            let (rect, resp) = ui.allocate_exact_size(BUTTON, Sense::click());
            if resp.hovered() || on {
                ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
            }
            let icon = Rect::from_min_size(rect.min + Vec2::new(4.0, 5.0), Vec2::splat(20.0));
            draw_icon(ui.painter(), icon, g);
            if g != Group::Panel {
                let c = Pos2::new(rect.right() - 7.0, rect.bottom() - 7.0);
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        c + Vec2::new(-3.0, -2.0),
                        c + Vec2::new(3.0, -2.0),
                        c + Vec2::new(0.0, 2.0),
                    ],
                    Tokens::text(),
                    Stroke::NONE,
                ));
            }
            let resp = resp.on_hover_text(tr(g.key()));
            if g == Group::Panel {
                if resp.clicked() {
                    app.toggle_docker(DockerTab::SignCut);
                }
                continue;
            }
            egui::Popup::menu(&resp)
                .id(egui::Id::new(("signcut_toolbar_menu", g.key())))
                .style(crate::ui::menus::menu_popup_style)
                .show(|ui| {
                    crate::ui::menus::body(ui, |ui| {
                        for t in g.tools() {
                            if crate::ui::menus::item(ui, t.key(), "", doc) {
                                app.signcut_tool(*t);
                            }
                        }
                    })
                });
        }
    });
}

/// The group's picture in `r` (20 x 20): simple line drawings.
pub fn draw_icon(p: &Painter, r: Rect, g: Group) {
    let ink = Tokens::text();
    let blue = Color32::from_rgb(0x1E, 0x6F, 0xD9);
    let red = Color32::from_rgb(0xD9, 0x2B, 0x2B);
    let s = Stroke::new(1.4, ink);
    let at = |x: f32, y: f32| r.min + Vec2::new(x, y);
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(at(x0, y0), at(x1, y1));
    match g {
        Group::Nesting => {
            // A sheet with parts packed into its corner.
            p.rect_stroke(rect(1.0, 1.0, 19.0, 19.0), 0.0, s, egui::StrokeKind::Inside);
            p.rect_filled(rect(3.0, 11.0, 10.0, 17.0), 0.0, blue);
            p.circle_filled(at(14.0, 14.0), 3.0, blue);
            p.rect_filled(rect(3.0, 4.0, 8.0, 9.0), 0.0, blue);
        }
        Group::Cutting => {
            // A blade cutting along a dashed line.
            for k in 0..4 {
                let x = 2.0 + k as f32 * 4.5;
                p.line_segment([at(x, 17.0), at(x + 2.5, 17.0)], Stroke::new(1.4, red));
            }
            p.line_segment([at(4.0, 14.0), at(15.0, 3.0)], Stroke::new(2.0, ink));
            p.add(egui::Shape::convex_polygon(
                vec![at(13.0, 2.0), at(18.0, 2.0), at(17.0, 6.0)],
                blue,
                Stroke::NONE,
            ));
        }
        Group::Measure => {
            // A ruler with ticks and a dimension arrow.
            p.rect_stroke(
                rect(1.0, 11.0, 19.0, 18.0),
                0.0,
                s,
                egui::StrokeKind::Inside,
            );
            for k in 0..5 {
                let x = 3.0 + k as f32 * 3.5;
                let h = if k % 2 == 0 { 3.5 } else { 2.0 };
                p.line_segment([at(x, 11.0), at(x, 11.0 + h)], Stroke::new(1.0, ink));
            }
            p.line_segment([at(2.0, 5.0), at(18.0, 5.0)], Stroke::new(1.2, blue));
            p.line_segment([at(2.0, 5.0), at(5.0, 3.0)], Stroke::new(1.2, blue));
            p.line_segment([at(2.0, 5.0), at(5.0, 7.0)], Stroke::new(1.2, blue));
            p.line_segment([at(18.0, 5.0), at(15.0, 3.0)], Stroke::new(1.2, blue));
            p.line_segment([at(18.0, 5.0), at(15.0, 7.0)], Stroke::new(1.2, blue));
        }
        Group::SignMaking => {
            // A letter outline with LED dots inside.
            p.add(egui::Shape::closed_line(
                vec![
                    at(3.0, 18.0),
                    at(3.0, 2.0),
                    at(10.0, 2.0),
                    at(10.0, 13.0),
                    at(17.0, 13.0),
                    at(17.0, 18.0),
                ],
                s,
            ));
            for (x, y) in [(6.5, 5.0), (6.5, 9.5), (6.5, 14.5), (13.5, 15.5)] {
                p.circle_filled(at(x, y), 1.6, red);
            }
        }
        Group::Effects => {
            // A star with hatching lines.
            let c = at(10.0, 10.0);
            let mut pts = Vec::new();
            for k in 0..10 {
                let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
                let rr = if k % 2 == 0 { 9.0 } else { 4.0 };
                pts.push(c + Vec2::new(a.cos(), a.sin()) * rr);
            }
            p.add(egui::Shape::closed_line(pts, s));
            for k in 0..3 {
                let y = 8.0 + k as f32 * 3.0;
                p.line_segment([at(7.0, y), at(13.0, y)], Stroke::new(1.0, blue));
            }
        }
        Group::Construct => {
            // A box's flat pattern: a face with flaps, fold lines dashed.
            p.rect_stroke(
                rect(6.0, 6.0, 14.0, 14.0),
                0.0,
                Stroke::new(1.0, blue),
                egui::StrokeKind::Inside,
            );
            p.add(egui::Shape::closed_line(
                vec![
                    at(6.0, 1.0),
                    at(14.0, 1.0),
                    at(14.0, 6.0),
                    at(19.0, 6.0),
                    at(19.0, 14.0),
                    at(14.0, 14.0),
                    at(14.0, 19.0),
                    at(6.0, 19.0),
                    at(6.0, 14.0),
                    at(1.0, 14.0),
                    at(1.0, 6.0),
                    at(6.0, 6.0),
                ],
                s,
            ));
        }
        Group::Cad => {
            // A bridge gap in an outline and a path with nodes.
            p.line_segment([at(2.0, 17.0), at(8.0, 17.0)], s);
            p.line_segment([at(12.0, 17.0), at(18.0, 17.0)], s);
            p.line_segment([at(2.0, 17.0), at(2.0, 11.0)], s);
            p.line_segment([at(18.0, 17.0), at(18.0, 11.0)], s);
            let pts = [at(3.0, 8.0), at(8.0, 3.0), at(13.0, 7.0), at(18.0, 3.0)];
            p.add(egui::Shape::line(pts.to_vec(), Stroke::new(1.4, blue)));
            for q in pts {
                p.rect_filled(Rect::from_center_size(q, Vec2::splat(3.0)), 0.0, red);
            }
        }
        Group::Arrange => {
            // Objects aligned to a line.
            p.line_segment([at(2.0, 1.0), at(2.0, 19.0)], Stroke::new(1.4, red));
            p.rect_filled(rect(3.0, 3.0, 16.0, 8.0), 0.0, blue);
            p.rect_filled(rect(3.0, 11.0, 11.0, 17.0), 0.0, blue);
        }
        Group::Panel => {
            // A panel with rows.
            p.rect_stroke(rect(2.0, 2.0, 18.0, 18.0), 0.0, s, egui::StrokeKind::Inside);
            p.line_segment([at(8.0, 2.0), at(8.0, 18.0)], s);
            for k in 0..3 {
                let y = 6.0 + k as f32 * 4.0;
                p.line_segment([at(10.0, y), at(16.0, y)], Stroke::new(1.0, ink));
            }
        }
    }
}
