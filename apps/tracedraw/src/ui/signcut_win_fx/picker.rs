//! The method list of the effects windows: a tall drop-down whose button
//! and items show a small picture of the method next to its name, and the
//! pictures themselves (drawn here, simple line drawings).

use crate::i18n::tr;
use crate::theme::Tokens;
use egui::{
    Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2,
};

/// Draws a method's picture into a square.
pub type Picture = fn(&Painter, Rect, Color32);

/// Height of the button and of each item.
const ROW_H: f32 = 52.0;
/// Side of a picture.
const PIC: f32 = 40.0;

/// The id of the list's pop-up, for `egui::Popup::open_id`.
pub fn popup_id(salt: &str) -> egui::Id {
    egui::Id::new(("signcut_fx_method", salt))
}

/// The method drop-down: the button shows the chosen method's picture,
/// the caption "Method" and its name; a click opens the list of every
/// method with its picture. Returns true when the choice changed.
pub fn method_picker<T: Copy + PartialEq>(
    ui: &mut Ui,
    salt: &str,
    value: &mut T,
    items: &[(T, String, Picture)],
) -> bool {
    let before = *value;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::click());
    let fill = if resp.hovered() {
        Tokens::hovered_row()
    } else {
        Tokens::field()
    };
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::ZERO,
        fill,
        Stroke::new(1.0, Tokens::control_border()),
        StrokeKind::Inside,
    );
    if let Some((_, name, pic)) = items.iter().find(|i| i.0 == *value).or(items.first()) {
        draw_item(p, rect, name, *pic, Some(&tr("signcut.wfx_method")));
    }
    // The drop-down arrow.
    let c = Pos2::new(rect.right() - 14.0, rect.center().y);
    p.add(egui::Shape::convex_polygon(
        vec![
            c + Vec2::new(-5.0, -3.0),
            c + Vec2::new(5.0, -3.0),
            c + Vec2::new(0.0, 3.0),
        ],
        Tokens::text(),
        Stroke::NONE,
    ));
    egui::Popup::from_toggle_button_response(&resp)
        .id(popup_id(salt))
        .width(width)
        .show(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            for (v, name, pic) in items {
                let (r, item) = ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::click());
                let picked = *v == *value;
                if picked || item.hovered() {
                    let fill = if picked {
                        Tokens::selected_row()
                    } else {
                        Tokens::hovered_row()
                    };
                    ui.painter().rect_filled(r, 0.0, fill);
                }
                draw_item(ui.painter(), r, name, *pic, None);
                if item.clicked() {
                    *value = *v;
                }
            }
        });
    *value != before
}

/// A picture with the name at its right (under a small caption when
/// given).
fn draw_item(p: &Painter, r: Rect, name: &str, pic: Picture, caption: Option<&str>) {
    let pr = Rect::from_min_size(
        Pos2::new(r.left() + 6.0, r.center().y - PIC / 2.0),
        Vec2::splat(PIC),
    );
    pic(p, pr, Tokens::text());
    let x = pr.right() + 10.0;
    match caption {
        Some(c) => {
            p.text(
                Pos2::new(x, r.center().y - 2.0),
                Align2::LEFT_BOTTOM,
                c,
                FontId::proportional(10.0),
                Tokens::text_dim(),
            );
            p.text(
                Pos2::new(x, r.center().y),
                Align2::LEFT_TOP,
                name,
                FontId::proportional(14.0),
                Tokens::text(),
            );
        }
        None => {
            p.text(
                Pos2::new(x, r.center().y),
                Align2::LEFT_CENTER,
                name,
                FontId::proportional(13.0),
                Tokens::text(),
            );
        }
    }
}

fn line(p: &Painter, a: Pos2, b: Pos2, c: Color32) {
    p.line_segment([a, b], Stroke::new(1.2, c));
}

fn frame(p: &Painter, r: Rect, c: Color32) {
    p.rect_stroke(r, 0.0, Stroke::new(1.2, c), StrokeKind::Inside);
}

/// Pieces of a jigsaw: a square in four with a knob on each inner edge.
pub fn puzzle(p: &Painter, r: Rect, c: Color32) {
    let r = r.shrink(4.0);
    frame(p, r, c);
    let m = r.center();
    line(p, Pos2::new(m.x, r.top()), Pos2::new(m.x, r.bottom()), c);
    line(p, Pos2::new(r.left(), m.y), Pos2::new(r.right(), m.y), c);
    let q = r.width() / 4.0;
    for k in [
        Pos2::new(m.x + 3.0, r.top() + q),
        Pos2::new(m.x - 3.0, r.bottom() - q),
        Pos2::new(r.left() + q, m.y - 3.0),
        Pos2::new(r.right() - q, m.y + 3.0),
    ] {
        p.circle(k, 3.0, Tokens::field(), Stroke::new(1.2, c));
    }
}

/// Circles of several sizes filling a rounded square.
pub fn circles(p: &Painter, r: Rect, c: Color32) {
    let r = r.shrink(3.0);
    p.rect_stroke(r, 6.0, Stroke::new(1.2, c), StrokeKind::Inside);
    let m = r.center();
    for (dx, dy, rad) in [
        (-7.0, -6.0, 6.0),
        (7.0, -7.0, 4.5),
        (-5.0, 8.0, 5.0),
        (8.0, 6.0, 5.5),
        (1.5, 0.5, 2.0),
    ] {
        p.circle_stroke(m + Vec2::new(dx, dy), rad, Stroke::new(1.2, c));
    }
}

/// A block with its flat shadow falling down to the right.
pub fn shadow(p: &Painter, r: Rect, c: Color32) {
    let body = Rect::from_min_size(r.min + Vec2::new(8.0, 4.0), Vec2::new(16.0, 26.0));
    let grey = Color32::from_gray(150);
    p.add(egui::Shape::convex_polygon(
        vec![
            body.left_bottom(),
            body.right_bottom(),
            body.right_bottom() + Vec2::new(12.0, 6.0),
            body.left_bottom() + Vec2::new(12.0, 6.0),
        ],
        grey,
        Stroke::NONE,
    ));
    p.rect_filled(
        body.translate(Vec2::new(4.0, 3.0)),
        0.0,
        grey.gamma_multiply(0.6),
    );
    p.rect_filled(body, 0.0, c);
}

/// Small squares scattered at different turns and sizes.
pub fn copies(p: &Painter, r: Rect, c: Color32) {
    let m = r.center();
    for (dx, dy, s, a) in [
        (-10.0, -10.0, 5.0, 0.3_f32),
        (9.0, -11.0, 4.0, 1.0),
        (-11.0, 9.0, 4.5, 0.7),
        (10.0, 8.0, 6.0, 0.1),
        (0.0, 0.0, 3.5, 1.3),
    ] {
        let o = m + Vec2::new(dx, dy);
        let pts: Vec<Pos2> = (0..4)
            .map(|k| {
                let t = a + k as f32 * std::f32::consts::FRAC_PI_2;
                o + Vec2::new(t.cos(), t.sin()) * s
            })
            .collect();
        p.add(egui::Shape::closed_line(pts, Stroke::new(1.2, c)));
    }
}

/// Parallel lines inside a square.
pub fn parallel(p: &Painter, r: Rect, c: Color32) {
    let r = r.shrink(4.0);
    frame(p, r, c);
    let mut y = r.top() + 5.0;
    while y < r.bottom() - 2.0 {
        line(
            p,
            Pos2::new(r.left() + 3.0, y),
            Pos2::new(r.right() - 3.0, y),
            c,
        );
        y += 5.0;
    }
}

/// One line going back and forth inside a square.
pub fn snake(p: &Painter, r: Rect, c: Color32) {
    let r = r.shrink(4.0);
    frame(p, r, c);
    let (l, rt) = (r.left() + 3.0, r.right() - 3.0);
    let mut pts = Vec::new();
    let mut y = r.top() + 5.0;
    let mut right = true;
    while y < r.bottom() - 2.0 {
        let (a, b) = if right { (l, rt) } else { (rt, l) };
        pts.push(Pos2::new(a, y));
        pts.push(Pos2::new(b, y));
        right = !right;
        y += 5.0;
    }
    p.add(egui::Shape::line(pts, Stroke::new(1.2, c)));
}

/// Crossed lines inside a square.
pub fn crossed(p: &Painter, r: Rect, c: Color32) {
    let r = r.shrink(4.0);
    frame(p, r, c);
    let mut t = 5.0;
    while t < r.width() - 2.0 {
        line(
            p,
            Pos2::new(r.left() + 3.0, r.top() + t),
            Pos2::new(r.right() - 3.0, r.top() + t),
            c,
        );
        line(
            p,
            Pos2::new(r.left() + t, r.top() + 3.0),
            Pos2::new(r.left() + t, r.bottom() - 3.0),
            c,
        );
        t += 5.0;
    }
}

/// A grid of cells.
pub fn grid(p: &Painter, r: Rect, c: Color32) {
    let r = r.shrink(4.0);
    for k in 0..=3 {
        let f = k as f32 / 3.0;
        let x = r.left() + r.width() * f;
        let y = r.top() + r.height() * f;
        line(p, Pos2::new(x, r.top()), Pos2::new(x, r.bottom()), c);
        line(p, Pos2::new(r.left(), y), Pos2::new(r.right(), y), c);
    }
}

/// A box divided by lines into equal strips.
pub fn dividing(p: &Painter, r: Rect, c: Color32) {
    let r = r.shrink(4.0);
    p.rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Tokens::text_dim()),
        StrokeKind::Inside,
    );
    for k in 1..=3 {
        let y = r.top() + r.height() * k as f32 / 4.0;
        line(p, Pos2::new(r.left(), y), Pos2::new(r.right(), y), c);
    }
}
