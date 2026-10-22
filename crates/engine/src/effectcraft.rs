//! Save-linked EffectCraft footage. Native preparation/rendering lives in the UI host; attaching
//! the result is one ordinary undoable project edit and never replaces the original media file.
use crate::{EngineError, Result, Session, commands::CommandSpec};
use filmcraft_project::{ItemId, MediaRef, TrackItem, TrackKind};
use filmcraft_time::FrameRate;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const LINK_KEY: &str = "filmcraft.effectcraft.v1";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub project: String,
    pub output: String,
    pub token: String,
}
impl Link {
    pub fn validate(&self) -> std::result::Result<(), String> {
        let project = std::path::Path::new(&self.project);
        let output = std::path::Path::new(&self.output);
        if !project.is_absolute()
            || project.file_name().and_then(|x| x.to_str()) != Some("clip.ecproj")
            || output != project.with_file_name("render.mov")
            || self.token.is_empty()
            || self.token.len() > 128
        {
            return Err("invalid EffectCraft link paths".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub filmcraft_project: String,
    pub sequence: ItemId,
    pub clip: TrackItem,
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub rate: FrameRate,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pending {
    pub plan: Plan,
    pub link: Link,
}
fn pending_key(seq: ItemId, clip: filmcraft_project::ClipId) -> String {
    format!("filmcraft.effectcraft.pending.{}.{}", seq.0, clip.0)
}
fn fail(message: impl Into<String>) -> EngineError {
    EngineError::Other(message.into())
}
fn prepare(s: &mut Session, p: &Value) -> Result<Value> {
    if let Some(ids) = p.get("clips")
        && !ids.as_array().is_some_and(|a| !a.is_empty() && a.len() <= 4096 && a.iter().all(|id| id.as_u64().is_some()))
    {
        return Err(fail("clips must be a nonempty array of clip ids"));
    }
    let q = s.active_sequence().ok_or(EngineError::NoSequence)?;
    let ids = crate::commands::clips_p(s, p);
    if ids.iter().any(|id| q.find_item(*id).is_none()) {
        return Err(fail("no such clip"));
    }
    let video: Vec<_> = ids.iter().filter_map(|id| q.find_item(*id)).filter(|(t, _)| q.track(*t).is_some_and(|t| t.kind == TrackKind::Video)).collect();
    let [(track, it)] = video.as_slice() else {
        return Err(fail("select one video clip for EffectCraft"));
    };
    if q.track(*track).is_some_and(|t| t.locked) {
        return Err(fail("the clip's track is locked"));
    }
    let item = s.project.item(it.item).ok_or_else(|| fail("missing source item"))?;
    let sequence = s.state.active_sequence.ok_or(EngineError::NoSequence)?;
    if let Some(text) = item.metadata.get(&pending_key(sequence, it.id)) {
        let mut pending: Pending = serde_json::from_str(text).map_err(|e| fail(e.to_string()))?;
        pending.link.validate().map_err(fail)?;
        if it.item != pending.plan.clip.item
            || it.source_in != pending.plan.clip.source_in
            || it.duration != pending.plan.clip.duration
            || it.speed != 1.0
            || it.reverse
            || it.frame_hold.is_some()
        {
            return Err(fail("the clip's source range changed after preparing EffectCraft; restore its range before applying"));
        }
        pending.plan.clip = (*it).clone();
        pending.plan.filmcraft_project = s.path.clone().ok_or_else(|| fail("save the FilmCraft project first"))?;
        return Ok(json!({"pending":pending}));
    }
    if let Some(text) = item.metadata.get(LINK_KEY) {
        let link: Link = serde_json::from_str(text).map_err(|e| fail(format!("invalid EffectCraft link: {e}")))?;
        link.validate().map_err(fail)?;
        return Ok(json!({"existing":link}));
    }
    let project = s.path.clone().ok_or_else(|| fail("save the FilmCraft project before opening EffectCraft"))?;
    let media = item.as_media().filter(|m| !m.offline).ok_or_else(|| fail("EffectCraft needs an online video file"))?;
    let MediaRef::File { path } = &media.media else {
        return Err(fail("EffectCraft currently supports video files; nested sequences and generated clips are not supported"));
    };
    let v = media.info.video.as_ref().ok_or_else(|| fail("the clip has no video"))?;
    if it.speed != 1.0 || it.reverse || it.frame_hold.is_some() || it.multicam.is_some() {
        return Err(fail("EffectCraft currently needs a normal-speed video clip without reverse, frame hold or multicam"));
    }
    let source = s.media.follow(path).map_err(|_| fail("source file is not available to the media pool"))?;
    Ok(
        json!({"plan":Plan {filmcraft_project:project,sequence:s.state.active_sequence.ok_or(EngineError::NoSequence)?,clip:(*it).clone(),source,width:v.width,height:v.height,rate:q.settings.frame_rate}}),
    )
}
fn stage(s: &mut Session, p: &Value) -> Result<Value> {
    let pending: Pending = serde_json::from_value(p.clone()).map_err(|e| fail(e.to_string()))?;
    pending.link.validate().map_err(fail)?;
    check_target(s, &pending.plan)?;
    let text = serde_json::to_string(&pending).map_err(|e| fail(e.to_string()))?;
    let key = pending_key(pending.plan.sequence, pending.plan.clip.id);
    s.edit("Prepare EffectCraft project", |project, _| {
        let item = project.item_mut(pending.plan.clip.item).ok_or_else(|| fail("missing source item"))?;
        item.metadata.insert(key, text);
        Ok(())
    })?;
    Ok(json!({"project":pending.link.project}))
}
fn check_target(s: &Session, plan: &Plan) -> Result<()> {
    if s.path.as_deref() != Some(&plan.filmcraft_project) {
        return Err(fail("the FilmCraft project changed while EffectCraft was preparing"));
    }
    let q = s.project.sequence(plan.sequence).ok_or_else(|| fail("the original sequence no longer exists"))?;
    let (tid, it) = q.find_item(plan.clip.id).ok_or_else(|| fail("the original clip no longer exists"))?;
    if serde_json::to_value(it).map_err(|e| fail(e.to_string()))? != serde_json::to_value(&plan.clip).map_err(|e| fail(e.to_string()))?
        || q.track(tid).is_some_and(|t| t.locked)
    {
        return Err(fail("the clip changed or its track was locked while EffectCraft was preparing; no clip was replaced"));
    }
    Ok(())
}
fn attach(s: &mut Session, p: &Value) -> Result<Value> {
    let plan: Plan = serde_json::from_value(p.get("plan").cloned().ok_or_else(|| fail("missing plan"))?).map_err(|e| fail(e.to_string()))?;
    let link: Link = serde_json::from_value(p.get("link").cloned().ok_or_else(|| fail("missing link"))?).map_err(|e| fail(e.to_string()))?;
    link.validate().map_err(fail)?;
    check_target(s, &plan)?;
    let src = s.media.open_file(&link.output, &*s.services)?;
    if !src.info().has_video() || src.info().duration + plan.rate.frame_duration() < plan.clip.duration {
        return Err(fail("the EffectCraft render is missing video or is shorter than the clip"));
    }
    let metadata = serde_json::to_string(&link).map_err(|e| fail(e.to_string()))?;
    let id = s.edit_group("Edit in EffectCraft", |s| {
        let id = crate::commands::import_streamed(s, &link.output, None)?;
        s.edit("Link EffectCraft render", |project, _| {
            if let Some(source) = project.item_mut(plan.clip.item) {
                source.metadata.remove(&pending_key(plan.sequence, plan.clip.id));
            }
            let item = project.item_mut(id).ok_or_else(|| fail("missing imported render"))?;
            item.name = format!("{} · EffectCraft", plan.clip.name);
            item.metadata.insert(LINK_KEY.into(), metadata);
            let (_, dst) = project.sequence_mut(plan.sequence).and_then(|q| q.find_item_mut(plan.clip.id)).ok_or_else(|| fail("clip disappeared"))?;
            dst.item = id;
            dst.source_in = filmcraft_time::Tick::ZERO;
            let target = dst.clone();
            for e in &mut dst.effects {
                crate::clip_ops::retime_effect(e, &plan.clip, &target, false);
            }
            Ok(())
        })?;
        Ok(id)
    })?;
    Ok(json!({"item":id.0,"project":link.project}))
}
/// A completed render must invalidate media even if an overlapping generic file scan has an
/// older stamp. This refresh is scoped to files explicitly linked in the current project.
fn refresh(s: &mut Session, p: &Value) -> Result<Value> {
    let output = p.get("output").and_then(Value::as_str).ok_or_else(|| fail("missing EffectCraft output path"))?;
    let ids: Vec<_> = s
        .project
        .items
        .values()
        .filter_map(|item| {
            let link: Link = serde_json::from_str(item.metadata.get(LINK_KEY)?).ok()?;
            (link.validate().is_ok() && link.output == output).then_some(item.id)
        })
        .collect();
    if ids.is_empty() {
        return Ok(json!({"refreshed":[]}));
    }
    let src = s.media.open_file(output, &*s.services)?;
    if !src.info().has_video() {
        return Err(fail("EffectCraft output has no video"));
    }
    let identity = crate::relink::identity_of(&*s.services, output).ok();
    let project = std::sync::Arc::make_mut(&mut s.project);
    for id in &ids {
        if let Some(media) = project.item_mut(*id).and_then(|i| i.as_media_mut()) {
            media.media = MediaRef::File { path: output.into() };
            media.offline = false;
            media.info = src.info().clone();
            media.identity = identity;
            s.media.insert_file(*id, output, src.clone());
        }
    }
    s.bump_view();
    Ok(json!({"refreshed":ids}))
}
fn rebind(s: &mut Session, p: &Value) -> Result<Value> {
    let old: Link = serde_json::from_value(p.get("old").cloned().ok_or_else(|| fail("missing old link"))?).map_err(|e| fail(e.to_string()))?;
    let new: Link = serde_json::from_value(p.get("new").cloned().ok_or_else(|| fail("missing new link"))?).map_err(|e| fail(e.to_string()))?;
    old.validate().map_err(fail)?;
    new.validate().map_err(fail)?;
    let ids: Vec<_> = s
        .project
        .items
        .values()
        .filter(|i| i.metadata.get(LINK_KEY).and_then(|text| serde_json::from_str::<Link>(text).ok()).as_ref() == Some(&old))
        .map(|i| i.id)
        .collect();
    if ids.is_empty() {
        return Err(fail("the EffectCraft link is no longer part of this project"));
    }
    let text = serde_json::to_string(&new).map_err(|e| fail(e.to_string()))?;
    s.edit("Move EffectCraft working copy", |project, _| {
        for id in &ids {
            if let Some(item) = project.item_mut(*id) {
                item.metadata.insert(LINK_KEY.into(), text.clone());
            }
        }
        Ok(())
    })?;
    Ok(json!({"items":ids}))
}
pub(crate) fn commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "clip.editInEffectCraft",
            label: "EffectCraft…",
            menu: &["Clip"],
            shortcut: None,
            params: r#"{"clips":[id]?}"#,
            enabled: crate::commands::has_selection,
            run: prepare,
            journal: false,
        },
        CommandSpec {
            id: "effectcraft.attach",
            label: "EffectCraft…",
            menu: &[],
            shortcut: None,
            params: r#"{"plan":object,"link":object}"#,
            enabled: crate::commands::has_seq,
            run: attach,
            journal: true,
        },
        CommandSpec {
            id: "effectcraft.stage",
            label: "EffectCraft…",
            menu: &[],
            shortcut: None,
            params: r#"{"plan":object,"link":object}"#,
            enabled: crate::commands::has_seq,
            run: stage,
            journal: true,
        },
        CommandSpec {
            id: "clip.applyEffectCraft",
            label: "Apply EffectCraft Changes",
            menu: &["Clip"],
            shortcut: None,
            params: r#"{"clips":[id]?}"#,
            enabled: crate::commands::has_selection,
            run: prepare,
            journal: false,
        },
        CommandSpec {
            id: "effectcraft.refresh",
            label: "EffectCraft…",
            menu: &[],
            shortcut: None,
            params: r#"{"output":path}"#,
            enabled: |_| Ok(()),
            run: refresh,
            journal: false,
        },
        CommandSpec {
            id: "effectcraft.rebind",
            label: "EffectCraft…",
            menu: &[],
            shortcut: None,
            params: r#"{"old":object,"new":object}"#,
            enabled: crate::commands::has_seq,
            run: rebind,
            journal: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media_test_util::{make_movie, session_with, tmp_dir};
    use filmcraft_media::DemoScene;
    use filmcraft_time::Tick;
    fn fixture() -> (Session, Plan, Link) {
        let root = tmp_dir("effectcraft-engine");
        let source = root.join("source.mov");
        let output = root.join("render.mov");
        make_movie(&source, DemoScene::Aurora, 160, 90, 12);
        make_movie(&output, DemoScene::CityNight, 160, 90, 8);
        let (mut s, _, seq) = session_with(&[&source]);
        s.path = Some(root.join("edit.fcproj").to_string_lossy().into_owned());
        let q = std::sync::Arc::make_mut(&mut s.project).sequence_mut(seq).unwrap();
        let v = &mut q.video_tracks[0].items[0];
        v.source_in = FrameRate::FPS_24.tick_of(4);
        v.duration = FrameRate::FPS_24.tick_of(8);
        s.state.selection = vec![v.id];
        let p = s.execute("clip.editInEffectCraft", json!({})).unwrap();
        let plan: Plan = serde_json::from_value(p["plan"].clone()).unwrap();
        let link =
            Link { project: root.join("clip.ecproj").to_string_lossy().into_owned(), output: output.to_string_lossy().into_owned(), token: "test".into() };
        (s, plan, link)
    }
    #[test]
    fn attach_preserves_clip_and_audio_and_is_one_undoable_edit() {
        let (mut s, p, l) = fixture();
        let before = s.project.clone();
        let undo = s.history.undo.len();
        let audio = s.active_sequence().unwrap().audio_tracks.clone();
        let result = s.execute("effectcraft.attach", json!({"plan":p,"link":l})).unwrap();
        let q = s.active_sequence().unwrap();
        let c = &q.video_tracks[0].items[0];
        assert_eq!(c.id, p.clip.id);
        assert_eq!(c.start, p.clip.start);
        assert_eq!(c.duration, p.clip.duration);
        assert_eq!(c.source_in, Tick::ZERO);
        assert_ne!(c.item, p.clip.item);
        assert_eq!(q.audio_tracks, audio);
        let id = ItemId(result["item"].as_u64().unwrap());
        assert_eq!(serde_json::from_str::<Link>(&s.project.item(id).unwrap().metadata[LINK_KEY]).unwrap(), l);
        assert_eq!(s.history.undo.len(), undo + 1);
        assert_eq!(s.execute("clip.editInEffectCraft", json!({})).unwrap()["existing"], json!(l));
        assert!(s.undo().is_some());
        assert_eq!(s.project, before);
        assert!(s.redo().is_some());
        let serialized = serde_json::to_string(&*s.project).unwrap();
        let restored: filmcraft_project::Project = serde_json::from_str(&serialized).unwrap();
        assert!(restored.item(id).unwrap().metadata.contains_key(LINK_KEY));
    }
    #[test]
    fn hostile_parameters_and_changed_targets_leave_project_intact() {
        let (mut s, p, l) = fixture();
        let before = s.project.clone();
        for v in [json!({"clips":"oops"}), json!({"clips":[-1]}), json!({"clips":[]}), json!({"clips":[999999999999u64]})] {
            assert!(s.execute("clip.editInEffectCraft", v).is_err());
            assert_eq!(s.project, before);
        }
        for v in [json!({}), json!({"plan":null,"link":l}), json!({"plan":p,"link":{"project":"relative","output":"/tmp/render.mov","token":"x"}})] {
            assert!(s.execute("effectcraft.attach", v).is_err());
            assert_eq!(s.project, before);
        }
        let mut wrong = p.clone();
        wrong.clip.start += Tick(1);
        assert!(s.execute("effectcraft.attach", json!({"plan":wrong,"link":l})).is_err());
        assert_eq!(s.project, before);
        s.path = None;
        assert!(s.execute("clip.editInEffectCraft", json!({})).is_err());
        assert!(s.execute("effectcraft.attach", json!({"plan":p,"link":l})).is_err());
        assert_eq!(s.project, before);
    }
    #[test]
    fn locked_speed_and_short_render_are_rejected() {
        let (mut s, p, l) = fixture();
        std::sync::Arc::make_mut(&mut s.project).sequence_mut(p.sequence).unwrap().video_tracks[0].locked = true;
        assert!(s.execute("clip.editInEffectCraft", json!({})).is_err());
        assert!(s.execute("effectcraft.attach", json!({"plan":p,"link":l})).is_err());
        let q = std::sync::Arc::make_mut(&mut s.project).sequence_mut(p.sequence).unwrap();
        q.video_tracks[0].locked = false;
        q.video_tracks[0].items[0].speed = 2.0;
        assert!(s.execute("clip.editInEffectCraft", json!({})).is_err());
        let (mut s, p, l) = fixture();
        make_movie(Path::new(&l.output), DemoScene::Aurora, 160, 90, 1);
        let before = s.project.clone();
        assert!(s.execute("effectcraft.attach", json!({"plan":p,"link":l})).is_err());
        assert_eq!(s.project, before);
    }
    #[test]
    fn explicit_refresh_updates_picture_without_an_edit_or_a_prior_stamp() {
        use crate::media_test_util::frame_rgba;
        let (mut s, p, l) = fixture();
        s.execute("effectcraft.attach", json!({"plan":p,"link":l})).unwrap();
        let before = frame_rgba(&mut s, 1, 1.0).2;
        let undo = s.history.undo.len();
        s.saved_revision = s.revision;
        make_movie(Path::new(&l.output), DemoScene::Aurora, 160, 90, 8);
        s.media_stamps.clear(); // A concurrent scan can have no remembered output stamp yet.
        let refreshed = s.execute("effectcraft.refresh", json!({"output":l.output})).unwrap();
        assert_eq!(refreshed["refreshed"].as_array().unwrap().len(), 1);
        assert_ne!(frame_rgba(&mut s, 1, 1.0).2, before);
        assert_eq!(s.history.undo.len(), undo);
        assert!(!s.is_dirty());
        assert!(s.execute("effectcraft.refresh", json!({"output":false})).is_err());
        assert_eq!(s.execute("effectcraft.refresh", json!({"output":"/not-a-linked-file.mov"})).unwrap(), json!({"refreshed":[]}));
    }
    #[test]
    fn automatic_effect_points_do_not_look_like_a_concurrent_edit() {
        let (mut s, mut p, l) = fixture();
        let c = &mut std::sync::Arc::make_mut(&mut s.project).sequence_mut(p.sequence).unwrap().video_tracks[0].items[0];
        c.effects = vec![filmcraft_project::find_effect("motion").unwrap().instance()];
        p.clip = c.clone();
        assert!(s.execute("effectcraft.attach", json!({"plan":p,"link":l})).is_ok());
    }
    #[test]
    fn staging_keeps_the_original_clip_and_persists_its_pending_link() {
        let (mut s, p, l) = fixture();
        let before = s.project.clone();
        s.execute("effectcraft.stage", json!({"plan":p,"link":l})).unwrap();
        assert_eq!(s.active_sequence().unwrap().video_tracks[0].items[0], p.clip);
        assert_eq!(s.project.items.len(), before.items.len());
        let pending = s.execute("clip.editInEffectCraft", json!({})).unwrap();
        assert_eq!(pending["pending"]["link"], json!(l));
        let saved = serde_json::to_string(&*s.project).unwrap();
        let loaded: filmcraft_project::Project = serde_json::from_str(&saved).unwrap();
        assert!(loaded.item(p.clip.item).unwrap().metadata.contains_key(&pending_key(p.sequence, p.clip.id)));
        assert!(s.execute("effectcraft.stage", json!({"plan":null,"link":l})).is_err());
        assert!(s.undo().is_some());
        assert_eq!(s.project, before);
    }
    #[test]
    fn migrated_link_keeps_the_previous_picture_until_explicit_apply() {
        use crate::media_test_util::frame_rgba;
        let (mut s, p, l) = fixture();
        s.execute("effectcraft.attach", json!({"plan":p,"link":l})).unwrap();
        let picture = frame_rgba(&mut s, 1, 1.0).2;
        let clip = s.active_sequence().unwrap().video_tracks[0].items[0].clone();
        let dir = tmp_dir("effectcraft-migrated");
        let new = Link {
            project: dir.join("clip.ecproj").to_string_lossy().into_owned(),
            output: dir.join("render.mov").to_string_lossy().into_owned(),
            token: "migrated".into(),
        };
        make_movie(Path::new(&new.output), DemoScene::Aurora, 160, 90, 8);
        assert!(s.execute("effectcraft.rebind", json!({})).is_err());
        s.execute("effectcraft.rebind", json!({"old":l,"new":new})).unwrap();
        assert_eq!(frame_rgba(&mut s, 1, 1.0).2, picture);
        assert_eq!(s.active_sequence().unwrap().video_tracks[0].items[0], clip);
        s.execute("effectcraft.refresh", json!({"output":new.output})).unwrap();
        assert_ne!(frame_rgba(&mut s, 1, 1.0).2, picture);
        assert_eq!(s.active_sequence().unwrap().video_tracks[0].items[0], clip);
    }
    use std::path::Path;
}
