//! Mouse edits of the same In/Out interval set by I/O. Out's drawn edge is exclusive.
use egui::{Pos2, pos2};
use egui_kittest::Harness;
use filmcraft_engine::Session;
use filmcraft_time::Tick;
use filmcraft_ui_egui::{FilmcraftApp, control::ControlRequest};
use serde_json::{Value, json};
use std::sync::mpsc::{Sender, channel};

struct Driver {
    harness: Harness<'static, FilmcraftApp>,
    tx: Sender<ControlRequest>,
}
impl Driver {
    fn new() -> Self {
        let mut s = Session::default();
        s.execute("file.openDemoProject", json!({})).unwrap();
        let (tx, rx) = channel();
        let app = FilmcraftApp::new(s).with_control(rx);
        let harness = Harness::builder().with_size(egui::vec2(1600.0, 980.0)).with_step_dt(1.0 / 60.0).with_max_steps(10000).build_eframe(move |_| app);
        let mut d = Self { harness, tx };
        d.frames(4);
        d
    }
    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            let ctx = self.harness.ctx.clone();
            let mut input = std::mem::take(self.harness.input_mut());
            eframe::App::raw_input_hook(self.harness.state_mut(), &ctx, &mut input);
            *self.harness.input_mut() = input;
            self.harness.step();
        }
    }
    fn call(&mut self, method: &str, params: Value) -> Value {
        let (req, reply) = ControlRequest::new(method, params);
        self.tx.send(req).unwrap();
        for _ in 0..600 {
            self.frames(1);
            if let Ok(v) = reply.try_recv() {
                assert_eq!(v["ok"], true, "{method}: {v}");
                return v["result"].clone();
            }
        }
        panic!("no reply");
    }
    fn exec(&mut self, id: &str, p: Value) -> Value {
        self.call("engine.execute", json!({"command":id,"params":p}))
    }
    fn grip(&mut self, id: &str) -> Pos2 {
        self.frames(2);
        let elements = self.call("ui.elements", json!({"prefix":id}));
        let r = elements.as_array().unwrap().iter().find(|e| e["id"] == id).unwrap()["rect"].as_array().unwrap();
        pos2((r[0].as_f64().unwrap() + r[2].as_f64().unwrap() / 2.0) as f32, (r[1].as_f64().unwrap() + r[3].as_f64().unwrap() / 2.0) as f32)
    }
    fn press(&mut self, p: Pos2, button: egui::PointerButton) {
        self.harness.input_mut().events.push(egui::Event::PointerMoved(p));
        self.frames(1);
        self.harness.input_mut().events.push(egui::Event::PointerButton { pos: p, button, pressed: true, modifiers: Default::default() });
        self.frames(1);
    }
    fn release(&mut self, p: Pos2, button: egui::PointerButton) {
        self.harness.input_mut().events.push(egui::Event::PointerButton { pos: p, button, pressed: false, modifiers: Default::default() });
        self.frames(2);
    }
    fn move_to(&mut self, p: Pos2) {
        self.harness.input_mut().events.push(egui::Event::PointerMoved(p));
        self.frames(2);
    }
    fn drag(&mut self, id: &str, edge: Tick, offset: f32) {
        let from = self.grip(id);
        let layout = self.harness.state().tl.layout.as_ref().unwrap();
        let source_edge = if id.ends_with("inPoint") { self.marks().0.unwrap() } else { self.marks().1.unwrap() + self.fd() };
        let to = pos2(layout.x_of(edge) + (from.x - layout.x_of(source_edge)) + offset, from.y);
        self.press(from, egui::PointerButton::Primary);
        self.move_to(to);
        self.release(to, egui::PointerButton::Primary);
    }
    fn marks(&self) -> (Option<Tick>, Option<Tick>) {
        let q = self.harness.state().session.active_sequence().unwrap();
        (q.mark_in, q.mark_out)
    }
    fn fd(&self) -> Tick {
        self.harness.state().session.active_sequence().unwrap().settings.frame_rate.frame_duration()
    }
}

#[test]
fn grips_snap_to_clip_cuts_without_moving_playhead_or_clips_and_undo_once() {
    let mut d = Driver::new();
    // Exercise the user's keyboard entry path, then move its handles with the mouse.
    d.exec("playhead.set", json!({"seconds":2}));
    d.call("ui.set", json!({"focused":"Timeline"}));
    d.call("ui.key", json!({"key":"I"}));
    d.exec("playhead.set", json!({"seconds":12}));
    d.call("ui.key", json!({"key":"O"}));
    let clips = d.harness.state().session.active_sequence().unwrap().video_tracks[0].items.clone();
    let cut = clips[1].start;
    let end = clips[2].end();
    let original = d.marks();
    let ph = d.harness.state().session.playhead();
    let undo = d.harness.state().session.history.undo.len();
    d.drag("timeline.inPoint", cut, 3.0);
    assert_eq!(d.marks(), (Some(cut), original.1));
    assert_eq!(d.harness.state().session.history.undo.len(), undo + 1);
    d.exec("edit.undo", json!({}));
    assert_eq!(d.marks(), original);
    d.exec("edit.redo", json!({}));
    assert_eq!(d.marks().0, Some(cut));
    d.drag("timeline.outPoint", end, -3.0);
    assert_eq!(d.marks().1, Some(end - d.fd()), "Out is inclusive: the grey band's edge must land exactly on the clip end");
    assert_eq!(d.harness.state().session.playhead(), ph);
    assert_eq!(d.harness.state().session.active_sequence().unwrap().video_tracks[0].items, clips);
}

#[test]
fn unsnapped_handles_remain_frame_aligned_and_cannot_cross() {
    let mut d = Driver::new();
    d.exec("sequence.snap", json!({"on":false}));
    d.exec("markers.markIn", json!({"seconds":2}));
    d.exec("markers.markOut", json!({"seconds":12}));
    let cut = d.harness.state().session.active_sequence().unwrap().video_tracks[0].items[1].start;
    d.drag("timeline.inPoint", cut, 5.0);
    assert_ne!(d.marks().0, Some(cut));
    let rate = d.harness.state().session.sequence_rate();
    assert_eq!(d.marks().0.unwrap(), rate.snap_nearest(d.marks().0.unwrap()));
    let out = d.marks().1.unwrap();
    d.drag("timeline.inPoint", out + Tick::from_seconds_f64(2.0), 0.0);
    assert_eq!(d.marks(), (Some(out), Some(out)));
    d.drag("timeline.outPoint", Tick::ZERO, 0.0);
    assert_eq!(d.marks(), (Some(out), Some(out)));
}

#[test]
fn escape_discards_preview_and_non_primary_drags_do_not_edit_marks() {
    let mut d = Driver::new();
    d.exec("markers.markIn", json!({"seconds":2}));
    d.exec("markers.markOut", json!({"seconds":12}));
    let before = d.harness.state().session.project.clone();
    let undo = d.harness.state().session.history.undo.len();
    let from = d.grip("timeline.inPoint");
    let to = from + egui::vec2(70.0, 0.0);
    d.press(from, egui::PointerButton::Primary);
    d.move_to(to);
    assert_eq!(d.harness.state().session.project, before, "preview must not mutate the project");
    d.call("ui.key", json!({"key":"Escape"}));
    d.release(to, egui::PointerButton::Primary);
    assert_eq!(d.harness.state().session.project, before);
    assert_eq!(d.harness.state().session.history.undo.len(), undo);
    for button in [egui::PointerButton::Secondary, egui::PointerButton::Middle] {
        let from = d.grip("timeline.inPoint");
        d.press(from, button);
        d.move_to(from + egui::vec2(20.0, 0.0));
        d.release(from + egui::vec2(20.0, 0.0), button);
        assert_eq!(d.harness.state().session.project, before);
    }
}

#[test]
fn existing_ruler_scrub_can_cross_a_mark_grip_without_being_captured() {
    let mut d = Driver::new();
    d.exec("sequence.snap", json!({"on":false}));
    d.exec("markers.markIn", json!({"seconds":2}));
    d.exec("markers.markOut", json!({"seconds":12}));
    let marks = d.marks();
    let to = d.grip("timeline.inPoint");
    let layout = d.harness.state().tl.layout.as_ref().unwrap();
    let from = pos2(layout.x_of(Tick::from_seconds_f64(6.0)), to.y);
    let time = d.harness.state().session.sequence_rate().snap_nearest(layout.tick_at(to.x));
    d.press(from, egui::PointerButton::Primary);
    d.move_to(to);
    d.release(to, egui::PointerButton::Primary);
    assert_eq!(d.marks(), marks);
    assert_eq!(d.harness.state().session.playhead(), time);
}
