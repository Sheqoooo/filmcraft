//! Copy a selected effect snapshot; paste it without placing or replacing any timeline clips.
use crate::{EngineError, Result, Session};
use filmcraft_project::{EffectKind, TrackKind};
use serde_json::{Value, json};

pub(crate) fn copy(s: &mut Session, _: &Value) -> Result<Value> {
    let sel = s.state.selected_effect.as_ref().ok_or_else(|| EngineError::Other("select an effect first".into()))?;
    let source = s.active_sequence().and_then(|q| q.find_item(sel.clip)).map(|(_, it)| it).ok_or_else(|| EngineError::Other("no such source clip".into()))?;
    let index = sel.index(&source.effects).ok_or_else(|| EngineError::Other("no such effect on source clip".into()))?;
    let effect = source.effects.get(index).ok_or_else(|| EngineError::Other("no such effect".into()))?;
    let def = effect.def().ok_or_else(|| EngineError::Other("unknown effect".into()))?;
    if !matches!(def.kind, EffectKind::Video | EffectKind::Audio) || filmcraft_project::graphic::is_layer_id(def.id) {
        return Err(EngineError::Other("only clip effects can be copied".into()));
    }
    s.state.effect_clipboard = Some((source.clone(), index));
    s.state.clipboard.clear();
    s.state.clipboard_markers.clear();
    Ok(json!({"copied": 1}))
}

pub(crate) fn paste(s: &mut Session, p: &Value) -> Result<Value> {
    let (source, index) = s.state.effect_clipboard.clone().ok_or_else(|| EngineError::Other("copy an effect first".into()))?;
    let effect = source.effects.get(index).ok_or_else(|| EngineError::Other("no copied effect".into()))?;
    let def = effect.def().ok_or_else(|| EngineError::Other("unknown copied effect".into()))?;
    let audio = def.kind == EffectKind::Audio;
    if let Some(clips) = p.get("clips")
        && clips.as_array().is_none_or(|ids| ids.iter().any(|v| v.as_u64().is_none()))
    {
        return Err(EngineError::Other("destination clips must be a list of clip ids".into()));
    }
    let named = crate::commands::clips_p(s, p);
    let q = s.active_sequence().ok_or(EngineError::NoSequence)?;
    if named.iter().any(|c| q.find_item(*c).is_none()) {
        return Err(EngineError::Other("no such destination clip".into()));
    }
    let targets = if audio { crate::commands::audio_effect_targets(s, &named) } else { named };
    let targets: Vec<_> =
        targets.into_iter().filter(|c| q.find_item(*c).and_then(|(tid, _)| q.track(tid)).is_some_and(|t| (t.kind == TrackKind::Audio) == audio)).collect();
    if targets.is_empty() {
        return Err(EngineError::Other("select compatible destination clips first".into()));
    }
    if targets.iter().any(|c| q.find_item(*c).and_then(|(tid, _)| q.track(tid)).is_some_and(|t| t.locked)) {
        return Err(EngineError::Other("destination track is locked".into()));
    }
    let n = s.edit_sequence("Paste Effect", |q, _, _| {
        let mut count = 0;
        for t in q.all_tracks_mut() {
            for dst in t.items.iter_mut().filter(|it| targets.contains(&it.id)) {
                let mut copied = effect.clone();
                // Keep animation offsets relative to the destination's source in-point.
                crate::clip_ops::retime_effect(&mut copied, &source, dst, false);
                if def.intrinsic
                    && let Some(slot) = dst.effects.iter_mut().find(|e| e.effect == copied.effect)
                {
                    *slot = copied;
                } else {
                    let pos = if audio {
                        dst.effects.len()
                    } else {
                        dst.effects.iter().position(|e| e.def().is_some_and(|d| d.intrinsic)).unwrap_or(dst.effects.len())
                    };
                    dst.effects.insert(pos, copied);
                }
                count += 1;
            }
        }
        Ok(count)
    })?;
    Ok(json!({"clips": n}))
}
