//! Shared Source/Program tabs follow the playback target without stealing Timeline keyboard focus.
use egui_kittest::Harness;
use filmcraft_engine::Session;
use filmcraft_ui_egui::{
    FilmcraftApp,
    dock::{DockNode, Floating, PanelKind, SplitSize},
    menus,
};
use serde_json::json;

fn tabs(panels: Vec<PanelKind>, active: usize) -> DockNode {
    DockNode::Tabs { panels, active }
}
fn paired() -> FilmcraftApp {
    let mut session = Session::default();
    session.execute("file.openDemoProject", json!({})).unwrap();
    session.execute("source.open", json!({"item":5})).unwrap();
    let mut app = FilmcraftApp::new(session);
    app.ui.dock = DockNode::Split {
        vertical: true,
        size: SplitSize::Ratio(0.6),
        a: Box::new(tabs(vec![PanelKind::Program, PanelKind::Source], 1)),
        b: Box::new(tabs(vec![PanelKind::Timeline], 0)),
    };
    app.ui.focused = PanelKind::Timeline;
    app
}
fn step(h: &mut Harness<'_, FilmcraftApp>) {
    let ctx = h.ctx.clone();
    let mut raw = std::mem::take(h.input_mut());
    eframe::App::raw_input_hook(h.state_mut(), &ctx, &mut raw);
    *h.input_mut() = raw;
    h.step();
}
fn space(h: &mut Harness<'_, FilmcraftApp>) {
    for pressed in [true, false] {
        h.input_mut().events.push(egui::Event::Key { key: egui::Key::Space, physical_key: None, pressed, repeat: false, modifiers: Default::default() });
    }
    step(h);
}

#[test]
fn timeline_space_reveals_program_and_second_space_stops_the_same_player() {
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 800.0)).with_step_dt(1.0 / 60.0).build_eframe(move |_| paired());
    for _ in 0..4 {
        step(&mut h);
    }
    h.state_mut().ui.focused = PanelKind::Timeline;
    assert!(h.state().ui.dock.is_visible(PanelKind::Source));
    let before = h.state().session.project.clone();
    space(&mut h);
    assert!(h.state().playback.playing);
    assert!(h.state().ui.dock.is_visible(PanelKind::Program), "Timeline playback must reveal its image in the shared monitor group");
    assert!(!h.state().ui.dock.is_visible(PanelKind::Source));
    assert_eq!(h.state().ui.focused, PanelKind::Timeline);
    space(&mut h);
    assert!(!h.state().playback.playing);
    assert!(!h.state().source_playback.clock.playing);
    assert_eq!(h.state().session.project, before);
}

#[test]
fn floating_and_maximized_shared_monitors_follow_both_playback_targets() {
    let mut app = paired();
    let ctx = egui::Context::default();
    app.ui.dock = tabs(vec![PanelKind::Timeline], 0);
    app.ui.floating = vec![Floating { panels: vec![PanelKind::Program, PanelKind::Source], active: 1, rect: [30.0, 40.0, 800.0, 450.0] }];
    app.ui.keys.maximized = Some(PanelKind::Source);
    menus::invoke(&mut app, &ctx, "playback.forward", json!({"monitor":"program"})).unwrap();
    assert_eq!(app.ui.floating[0].active, 0);
    assert_eq!(app.ui.keys.maximized, Some(PanelKind::Program));
    assert_eq!(app.ui.focused, PanelKind::Timeline);
    let rect = app.ui.floating[0].rect;
    app.play_source().unwrap();
    assert_eq!(app.ui.floating[0].active, 1);
    assert_eq!(app.ui.keys.maximized, Some(PanelKind::Source));
    assert_eq!(app.ui.floating[0].rect, rect);
    assert!(app.source_playback.clock.playing);
    assert!(!app.playback.playing);
    assert_eq!(app.ui.focused, PanelKind::Timeline);
}

#[test]
fn playback_keeps_separate_monitor_groups_and_closed_program_panels_unchanged() {
    let mut app = paired();
    app.ui.dock = DockNode::Split {
        vertical: false,
        size: SplitSize::Ratio(0.5),
        a: Box::new(tabs(vec![PanelKind::Source, PanelKind::EffectControls], 0)),
        b: Box::new(tabs(vec![PanelKind::Program, PanelKind::Metadata], 1)),
    };
    let before = app.ui.dock.clone();
    app.play(1.0);
    assert_eq!(app.ui.dock, before);
    app.stop();
    app.ui.dock = tabs(vec![PanelKind::Timeline, PanelKind::Source], 0);
    let before = app.ui.dock.clone();
    app.play(1.0);
    assert_eq!(app.ui.dock, before);
}
