//! Native save-linked EffectCraft jobs. Only complete renders replace our own sidecar movie.
use crate::FilmcraftApp;
use filmcraft_engine::effectcraft::{LINK_KEY, Link, Plan};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

type Result<T> = std::result::Result<T, String>;
type Stamp = (u64, u128);
#[derive(Serialize, Deserialize)]
struct Manifest {
    link: Link,
    rendered: Option<Stamp>,
}
struct Report {
    link: Link,
    stamp: Option<Stamp>,
    result: Result<bool>,
}
enum Done {
    Prepared(Box<Plan>, Link),
    Scanned(Vec<Report>),
}
#[derive(Default)]
pub struct State {
    worker: Option<Receiver<Result<Done>>>,
    seen: BTreeMap<String, Option<Stamp>>,
    last_scan: f64,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn stamp(path: &Path) -> Result<Stamp> {
    let m = fs::metadata(path).map_err(err)?;
    Ok((m.len(), m.modified().map_err(err)?.duration_since(UNIX_EPOCH).map_err(err)?.as_nanos()))
}
fn unique() -> Result<String> {
    Ok(format!("{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).map_err(err)?.as_nanos()))
}
fn cli() -> Result<PathBuf> {
    if let Some(p) = std::env::var_os("EFFECTCRAFT_CLI_PATH") {
        return Ok(p.into());
    }
    let exe = std::env::current_exe().map_err(err)?;
    if let Some(contents) = exe.parent().and_then(Path::parent) {
        let p = contents.join("Helpers/effectcraft-cli");
        if p.is_file() {
            return Ok(p);
        }
    }
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.dev-tools/effectcraft/effectcraft-cli");
    if p.is_file() {
        return Ok(p);
    }
    Err("EffectCraft render helper is missing. Rebuild FilmCraft Dev with its update helper.".into())
}
/// A linked project alone cannot authorize overwriting a file: require our matching sidecar.
fn manifest(link: &Link) -> Result<Manifest> {
    link.validate()?;
    let path = Path::new(&link.project).with_file_name("link.json");
    let f = fs::File::open(&path).map_err(err)?;
    if f.metadata().map_err(err)?.len() > 65536 {
        return Err("EffectCraft link manifest is too large".into());
    }
    let m: Manifest = serde_json::from_reader(f.take(65537)).map_err(err)?;
    if m.link != *link {
        return Err("EffectCraft link does not match its sidecar manifest".into());
    }
    for p in [&link.project, &link.output] {
        if fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("EffectCraft sidecars must not be symbolic links".into());
        }
    }
    Ok(m)
}
fn write_manifest(m: &Manifest) -> Result<()> {
    use filmcraft_engine::Services;
    let path = Path::new(&m.link.project).with_file_name("link.json");
    let data = serde_json::to_vec_pretty(m).map_err(err)?;
    filmcraft_engine::FsServices.write_file(&path.to_string_lossy(), &data).map_err(err)
}
fn process(args: &[String], log: &Path) -> Result<()> {
    let out = fs::File::create(log).map_err(err)?;
    let mut child = Command::new(cli()?)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out.try_clone().map_err(err)?))
        .stderr(Stdio::from(out))
        .spawn()
        .map_err(err)?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                let mut f = fs::File::open(log).map_err(err)?;
                let length = f.metadata().map_err(err)?.len();
                f.seek(SeekFrom::Start(length.saturating_sub(8192))).map_err(err)?;
                let mut text = Vec::new();
                f.take(8192).read_to_end(&mut text).map_err(err)?;
                return Err(format!("EffectCraft render helper failed ({status}): {}\nLog: {}", String::from_utf8_lossy(&text), log.display()));
            }
            Ok(None) if start.elapsed() < Duration::from_secs(3600) => std::thread::sleep(Duration::from_millis(100)),
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(match result {
                    Err(e) => e.to_string(),
                    _ => "EffectCraft job timed out after one hour; the last good render is unchanged".into(),
                });
            }
        }
    }
}
/// Render in a fresh directory, verify it, then atomically publish. A save during rendering is
/// deliberately not published; the next scan picks up the latest saved project instead.
fn render(link: &Link, before: Stamp) -> Result<bool> {
    let mut m = manifest(link)?;
    let dir = Path::new(&link.project).parent().ok_or("missing sidecar folder")?;
    let stage = dir.join(format!("render-{}", unique()?));
    fs::create_dir(&stage).map_err(err)?;
    let output = stage.join("render.mov");
    let result = (|| {
        let args: Vec<String> = [
            "render",
            "--project",
            &link.project,
            "--comp",
            "FilmCraft Clip",
            "--out",
            &output.to_string_lossy(),
            "--format",
            "prores",
            "--prores",
            "4444",
            "--channels",
            "rgba",
            "--audio",
            "off",
            "--resolution",
            "full",
            "--quality",
            "best",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        process(&args, &dir.join("render.log"))?;
        let source = filmcraft_engine::MediaPool::default().open_file(&output.to_string_lossy(), &filmcraft_engine::FsServices).map_err(err)?;
        if !source.info().has_video() || source.info().duration <= filmcraft_time::Tick::ZERO {
            return Err("EffectCraft produced no video; keeping the last good render".into());
        }
        source.video_frame(filmcraft_media::FrameRequest::full(filmcraft_time::Tick::ZERO)).map_err(err)?;
        if stamp(Path::new(&link.project))? != before {
            return Ok(false);
        }
        // Recheck ownership immediately before publishing a completed file.
        manifest(link)?;
        fs::rename(&output, &link.output).map_err(err)?;
        m.rendered = Some(before);
        write_manifest(&m)?;
        Ok(true)
    })();
    let _ = fs::remove_dir_all(&stage);
    result
}
fn prepare(plan: Plan) -> Result<Done> {
    if plan.width == 0
        || plan.height == 0
        || plan.width > 8192
        || plan.height > 8192
        || plan.rate.num <= 0
        || plan.rate.den <= 0
        || plan.rate.num as f64 / plan.rate.den as f64 > 240.0
        || plan.clip.duration <= filmcraft_time::Tick::ZERO
        || plan.clip.duration.seconds() > 86400.0
        || plan.clip.source_in < filmcraft_time::Tick::ZERO
    {
        return Err("unsupported clip dimensions, frame rate or duration for EffectCraft".into());
    }
    let parent = Path::new(&plan.filmcraft_project).parent().ok_or("save the FilmCraft project first")?;
    let root = parent.join("EffectCraft");
    fs::create_dir_all(&root).map_err(err)?;
    let token = unique()?;
    let dir = root.join(format!("clip-{}-{token}", plan.clip.id.0));
    fs::create_dir(&dir).map_err(err)?;
    let dir = fs::canonicalize(dir).map_err(err)?;
    let link = Link { project: dir.join("clip.ecproj").to_string_lossy().into_owned(), output: dir.join("render.mov").to_string_lossy().into_owned(), token };
    let source = fs::canonicalize(&plan.source).map_err(err)?.to_string_lossy().into_owned();
    let args = vec!["run".into(), "--empty".into(), "comp.new".into(), json!({"name":"FilmCraft Clip","width":plan.width,"height":plan.height,"fps":plan.rate.num as f64 / plan.rate.den as f64,"duration":plan.clip.duration.seconds()}).to_string(), "file.import".into(), json!({"paths":[source],"sequence":false,"addToComp":true}).to_string(), "layer.timing".into(), json!({"start":-plan.clip.source_in.seconds(),"in":0,"out":plan.clip.duration.seconds()}).to_string(), "--save-as".into(), link.project.clone(), "--json".into()];
    process(&args, &dir.join("prepare.log"))?;
    write_manifest(&Manifest { link: link.clone(), rendered: None })?;
    if !render(&link, stamp(Path::new(&link.project))?)? {
        return Err("EffectCraft project changed during preparation; retry the command".into());
    }
    Ok(Done::Prepared(Box::new(plan), link))
}
impl State {
    fn spawn(&mut self, ctx: &egui::Context, job: impl FnOnce() -> Result<Done> + Send + 'static) -> Result<()> {
        if self.worker.is_some() {
            return Err("An EffectCraft job is already running; please wait".into());
        }
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("effectcraft".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
                    .unwrap_or_else(|_| Err("EffectCraft worker failed; the project remains available".into()));
                let _ = tx.send(result);
                ctx.request_repaint();
            })
            .map_err(err)?;
        self.worker = Some(rx);
        Ok(())
    }
}
pub fn invoke(app: &mut FilmcraftApp, ctx: &egui::Context, params: Value) -> Result<Value> {
    let result = app.session.execute("clip.editInEffectCraft", params).map_err(err)?;
    if let Some(v) = result.get("existing") {
        let link: Link = serde_json::from_value(v.clone()).map_err(err)?;
        manifest(&link)?;
        app.effectcraft.seen.remove(&link.project);
        crate::panels::menu_dialogs::open_path(app, ctx, &link.project, false)?;
        return Ok(result);
    }
    let plan = serde_json::from_value(result.get("plan").cloned().ok_or("missing EffectCraft preparation plan")?).map_err(err)?;
    app.effectcraft.spawn(ctx, move || prepare(plan))?;
    app.ui.status = tl!("Preparing EffectCraft clip…").into();
    ctx.request_repaint();
    Ok(json!({"preparing":true}))
}
/// Called each frame, including when the window is unfocused. File I/O and rendering stay off
/// the UI thread; opening another FilmCraft project never attaches an old preparation job.
pub fn tick(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let mut state = std::mem::take(&mut app.effectcraft);
    if let Some(rx) = &state.worker {
        match rx.try_recv() {
            Ok(result) => {
                state.worker = None;
                let result = result.and_then(|done| match done {
                    Done::Prepared(plan, link) => {
                        app.session.execute("effectcraft.attach", json!({"plan":plan,"link":link})).map_err(err)?;
                        app.session.execute("effectcraft.refresh", json!({"output":link.output})).map_err(err)?;
                        crate::panels::menu_dialogs::open_path(app, ctx, &link.project, false)?;
                        app.ui.status = tl!("EffectCraft clip ready").into();
                        Ok(())
                    }
                    Done::Scanned(reports) => {
                        for r in reports {
                            // An edit made while a previous render was running needs another look.
                            if !matches!(r.result, Ok(false)) {
                                state.seen.insert(r.link.project.clone(), r.stamp);
                            }
                            match r.result {
                                Ok(true) => {
                                    app.session.execute("effectcraft.refresh", json!({"output":r.link.output})).map_err(err)?;
                                    app.ui.status = tl!("EffectCraft clip updated").into();
                                }
                                Ok(false) => {}
                                Err(e) => {
                                    app.ui.status = e.clone();
                                    app.toast = Some((e, ctx.input(|i| i.time)));
                                }
                            }
                        }
                        Ok(())
                    }
                });
                if let Err(e) = result {
                    app.ui.status = e.clone();
                    app.toast = Some((e, ctx.input(|i| i.time)));
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                state.worker = None;
                app.ui.status = "EffectCraft worker disconnected; retry the command".into();
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    let links: BTreeMap<_, _> = app
        .session
        .project
        .items
        .values()
        .filter_map(|item| {
            let link: Link = serde_json::from_str(item.metadata.get(LINK_KEY)?).ok()?;
            Some((link.project.clone(), link))
        })
        .collect();
    state.seen.retain(|p, _| links.contains_key(p));
    let now = ctx.input(|i| i.time);
    if state.worker.is_none() && !links.is_empty() && now - state.last_scan >= 1.0 {
        state.last_scan = now;
        let seen = state.seen.clone();
        if let Err(e) = state.spawn(ctx, move || {
            let mut reports = Vec::new();
            for (path, link) in links {
                let current = stamp(Path::new(&path)).ok();
                if seen.get(&path) == Some(&current) {
                    continue;
                }
                let result = (|| {
                    let m = manifest(&link)?;
                    let current = current.ok_or("EffectCraft project is missing or unreadable")?;
                    if m.rendered == Some(current) && Path::new(&link.output).is_file() {
                        return Ok(true);
                    }
                    render(&link, current)
                })();
                reports.push(Report { link, stamp: current, result });
            }
            Ok(Done::Scanned(reports))
        }) {
            app.ui.status = e;
        }
    }
    if state.worker.is_some() || !app.session.project.items.values().all(|i| !i.metadata.contains_key(LINK_KEY)) {
        ctx.request_repaint_after(Duration::from_millis(if state.worker.is_some() { 100 } else { 1000 }));
    }
    app.effectcraft = state;
}

#[cfg(test)]
mod tests {
    use super::*;
    use filmcraft_engine::Session;
    use filmcraft_media::{DemoScene, Generator, MediaSource, generators::GeneratorSource};
    use filmcraft_project::{ItemKind, Label, MediaClip, MediaRef, Project, SequenceSettings, TrackKind};
    use filmcraft_time::{FrameRate, Tick, TimeRange};
    use std::sync::{Arc, Mutex};
    fn folder() -> PathBuf {
        let p = std::env::temp_dir().join(format!("filmcraft-effectcraft-{}", unique().unwrap()));
        fs::create_dir(&p).unwrap();
        p
    }
    fn fixture() -> (Session, Plan) {
        let dir = folder();
        let path = dir.join("source.mov");
        let rate = FrameRate::FPS_24;
        let duration = rate.tick_of(12);
        let source = Arc::new(GeneratorSource::new(Generator::Demo(DemoScene::Aurora), 160, 90, rate, duration));
        let mut p = Project::new("EffectCraft roundtrip");
        let item = p.add_item(
            "Source",
            Label::Iris,
            ItemKind::Media(MediaClip {
                media: MediaRef::Generator(Generator::Demo(DemoScene::Aurora)),
                info: source.info().clone(),
                interpret: Default::default(),
                mark_in: None,
                mark_out: None,
                markers: vec![],
                offline: false,
                proxy: None,
                identity: None,
            }),
            None,
        );
        let seq = p.new_sequence("Seq", SequenceSettings { width: 160, height: 90, frame_rate: rate, ..Default::default() }, 1, 1, None);
        let video = p.make_track_item(item, TrackKind::Video, Tick::ZERO, TimeRange::new(Tick::ZERO, duration), rate).unwrap();
        let audio = p.make_track_item(item, TrackKind::Audio, Tick::ZERO, TimeRange::new(Tick::ZERO, duration), rate).unwrap();
        p.sequence_mut(seq).unwrap().video_tracks[0].items.push(video);
        p.sequence_mut(seq).unwrap().audio_tracks[0].items.push(audio);
        let shared: filmcraft_media::SharedSource = source;
        let provider = move |id| (id == item).then(|| shared.clone());
        filmcraft_export::export(
            &Arc::new(p),
            seq,
            &filmcraft_export::ExportSettings { format: filmcraft_export::Format::ProRes, path: path.to_string_lossy().into_owned(), ..Default::default() },
            &provider,
            &Default::default(),
        )
        .unwrap();
        let mut s = Session::default();
        let r = s.execute("file.import", json!({"paths":[path]})).unwrap();
        let item = filmcraft_project::ItemId(r["items"][0].as_u64().unwrap());
        let p = Arc::make_mut(&mut s.project);
        let seq = p.new_sequence("Edit", SequenceSettings { width: 160, height: 90, frame_rate: rate, ..Default::default() }, 1, 1, None);
        let video = p.make_track_item(item, TrackKind::Video, Tick::ZERO, TimeRange::new(rate.tick_of(4), rate.tick_of(8)), rate).unwrap();
        let audio = p.make_track_item(item, TrackKind::Audio, Tick::ZERO, TimeRange::new(rate.tick_of(4), rate.tick_of(8)), rate).unwrap();
        let clip = video.id;
        p.sequence_mut(seq).unwrap().video_tracks[0].items.push(video);
        p.sequence_mut(seq).unwrap().audio_tracks[0].items.push(audio);
        s.state.active_sequence = Some(seq);
        s.state.selection = vec![clip];
        s.path = Some(dir.join("edit.fcproj").to_string_lossy().into_owned());
        let plan = serde_json::from_value(s.execute("clip.editInEffectCraft", json!({})).unwrap()["plan"].clone()).unwrap();
        (s, plan)
    }
    #[test]
    fn malformed_or_unowned_links_cannot_publish_files() {
        let dir = folder();
        let link = Link {
            project: dir.join("clip.ecproj").to_string_lossy().into_owned(),
            output: dir.join("render.mov").to_string_lossy().into_owned(),
            token: "one".into(),
        };
        fs::write(&link.output, b"last good output").unwrap();
        assert!(manifest(&link).is_err());
        write_manifest(&Manifest { link: link.clone(), rendered: None }).unwrap();
        let mut wrong = link.clone();
        wrong.token = "two".into();
        assert!(manifest(&wrong).is_err());
        assert_eq!(fs::read(&link.output).unwrap(), b"last good output");
        fs::write(dir.join("link.json"), vec![0; 65537]).unwrap();
        assert!(manifest(&link).is_err());
    }
    #[test]
    fn worker_panics_are_returned_as_errors() {
        let mut state = State::default();
        state.spawn(&egui::Context::default(), || panic!("synthetic worker failure")).unwrap();
        let result = state.worker.take().unwrap().recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(result.is_err());
    }
    #[test]
    fn menu_errors_are_visible_without_starting_a_job() {
        let (mut s, _) = fixture();
        s.path = None;
        let mut app = FilmcraftApp::new(s);
        let error = crate::menus::invoke(&mut app, &egui::Context::default(), "clip.editInEffectCraft", json!({})).unwrap_err();
        assert!(error.contains("save"));
        assert_eq!(app.ui.status, error);
        assert!(app.effectcraft.worker.is_none());
    }
    fn pump(app: &mut FilmcraftApp, ctx: &egui::Context) {
        let start = Instant::now();
        loop {
            let _ = ctx.run_ui(egui::RawInput { time: Some(2.0 + start.elapsed().as_secs_f64()), ..Default::default() }, |ui| tick(app, ui.ctx()));
            if app.effectcraft.worker.is_none() {
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(30), "EffectCraft worker timed out in test");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    /// Requires the pinned, downloaded EffectCraft CLI. CPU only; no GUI application is opened.
    #[test]
    #[ignore = "requires EffectCraft CLI; run explicitly for native roundtrip validation"]
    fn real_cli_save_updates_same_timeline_clip_and_failure_keeps_last_good_render() {
        let (s, plan) = fixture();
        let audio = s.active_sequence().unwrap().audio_tracks.clone();
        let original = fs::read(&plan.source).unwrap();
        let mut app = FilmcraftApp::new(s);
        let opened = Arc::new(Mutex::new(Vec::new()));
        let out = opened.clone();
        app.hooks.open_path = Some(Box::new(move |path, _| {
            out.lock().unwrap().push(path.to_owned());
            Ok(())
        }));
        let ctx = egui::Context::default();
        invoke(&mut app, &ctx, json!({})).unwrap();
        pump(&mut app, &ctx);
        assert_eq!(opened.lock().unwrap().len(), 1, "{}", app.ui.status);
        let c = &app.session.active_sequence().unwrap().video_tracks[0].items[0];
        assert_eq!(c.id, plan.clip.id);
        assert_eq!(c.duration, plan.clip.duration);
        assert_eq!(c.source_in, Tick::ZERO);
        let item = c.item;
        let link: Link = serde_json::from_str(&app.session.project.item(item).unwrap().metadata[LINK_KEY]).unwrap();
        let first = fs::read(&link.output).unwrap();
        let first_picture = app.session.render_program(1.0).unwrap().over_black_rgba8();
        let saved: Value = serde_json::from_slice(&fs::read(&link.project).unwrap()).unwrap();
        let layer = saved["items"]
            .as_object()
            .unwrap()
            .values()
            .find_map(|it| it["kind"]["layers"].as_array().and_then(|a| a.first()).map(|l| l["id"].clone()))
            .unwrap();
        // Simulate exactly the saved project that the EffectCraft GUI writes, via its own engine.
        process(
            &[
                "run".into(),
                "--project".into(),
                link.project.clone(),
                "layer.setTransform".into(),
                json!({"layers":[layer],"prop":"opacity","value":0}).to_string(),
                "--save-as".into(),
                link.project.clone(),
                "--json".into(),
            ],
            &Path::new(&link.project).with_file_name("test-save.log"),
        )
        .unwrap();
        app.effectcraft.last_scan = f64::NEG_INFINITY;
        pump(&mut app, &ctx);
        let second = fs::read(&link.output).unwrap();
        assert!(first != second, "saving the effect must update the actual movie");
        let second_picture = app.session.render_program(1.0).unwrap().over_black_rgba8();
        assert!(first_picture != second_picture, "the Timeline must invalidate its cached source");
        assert_eq!(app.session.active_sequence().unwrap().video_tracks[0].items[0].item, item);
        assert_eq!(app.session.active_sequence().unwrap().audio_tracks, audio);
        let src = app.session.media.open_file(&link.output, &*app.session.services).unwrap();
        assert!(src.info().has_video());
        assert!(src.video_frame(filmcraft_media::FrameRequest::full(Tick::ZERO)).is_ok());
        fs::write(&link.project, b"broken saved project").unwrap();
        app.effectcraft.last_scan = f64::NEG_INFINITY;
        pump(&mut app, &ctx);
        assert_eq!(fs::read(&link.output).unwrap(), second, "failed renders keep the last good movie");
        assert!(app.ui.status.contains("failed"), "{}", app.ui.status);
        assert_eq!(fs::read(&plan.source).unwrap(), original, "original footage is untouched");
        assert_eq!(opened.lock().unwrap().len(), 1, "background updates never reopen the GUI");
        assert!(app.session.undo().is_some());
        assert_eq!(serde_json::to_value(&app.session.active_sequence().unwrap().video_tracks[0].items[0]).unwrap(), serde_json::to_value(&plan.clip).unwrap());
    }
}
