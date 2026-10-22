//! Native save-linked EffectCraft jobs. Only complete renders replace our own sidecar movie.
use crate::FilmcraftApp;
use filmcraft_engine::effectcraft::{Link, Plan};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
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
    #[serde(default)]
    local_media: bool,
}
enum Done {
    Prepared(Box<Plan>, Link),
    Migrated(Link, Link),
    Applied(Option<Box<Plan>>, Link),
}
#[derive(Default)]
pub struct State {
    worker: Option<Receiver<Result<Done>>>,
    error: Option<String>,
    preparing: bool,
    applying: bool,
}

/// Keep failures visible even with a hidden status bar. A status-only error used to look like
/// a dead menu action; `toast` is not currently rendered by this frontend.
pub fn report_error(app: &mut FilmcraftApp, error: String) {
    log::error!("EffectCraft: {error}");
    app.ui.status = error.clone();
    app.effectcraft.error = Some(error);
}

pub fn show(app: &mut FilmcraftApp, ctx: &egui::Context) {
    if app.effectcraft.error.is_none() && !app.effectcraft.preparing {
        return;
    }
    let mut close = false;
    let mut save = false;
    egui::Window::new("EffectCraft").id(egui::Id::new("effectcraft.job")).collapsible(false).default_width(480.0).show(ctx, |ui| {
        if let Some(error) = &app.effectcraft.error {
            ui.label(tl!("Error"));
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                let r = ui.label(error);
                app.auto.add_response("effectcraft.job.error", &r, error);
            });
            if app.session.path.is_none() {
                let r = ui.button(tl!("Save Project"));
                app.auto.add_response("effectcraft.job.save", &r, tl!("Save Project"));
                save = r.clicked();
            }
            let r = ui.button(tl!("Close"));
            app.auto.add_response("effectcraft.job.close", &r, tl!("Close"));
            close = r.clicked();
        } else {
            ui.horizontal(|ui| {
                ui.spinner();
                let text = if app.effectcraft.applying { tl!("Apply EffectCraft Changes") } else { tl!("Preparing EffectCraft clip…") };
                let r = ui.label(text);
                app.auto.add_response("effectcraft.job.progress", &r, text);
            });
        }
    });
    if close || save {
        app.effectcraft.error = None;
    }
    if save {
        let result = crate::menus::invoke(app, ctx, "file.saveAs", json!({}));
        if let Err(error) = result {
            report_error(app, error);
        }
    }
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn stamp(path: &Path) -> Result<Stamp> {
    let m = fs::metadata(path).map_err(err)?;
    Ok((m.len(), m.modified().map_err(err)?.duration_since(UNIX_EPOCH).map_err(err)?.as_nanos()))
}
fn unique() -> Result<String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    Ok(format!(
        "{}-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).map_err(err)?.as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ))
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
fn prepare(plan: Plan, root: PathBuf) -> Result<Done> {
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
    fs::create_dir_all(&root).map_err(err)?;
    let token = unique()?;
    let dir = root.join(format!("clip-{}-{token}", plan.clip.id.0));
    fs::create_dir(&dir).map_err(err)?;
    let dir = fs::canonicalize(dir).map_err(err)?;
    let link = Link { project: dir.join("clip.ecproj").to_string_lossy().into_owned(), output: dir.join("render.mov").to_string_lossy().into_owned(), token };
    let original = Path::new(&plan.source);
    let media_dir = dir.join("Media");
    fs::create_dir(&media_dir).map_err(err)?;
    let local = media_dir.join(original.file_name().ok_or("missing source filename")?);
    let mut input = fs::File::open(original).map_err(|e| format!("cannot read source video: {e}"))?;
    let mut output = fs::OpenOptions::new().write(true).create_new(true).open(&local).map_err(err)?;
    std::io::copy(&mut input, &mut output).map_err(|e| format!("cannot create local EffectCraft working copy: {e}"))?;
    output.sync_all().map_err(err)?;
    let source = local.to_string_lossy().into_owned();
    let args = vec!["run".into(), "--empty".into(), "comp.new".into(), json!({"name":"FilmCraft Clip","width":plan.width,"height":plan.height,"fps":plan.rate.num as f64 / plan.rate.den as f64,"duration":plan.clip.duration.seconds()}).to_string(), "file.import".into(), json!({"paths":[source],"sequence":false,"addToComp":true}).to_string(), "layer.timing".into(), json!({"start":-plan.clip.source_in.seconds(),"in":0,"out":plan.clip.duration.seconds()}).to_string(), "--save-as".into(), link.project.clone(), "--json".into()];
    process(&args, &dir.join("prepare.log"))?;
    write_manifest(&Manifest { link: link.clone(), rendered: None, local_media: true })?;
    Ok(Done::Prepared(Box::new(plan), link))
}
/// Recover links made by the first integration without overwriting the user's old .ecproj.
fn migrate(old: Link, root: PathBuf) -> Result<Done> {
    manifest(&old)?;
    let f = fs::File::open(&old.project).map_err(err)?;
    if f.metadata().map_err(err)?.len() > 64 * 1024 * 1024 {
        return Err("EffectCraft project exceeds the migration size limit".into());
    }
    let mut project: Value = serde_json::from_reader(f.take(64 * 1024 * 1024 + 1)).map_err(err)?;
    let token = unique()?;
    let dir = root.join(format!("recovered-{token}"));
    fs::create_dir_all(&dir).map_err(err)?;
    let dir = fs::canonicalize(dir).map_err(err)?;
    let items = project.get_mut("items").and_then(Value::as_object_mut).ok_or("missing EffectCraft project items")?;
    if items.len() > 2048 {
        return Err("too many items in EffectCraft project to migrate automatically".into());
    }
    let mut copied = 0;
    for item in items.values_mut() {
        let Some(kind) = item.get_mut("kind").and_then(Value::as_object_mut) else {
            continue;
        };
        if kind.get("type").and_then(Value::as_str) != Some("Footage") {
            continue;
        }
        let Some(path) = kind.get("path").and_then(Value::as_str).filter(|s| !s.is_empty()) else {
            continue;
        };
        if copied >= 128 {
            return Err("too many footage files to migrate automatically".into());
        }
        let path = Path::new(path);
        let source = if path.is_absolute() { path.to_path_buf() } else { Path::new(&old.project).parent().ok_or("missing project folder")?.join(path) };
        let folder = dir.join(format!("Media-{copied}"));
        fs::create_dir(&folder).map_err(err)?;
        let dest = folder.join(source.file_name().ok_or("missing footage filename")?);
        fs::copy(source, &dest).map_err(|e| format!("cannot copy EffectCraft footage into the local workspace: {e}"))?;
        kind.insert("path".into(), json!(dest.to_string_lossy()));
        kind.insert("missing".into(), json!(false));
        copied += 1;
    }
    if copied == 0 {
        return Err("the old EffectCraft project contains no readable file footage to recover".into());
    }
    let link = Link { project: dir.join("clip.ecproj").to_string_lossy().into_owned(), output: dir.join("render.mov").to_string_lossy().into_owned(), token };
    fs::write(&link.project, serde_json::to_vec_pretty(&project).map_err(err)?).map_err(err)?;
    write_manifest(&Manifest { link: link.clone(), rendered: None, local_media: true })?;
    Ok(Done::Migrated(old, link))
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
fn workspace_root(app: &FilmcraftApp) -> PathBuf {
    std::env::var_os("FILMCRAFT_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| app.session.prefs_path.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf))
        .unwrap_or_else(|| filmcraft_engine::temp_dir().join("FilmCraft Dev"))
        .join("EffectCraft")
}
pub fn invoke(app: &mut FilmcraftApp, ctx: &egui::Context, params: Value) -> Result<Value> {
    let result = app.session.execute("clip.editInEffectCraft", params).map_err(err)?;
    if let Some(v) = result.get("existing").or_else(|| result.get("pending").and_then(|p| p.get("link"))) {
        let link: Link = serde_json::from_value(v.clone()).map_err(err)?;
        if !manifest(&link)?.local_media {
            let root = workspace_root(app);
            app.effectcraft.spawn(ctx, move || migrate(link, root))?;
            app.effectcraft.preparing = true;
            app.effectcraft.applying = false;
            app.effectcraft.error = None;
            ctx.request_repaint();
            return Ok(json!({"preparing":true}));
        }
        crate::panels::menu_dialogs::open_path(app, ctx, &link.project, false)?;
        app.effectcraft.error = None;
        return Ok(result);
    }
    let plan = serde_json::from_value(result.get("plan").cloned().ok_or("missing EffectCraft preparation plan")?).map_err(err)?;
    let root = workspace_root(app);
    app.effectcraft.spawn(ctx, move || prepare(plan, root))?;
    app.effectcraft.error = None;
    app.effectcraft.preparing = true;
    app.effectcraft.applying = false;
    app.ui.status = tl!("Preparing EffectCraft clip…").into();
    ctx.request_repaint();
    Ok(json!({"preparing":true}))
}
pub fn apply(app: &mut FilmcraftApp, ctx: &egui::Context, params: Value) -> Result<Value> {
    let result = app.session.execute("clip.applyEffectCraft", params).map_err(err)?;
    let (plan, link) = if let Some(pending) = result.get("pending") {
        let p: filmcraft_engine::effectcraft::Pending = serde_json::from_value(pending.clone()).map_err(err)?;
        (Some(Box::new(p.plan)), p.link)
    } else if let Some(existing) = result.get("existing") {
        (None, serde_json::from_value(existing.clone()).map_err(err)?)
    } else {
        return Err("Open the clip in EffectCraft first, save your edits there, then apply them here".into());
    };
    if !manifest(&link)?.local_media {
        return Err("Open the clip in EffectCraft first to create an accessible local working copy".into());
    }
    app.effectcraft.spawn(ctx, move || {
        if !render(&link, stamp(Path::new(&link.project))?)? {
            return Err("EffectCraft was saved again while rendering; apply the latest saved version again".into());
        }
        Ok(Done::Applied(plan, link))
    })?;
    app.effectcraft.preparing = true;
    app.effectcraft.applying = true;
    app.effectcraft.error = None;
    ctx.request_repaint();
    Ok(json!({"rendering":true}))
}
/// Poll only explicitly requested work. Saving in EffectCraft never starts a render by itself.
pub fn tick(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let mut state = std::mem::take(&mut app.effectcraft);
    if let Some(rx) = &state.worker {
        match rx.try_recv() {
            Ok(result) => {
                state.worker = None;
                state.preparing = false;
                let result = result.and_then(|done| match done {
                    Done::Prepared(plan, link) => {
                        app.session.execute("effectcraft.stage", json!({"plan":plan,"link":link})).map_err(err)?;
                        crate::panels::menu_dialogs::open_path(app, ctx, &link.project, false)?;
                        app.ui.status = tl!("EffectCraft clip ready").into();
                        Ok(())
                    }
                    Done::Migrated(old, link) => {
                        app.session.execute("effectcraft.rebind", json!({"old":old,"new":link})).map_err(err)?;
                        crate::panels::menu_dialogs::open_path(app, ctx, &link.project, false)?;
                        app.ui.status = tl!("EffectCraft clip ready").into();
                        Ok(())
                    }
                    Done::Applied(plan, link) => {
                        if let Some(plan) = plan {
                            app.session.execute("effectcraft.attach", json!({"plan":plan,"link":link})).map_err(err)?;
                        }
                        app.session.execute("effectcraft.refresh", json!({"output":link.output})).map_err(err)?;
                        app.ui.status = tl!("EffectCraft clip updated").into();
                        Ok(())
                    }
                });
                if let Err(e) = result {
                    log::error!("EffectCraft: {e}");
                    state.error = Some(e.clone());
                    app.ui.status = e;
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                state.worker = None;
                state.preparing = false;
                state.error = Some("EffectCraft worker disconnected; retry the command".into());
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    if state.worker.is_some() {
        ctx.request_repaint_after(Duration::from_millis(100));
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
        write_manifest(&Manifest { link: link.clone(), rendered: None, local_media: true }).unwrap();
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
        app.ui.show_status_bar = false;
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| show(&mut app, ui.ctx()));
        assert!(app.auto.elements.iter().any(|e| e.id == "effectcraft.job.error"));
        assert!(app.auto.elements.iter().any(|e| e.id == "effectcraft.job.save"));
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
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| show(&mut app, ui.ctx()));
        assert!(app.auto.elements.iter().any(|e| e.id == "effectcraft.job.progress"));
        pump(&mut app, &ctx);
        assert_eq!(opened.lock().unwrap().len(), 1, "{}", app.ui.status);
        let c = &app.session.active_sequence().unwrap().video_tracks[0].items[0];
        assert_eq!(c.id, plan.clip.id);
        assert_eq!(c.duration, plan.clip.duration);
        assert_eq!(c.source_in, plan.clip.source_in);
        assert_eq!(c.item, plan.clip.item, "opening must leave the original clip in the Timeline");
        let pending = app.session.execute("clip.editInEffectCraft", json!({})).unwrap();
        let link: Link = serde_json::from_value(pending["pending"]["link"].clone()).unwrap();
        assert!(!Path::new(&link.output).exists(), "opening must not render any movie");
        let backup = Path::new(&plan.source).with_file_name("hidden-original.mov");
        fs::rename(&plan.source, &backup).unwrap();
        apply(&mut app, &ctx, json!({})).unwrap();
        pump(&mut app, &ctx);
        fs::rename(backup, &plan.source).unwrap();
        let item = app.session.active_sequence().unwrap().video_tracks[0].items[0].item;
        assert_ne!(item, plan.clip.item, "{}", app.ui.status);
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
        pump(&mut app, &ctx);
        assert!(fs::read(&link.output).unwrap() == first, "saving alone must not render");
        apply(&mut app, &ctx, json!({})).unwrap();
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
        apply(&mut app, &ctx, json!({})).unwrap();
        pump(&mut app, &ctx);
        assert_eq!(fs::read(&link.output).unwrap(), second, "failed renders keep the last good movie");
        assert!(app.ui.status.contains("failed"), "{}", app.ui.status);
        assert_eq!(fs::read(&plan.source).unwrap(), original, "original footage is untouched");
        assert_eq!(opened.lock().unwrap().len(), 1, "background updates never reopen the GUI");
        assert!(app.session.undo().is_some());
        assert_eq!(serde_json::to_value(&app.session.active_sequence().unwrap().video_tracks[0].items[0]).unwrap(), serde_json::to_value(&plan.clip).unwrap());
    }
    #[test]
    #[ignore = "requires EffectCraft CLI; explicitly validates recovery of the initial integration"]
    fn old_links_get_local_media_without_overwriting_the_old_project_or_render() {
        let (_, plan) = fixture();
        let Done::Prepared(_, old) = prepare(plan.clone(), folder()).unwrap() else {
            panic!("expected prepared project");
        };
        let mut project: Value = serde_json::from_slice(&fs::read(&old.project).unwrap()).unwrap();
        for item in project["items"].as_object_mut().unwrap().values_mut() {
            if item["kind"]["type"] == "Footage" {
                item["kind"]["path"] = json!(plan.source);
                item["kind"]["missing"] = json!(true);
            }
        }
        let old_bytes = serde_json::to_vec_pretty(&project).unwrap();
        fs::write(&old.project, &old_bytes).unwrap();
        fs::write(&old.output, b"previous render").unwrap();
        write_manifest(&Manifest { link: old.clone(), rendered: None, local_media: false }).unwrap();
        let Done::Migrated(_, new) = migrate(old.clone(), folder()).unwrap() else {
            panic!("expected migrated project");
        };
        assert_eq!(fs::read(&old.project).unwrap(), old_bytes);
        assert_eq!(fs::read(&old.output).unwrap(), b"previous render");
        assert!(!Path::new(&new.output).exists(), "migration must not render");
        let original = fs::read(&plan.source).unwrap();
        fs::remove_file(&plan.source).unwrap();
        let recovered: Value = serde_json::from_slice(&fs::read(&new.project).unwrap()).unwrap();
        for item in recovered["items"].as_object().unwrap().values() {
            if item["kind"]["type"] == "Footage" {
                assert_eq!(fs::read(item["kind"]["path"].as_str().unwrap()).unwrap(), original);
                assert_eq!(item["kind"]["missing"], false);
            }
        }
        assert!(render(&new, stamp(Path::new(&new.project)).unwrap()).unwrap());
    }
}
