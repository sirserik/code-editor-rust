//! Monochrome vector icons drawn directly through egui's `Painter`.
//!
//! Bundled because JetBrainsMono (the project's main font) lacks the
//! UI symbol glyphs we want — emoji fallback gives colour glyphs we don't
//! want either. Drawing thin strokes scales crisply at any zoom and lets us
//! tint everything with theme colours.

use egui::{Color32, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

/// Inset the icon rect inside its hit area to leave breathing room around the strokes.
fn pad(rect: Rect, frac: f32) -> Rect {
    let s = rect.width().min(rect.height());
    let p = s * frac;
    Rect::from_center_size(rect.center(), Vec2::splat(s - p * 2.0))
}

fn stroke_for(size: f32, color: Color32) -> Stroke {
    Stroke::new((size * 0.085).max(1.25), color)
}

pub fn document(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.2);
    let s = stroke_for(r.width(), color);
    let fold = r.width() * 0.28;
    // Page outline with folded top-right corner
    let tl = r.left_top();
    let tr_outer = Pos2::new(r.right() - fold, r.top());
    let corner_inner = Pos2::new(r.right(), r.top() + fold);
    let br = r.right_bottom();
    let bl = r.left_bottom();
    p.line_segment([tl, tr_outer], s);
    p.line_segment([tr_outer, corner_inner], s);
    p.line_segment([corner_inner, br], s);
    p.line_segment([br, bl], s);
    p.line_segment([bl, tl], s);
    // Fold triangle
    p.line_segment([tr_outer, Pos2::new(r.right() - fold, r.top() + fold)], s);
    p.line_segment([Pos2::new(r.right() - fold, r.top() + fold), corner_inner], s);
}

pub fn search(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.18);
    let s = stroke_for(r.width(), color);
    let radius = r.width() * 0.35;
    let center = Pos2::new(r.left() + radius + r.width() * 0.05, r.top() + radius + r.height() * 0.05);
    p.circle_stroke(center, radius, s);
    // Handle
    let off = radius / std::f32::consts::SQRT_2;
    let handle_start = Pos2::new(center.x + off, center.y + off);
    let handle_end = Pos2::new(r.right(), r.bottom());
    p.line_segment([handle_start, handle_end], s);
}

pub fn branch(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.2);
    let s = stroke_for(r.width(), color);
    let dot = (r.width() * 0.11).max(2.0);
    // Three "git" nodes: top-left, top-right, bottom-left.
    let a = Pos2::new(r.left() + dot, r.top() + dot);
    let b = Pos2::new(r.right() - dot, r.top() + dot);
    let c = Pos2::new(r.left() + dot, r.bottom() - dot);
    p.circle_stroke(a, dot, s);
    p.circle_stroke(b, dot, s);
    p.circle_stroke(c, dot, s);
    // Vertical from a to c
    p.line_segment([Pos2::new(a.x, a.y + dot), Pos2::new(c.x, c.y - dot)], s);
    // Curve from b down-and-left to the midline (drawn as two segments)
    let mid = Pos2::new(b.x, (a.y + c.y) * 0.5);
    let merge = Pos2::new(a.x + dot * 1.5, mid.y);
    p.line_segment([Pos2::new(b.x, b.y + dot), mid], s);
    p.line_segment([mid, merge], s);
}

pub fn gear(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.15);
    let s = stroke_for(r.width(), color);
    let center = r.center();
    let outer = r.width() * 0.42;
    let inner = outer * 0.62;
    // 8 teeth as short radial segments
    for i in 0..8 {
        let a = (i as f32) * std::f32::consts::TAU / 8.0;
        let (sin, cos) = a.sin_cos();
        let p1 = Pos2::new(center.x + cos * inner, center.y + sin * inner);
        let p2 = Pos2::new(center.x + cos * outer, center.y + sin * outer);
        p.line_segment([p1, p2], s);
    }
    // Body circle (between teeth & hub)
    p.circle_stroke(center, inner * 0.85, s);
    // Hub
    p.circle_stroke(center, inner * 0.32, s);
}

pub fn grid(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.22);
    let s = stroke_for(r.width(), color);
    let gap = r.width() * 0.12;
    let cell = (r.width() - gap) * 0.5;
    let positions = [
        Pos2::new(r.left(),                  r.top()),
        Pos2::new(r.left() + cell + gap,     r.top()),
        Pos2::new(r.left(),                  r.top() + cell + gap),
        Pos2::new(r.left() + cell + gap,     r.top() + cell + gap),
    ];
    for pos in positions {
        let cell_rect = Rect::from_min_size(pos, Vec2::splat(cell));
        p.rect_stroke(cell_rect, egui::CornerRadius::same(2), s, StrokeKind::Outside);
    }
}

pub fn sun(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.12);
    let s = stroke_for(r.width(), color);
    let center = r.center();
    let body = r.width() * 0.22;
    p.circle_stroke(center, body, s);
    // 8 rays
    let r_inner = body * 1.4;
    let r_outer = r.width() * 0.45;
    for i in 0..8 {
        let a = (i as f32) * std::f32::consts::TAU / 8.0;
        let (sin, cos) = a.sin_cos();
        let p1 = Pos2::new(center.x + cos * r_inner, center.y + sin * r_inner);
        let p2 = Pos2::new(center.x + cos * r_outer, center.y + sin * r_outer);
        p.line_segment([p1, p2], s);
    }
}

/// Six-point folder polygon: tab on the top-left, diagonal slant to the body's
/// top edge, then a plain rectangle for the body. One continuous outline.
fn folder_points(rect: Rect) -> [Pos2; 6] {
    let r = pad(rect, 0.1);
    let tab_w = r.width() * 0.4;
    let tab_h = r.height() * 0.22;
    let slant = tab_h; // 45° slope so the tab merges into the body naturally
    let body_top = r.top() + tab_h;
    [
        Pos2::new(r.left(), r.top()),
        Pos2::new(r.left() + tab_w, r.top()),
        Pos2::new(r.left() + tab_w + slant, body_top),
        Pos2::new(r.right(), body_top),
        Pos2::new(r.right(), r.bottom()),
        Pos2::new(r.left(), r.bottom()),
    ]
}

/// Filled folder used in welcome / recent-projects.
pub fn folder_filled(p: &Painter, rect: Rect, color: Color32) {
    let pts = folder_points(rect);
    p.add(egui::Shape::convex_polygon(
        pts.to_vec(),
        color,
        Stroke::NONE,
    ));
}

/// Hollow folder outline — same silhouette as `folder_filled` but stroked.
pub fn folder_outline(p: &Painter, rect: Rect, color: Color32) {
    let s = stroke_for(rect.width(), color);
    let pts = folder_points(rect);
    // Closed polyline — emits one continuous outline.
    let mut closed = pts.to_vec();
    closed.push(pts[0]);
    p.add(egui::Shape::line(closed, s));
}

pub fn plus(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.25);
    let s = stroke_for(r.width(), color);
    let c = r.center();
    let arm = r.width() * 0.5;
    p.line_segment([Pos2::new(c.x - arm, c.y), Pos2::new(c.x + arm, c.y)], s);
    p.line_segment([Pos2::new(c.x, c.y - arm), Pos2::new(c.x, c.y + arm)], s);
}

pub fn menu_bars(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.22);
    let s = stroke_for(r.width(), color);
    for i in 0..3 {
        let y = r.top() + (r.height() / 2.0) * i as f32;
        p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], s);
    }
}

pub fn refresh(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.18);
    let s = stroke_for(r.width(), color);
    let center = r.center();
    let radius = r.width() * 0.42;
    // Most of a circle (cw arc), with an arrowhead at the top.
    // Implement as 16 chord segments around 270° instead of a single arc.
    let segments = 18;
    let start_angle = -std::f32::consts::FRAC_PI_2 - 0.4; // top, slight left
    let sweep = std::f32::consts::PI * 1.6;
    let mut prev = Pos2::new(
        center.x + start_angle.cos() * radius,
        center.y + start_angle.sin() * radius,
    );
    for i in 1..=segments {
        let a = start_angle + sweep * (i as f32 / segments as f32);
        let cur = Pos2::new(center.x + a.cos() * radius, center.y + a.sin() * radius);
        p.line_segment([prev, cur], s);
        prev = cur;
    }
    // Arrowhead at the end of the arc (top-left region)
    let head_len = radius * 0.45;
    let end = prev;
    let tangent_angle = start_angle + sweep + std::f32::consts::FRAC_PI_2;
    let h1 = Pos2::new(
        end.x + (tangent_angle - 0.6).cos() * head_len,
        end.y + (tangent_angle - 0.6).sin() * head_len,
    );
    let h2 = Pos2::new(
        end.x + (tangent_angle + 0.6).cos() * head_len * 0.6,
        end.y + (tangent_angle + 0.6).sin() * head_len * 0.6,
    );
    p.line_segment([end, h1], s);
    p.line_segment([end, h2], s);
}

/// Lightning bolt — small filled zig-zag.
pub fn bolt(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.15);
    // 6-point bolt outline (top-right zig down-left, then bottom-left zag up-right)
    let cx = r.center().x;
    let pts = vec![
        Pos2::new(cx + r.width() * 0.15,  r.top()),                       // top notch
        Pos2::new(r.right(),              r.top() + r.height() * 0.06),
        Pos2::new(cx + r.width() * 0.05,  r.top() + r.height() * 0.46),
        Pos2::new(cx + r.width() * 0.30,  r.top() + r.height() * 0.50),
        Pos2::new(r.left(),               r.bottom()),
        Pos2::new(cx - r.width() * 0.05,  r.top() + r.height() * 0.58),
        Pos2::new(cx - r.width() * 0.30,  r.top() + r.height() * 0.54),
    ];
    // Concave shape — use plain polygon path with a stroke as fill substitute via
    // egui's mesh would be heavier. The concave-polygon helper handles non-convex.
    p.add(egui::Shape::Path(egui::epaint::PathShape {
        points: pts,
        closed: true,
        fill: color,
        stroke: Stroke::NONE.into(),
    }));
}

#[allow(dead_code)]
pub fn close_x(p: &Painter, rect: Rect, color: Color32) {
    let r = pad(rect, 0.28);
    let s = stroke_for(r.width(), color);
    p.line_segment([r.left_top(), r.right_bottom()], s);
    p.line_segment([r.right_top(), r.left_bottom()], s);
}
