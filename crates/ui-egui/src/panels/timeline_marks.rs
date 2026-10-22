//! Mouse-editable sequence In/Out boundaries. Preview locally; commit one ordinary mark command.
use egui::{Color32, CursorIcon, Pos2, Rect, Stroke, pos2};
use filmcraft_project::{ItemId, Sequence};
use filmcraft_time::Tick;
use serde_json::json;

use super::timeline::Layout;
use crate::FilmcraftApp;

#[derive(Clone)]
pub(super) struct MarkDrag {
    sequence: ItemId,
    is_in: bool,
    original: (Option<Tick>, Option<Tick>),
    edge: Tick,
    initial_edge: Tick,
    pointer_x: f32,
    cancelled: bool,
}

pub(super) fn marks(app: &FilmcraftApp, seq: &Sequence) -> (Option<Tick>, Option<Tick>) {
    let mut marks = (seq.mark_in, seq.mark_out);
    if let Some(d) = &app.tl.range_drag
        && !d.cancelled
        && app.session.state.active_sequence == Some(d.sequence)
    {
        if d.is_in {
            marks.0 = Some(d.edge);
        } else {
            marks.1 = Some(d.edge - seq.settings.frame_rate.frame_duration());
        }
    }
    marks
}

fn handle(layout: &Layout, edge: Tick, is_in: bool) -> Rect {
    let x = layout.x_of(edge);
    // In grips face right, Out grips face left; adjacent grips still have distinct targets.
    let (a, b) = if is_in { (x - 3.0, x + 9.0) } else { (x - 9.0, x + 3.0) };
    Rect::from_min_max(pos2(a, layout.ruler.max.y - 16.0), pos2(b, layout.ruler.max.y - 2.0))
}

pub(super) fn interact(app: &mut FilmcraftApp, ui: &mut egui::Ui, seq: &Sequence, layout: &Layout, sequence: ItemId) -> bool {
    let ctx = ui.ctx();
    let fd = seq.settings.frame_rate.frame_duration();
    let pointer = ctx.input(|i| i.pointer.interact_pos());
    let hover = pointer.filter(|p| ui.rect_contains_pointer(layout.ruler) && layout.ruler.contains(*p)).and_then(|p| {
        [(true, seq.mark_in), (false, seq.mark_out.map(|t| t + fd))]
            .into_iter()
            .filter_map(|(is_in, edge)| edge.map(|t| (is_in, t)))
            .filter(|(is_in, t)| handle(layout, *t, *is_in).contains(p))
            .min_by(|a, b| (handle(layout, a.1, a.0).center().x - p.x).abs().total_cmp(&(handle(layout, b.1, b.0).center().x - p.x).abs()))
    });
    if app.tl.range_drag.is_none()
        && app.tl.drag.is_none()
        && let Some((is_in, edge)) = hover
    {
        ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            app.tl.pressed = None;
            app.tl.range_drag = Some(MarkDrag {
                sequence,
                is_in,
                original: (seq.mark_in, seq.mark_out),
                edge,
                initial_edge: edge,
                pointer_x: pointer.map_or(layout.x_of(edge), |p| p.x),
                cancelled: false,
            });
        }
    }
    let Some(mut d) = app.tl.range_drag.take() else {
        return app.tl.drag.is_none() && app.tl.pressed.is_none() && hover.is_some() && !ctx.input(|i| i.pointer.secondary_down() || i.pointer.middle_down());
    };
    if d.sequence != sequence {
        return true;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape) || !i.focused) || d.original != (seq.mark_in, seq.mark_out) {
        d.cancelled = true;
    }
    ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
    // A release commits after this frame took its project snapshot; redraw the committed marks.
    ctx.request_repaint();
    if !d.cancelled
        && let Some(p) = pointer
        && ((p.x - d.pointer_x).abs() > 0.5 || d.edge != d.initial_edge)
    {
        let rate = seq.settings.frame_rate;
        let desired_x = layout.x_of(d.initial_edge) + (p.x - d.pointer_x);
        let mut edge = rate.snap_nearest(layout.tick_at(desired_x).max(Tick::ZERO));
        if app.session.state.snapping {
            let other = if d.is_in { seq.mark_out.map(|t| t + fd) } else { seq.mark_in };
            let candidates = seq
                .all_tracks()
                .flat_map(|t| t.items.iter().flat_map(|i| [i.start, i.end()]))
                .chain(seq.markers.iter().map(|m| m.start))
                .chain(other)
                .chain(std::iter::once(app.session.playhead()));
            if let Some(t) = candidates
                .filter(|t| (layout.x_of(*t) - desired_x).abs() < 9.0)
                .min_by(|a, b| (layout.x_of(*a) - desired_x).abs().total_cmp(&(layout.x_of(*b) - desired_x).abs()))
            {
                edge = rate.snap_nearest(t);
            }
        }
        d.edge = if d.is_in { edge.min(seq.mark_out.unwrap_or(Tick::MAX)) } else { edge.max(seq.mark_in.unwrap_or(Tick::ZERO) + fd) };
    }
    if ctx.input(|i| i.pointer.primary_released()) {
        if !d.cancelled && d.edge != d.initial_edge {
            let command = if d.is_in { "markers.markIn" } else { "markers.markOut" };
            let time = if d.is_in { d.edge } else { d.edge - fd };
            if let Err(error) = app.session.execute(command, json!({"time":time.0,"target":"program"})) {
                app.ui.status = error.to_string();
            }
        }
    } else if ctx.input(|i| i.pointer.primary_down()) {
        app.tl.range_drag = Some(d);
    }
    true
}

pub(super) fn paint(app: &mut FilmcraftApp, ui: &egui::Ui, seq: &Sequence, layout: &Layout) {
    let (a, b) = marks(app, seq);
    let fd = seq.settings.frame_rate.frame_duration();
    let painter = ui.painter().with_clip_rect(layout.ruler);
    for (is_in, edge, id, label) in [(true, a, "timeline.inPoint", "Mark In"), (false, b.map(|t| t + fd), "timeline.outPoint", "Mark Out")] {
        let Some(edge) = edge else { continue };
        let x = layout.x_of(edge);
        let r = handle(layout, edge, is_in);
        if !r.intersects(layout.ruler) {
            continue;
        }
        let dir = if is_in { 1.0 } else { -1.0 };
        let points: Vec<Pos2> = vec![pos2(x, r.min.y), pos2(x + dir * 8.0, r.min.y), pos2(x + dir * 8.0, r.min.y + 5.0), pos2(x, r.max.y)];
        painter.add(egui::Shape::convex_polygon(points, Color32::from_gray(225), Stroke::new(1.0, Color32::from_gray(35))));
        app.auto.add(id, r.intersect(layout.ruler), crate::i18n::t(label));
    }
    if let Some(d) = &app.tl.range_drag
        && !d.cancelled
    {
        let x = layout.x_of(d.edge);
        ui.painter()
            .with_clip_rect(Rect::from_min_max(layout.ruler.min, layout.content.max))
            .line_segment([pos2(x, layout.ruler.min.y), pos2(x, layout.content.max.y)], Stroke::new(1.0, app.tokens.accent));
    }
}
