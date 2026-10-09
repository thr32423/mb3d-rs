//! The animation part of the browser editor (MB3D's animation maker):
//! keyframes taken from the editor, their settings, a quick flipbook
//! preview of all frames and the rendering of the frames into files.

use super::{json_list, json_str, App};
use crate::anim::{Animation, Interpolation, Keyframe, OutputFormat};
use crate::animfile::Preview;
use crate::frames::{FrameResult, FrameRun};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Flipbook preview: small images of the frames.
#[derive(Default)]
struct Flipbook {
    gen: u64,
    frames: Vec<Option<Arc<Vec<u8>>>>,
    /// file index of each preview image
    indices: Vec<i64>,
    width: usize,
    running: bool,
    error: String,
}

/// Rendering the frames into files.
#[derive(Default)]
struct FileJob {
    running: bool,
    total: usize,
    done: usize,
    skipped: usize,
    /// number of the frame being calculated (1-based within the run)
    current: usize,
    seconds: f64,
    last: String,
    error: String,
    folder: String,
}

pub(super) struct AnimState {
    anim: Mutex<Animation>,
    /// keyframe ids (parallel to `anim.keyframes`) and their preview images
    ids: Mutex<Vec<u64>>,
    thumbs: Mutex<HashMap<u64, Preview>>,
    next_id: AtomicU64,
    ver: AtomicU64,
    default_frames: AtomicU32,
    thumbs_busy: AtomicBool,
    flip: Mutex<Flipbook>,
    flip_gen: AtomicU64,
    /// progress of the frame being calculated, 1/1000
    flip_progress: AtomicU32,
    job: Mutex<FileJob>,
    job_progress: AtomicU32,
    stop: AtomicBool,
}

impl AnimState {
    pub(super) fn new() -> AnimState {
        AnimState {
            anim: Mutex::new(Animation { output_folder: "frames".into(), name: "anim".into(), ..Default::default() }),
            ids: Mutex::new(Vec::new()),
            thumbs: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            ver: AtomicU64::new(1),
            default_frames: AtomicU32::new(50),
            thumbs_busy: AtomicBool::new(false),
            flip: Mutex::new(Flipbook::default()),
            flip_gen: AtomicU64::new(0),
            flip_progress: AtomicU32::new(0),
            job: Mutex::new(FileJob::default()),
            job_progress: AtomicU32::new(0),
            stop: AtomicBool::new(false),
        }
    }

    fn changed(&self) {
        self.ver.fetch_add(1, Ordering::SeqCst);
    }

    fn new_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Replaces the whole animation (opened file).
    fn set_all(&self, a: Animation, previews: Vec<Option<Preview>>) {
        let ids: Vec<u64> = a.keyframes.iter().map(|_| self.new_id()).collect();
        {
            let mut t = self.thumbs.lock().unwrap();
            t.clear();
            for (id, p) in ids.iter().zip(previews) {
                if let Some(p) = p {
                    t.insert(*id, p);
                }
            }
        }
        *self.ids.lock().unwrap() = ids;
        *self.anim.lock().unwrap() = a;
        self.changed();
    }
}

/// Renders the missing keyframe images in the background.
fn ensure_thumbs(app: &Arc<App>) {
    let st = &app.anim;
    if st.thumbs_busy.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let st = &app.anim;
        loop {
            let todo = {
                let a = st.anim.lock().unwrap();
                let ids = st.ids.lock().unwrap();
                let t = st.thumbs.lock().unwrap();
                ids.iter().position(|id| !t.contains_key(id)).map(|i| {
                    let mut one = a.clone();
                    one.keyframes = vec![a.keyframes[i].clone()];
                    (ids[i], one)
                })
            };
            let Some((id, one)) = todo else { break };
            let p = crate::frames::keyframe_preview(&one, 0).unwrap_or(Preview { width: 1, height: 1, rgb: vec![60, 20, 20] });
            st.thumbs.lock().unwrap().insert(id, p);
            st.changed();
        }
        st.thumbs_busy.store(false, Ordering::SeqCst);
        // a keyframe may have been added while the last image was calculated
        let missing = {
            let ids = st.ids.lock().unwrap();
            let t = st.thumbs.lock().unwrap();
            ids.iter().any(|id| !t.contains_key(id))
        };
        if missing {
            ensure_thumbs(&app);
        }
    });
}

pub(super) fn anim_json(app: &App) -> String {
    let st = &app.anim;
    let a = st.anim.lock().unwrap();
    let ids = st.ids.lock().unwrap();
    let thumbs = st.thumbs.lock().unwrap();
    let keys: Vec<String> = a
        .keyframes
        .iter()
        .zip(ids.iter())
        .enumerate()
        .map(|(i, (k, id))| {
            format!(
                "{{\"id\":{id},\"frames\":{},\"first\":{},\"thumb\":{},\"title\":{}}}",
                k.frames,
                a.file_index(a.first_frame_of(i)),
                thumbs.contains_key(id),
                json_str(k.source.as_deref().map(|s| s.split('\u{0}').next().unwrap_or("")).unwrap_or(""))
            )
        })
        .collect();
    let f = st.flip.lock().unwrap();
    let j = st.job.lock().unwrap();
    let folder = crate::animfile::resolve_output(&a, &std::env::current_dir().unwrap_or_default());
    format!(
        "{{\"ver\":{},\"width\":{},\"height\":{},\"aa\":{},\"interp\":{},\"loop\":{},\"name\":{},\"output\":{},\"output_abs\":{},\
         \"format\":{},\"start_index\":{},\"index_step\":{},\"overwrite\":{},\"depth\":{},\"stereo\":{},\"default_frames\":{},\"frame_count\":{},\
         \"keys\":[{}],\
         \"flip\":{{\"gen\":{},\"total\":{},\"ready\":{},\"indices\":{},\"width\":{},\"running\":{},\"progress\":{},\"error\":{}}},\
         \"job\":{{\"running\":{},\"total\":{},\"done\":{},\"skipped\":{},\"current\":{},\"progress\":{},\"seconds\":{:.2},\"last\":{},\"error\":{},\"folder\":{}}}}}",
        st.ver.load(Ordering::SeqCst),
        a.width,
        a.height,
        a.scale,
        json_str(if a.interpolation == Interpolation::Linear { "linear" } else { "bezier" }),
        a.looped,
        json_str(&a.name),
        json_str(&a.output_folder),
        json_str(&folder.to_string_lossy()),
        json_str(a.format.extension()),
        a.start_index,
        a.index_step,
        a.overwrite,
        a.save_depth,
        json_str(match a.stereo_bits & 0xC0 { 0x40 => "pair", 0xC0 | 0x80 => "very_left", _ => "off" }),
        st.default_frames.load(Ordering::Relaxed),
        a.frame_count(),
        keys.join(","),
        f.gen,
        f.frames.len(),
        format!("[{}]", f.frames.iter().map(|x| if x.is_some() { "1" } else { "0" }).collect::<Vec<_>>().join(",")),
        format!("[{}]", f.indices.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",")),
        f.width,
        f.running,
        st.flip_progress.load(Ordering::Relaxed) as f64 / 10.0,
        json_str(&f.error),
        j.running,
        j.total,
        j.done,
        j.skipped,
        j.current,
        st.job_progress.load(Ordering::Relaxed) as f64 / 10.0,
        j.seconds,
        json_str(&j.last),
        json_str(&j.error),
        json_str(&j.folder),
    )
}

/// Keyframe operations (`op` in the form).
pub(super) fn key_op(app: &Arc<App>, q: &HashMap<String, String>) -> Result<(), String> {
    let st = &app.anim;
    let op = q.get("op").map(String::as_str).unwrap_or("");
    let idx = q.get("i").and_then(|v| v.parse::<usize>().ok());
    let current = || app.scene.lock().unwrap().scene.clone();
    let mut a = st.anim.lock().unwrap();
    let mut ids = st.ids.lock().unwrap();
    let n = a.keyframes.len();
    let need = |i: Option<usize>| i.filter(|&i| i < n).ok_or_else(|| "no such keyframe".to_string());
    let frames = st.default_frames.load(Ordering::Relaxed);
    match op {
        "add" | "insert" => {
            let mut s = current();
            s.light_blend = None;
            if n == 0 {
                // the first keyframe sets the size of the animation (InsertFromHeader)
                a.set_size_from(&s);
            }
            let at = if op == "insert" { need(idx)? + 1 } else { n };
            a.keyframes.insert(at, Keyframe::new(s, frames));
            ids.insert(at, st.new_id());
        }
        "replace" => {
            let i = need(idx)?;
            let mut s = current();
            s.light_blend = None;
            a.keyframes[i].scene = s;
            a.keyframes[i].source = None;
            ids[i] = st.new_id();
        }
        "delete" => {
            let i = need(idx)?;
            a.keyframes.remove(i);
            let id = ids.remove(i);
            st.thumbs.lock().unwrap().remove(&id);
        }
        "up" | "down" => {
            let i = need(idx)?;
            let j = if op == "up" { i.checked_sub(1) } else { Some(i + 1).filter(|&j| j < n) };
            if let Some(j) = j {
                a.keyframes.swap(i, j);
                ids.swap(i, j);
            }
        }
        "reverse" => {
            a.keyframes.reverse();
            ids.reverse();
        }
        "clear" => {
            a.keyframes.clear();
            ids.clear();
            st.thumbs.lock().unwrap().clear();
        }
        "frames" => {
            let i = need(idx)?;
            a.keyframes[i].frames = q.get("frames").and_then(|v| v.trim().parse::<u32>().ok()).ok_or("bad frame count")?.min(100_000);
        }
        "all_frames" => {
            let f = q.get("frames").and_then(|v| v.trim().parse::<u32>().ok()).ok_or("bad frame count")?.min(100_000);
            for k in a.keyframes.iter_mut() {
                k.frames = f;
            }
        }
        "edit" => {
            // copy the parameters of a keyframe into the editor (SpeedButton8Click)
            let i = need(idx)?;
            let s = a.keyframes[i].scene.clone();
            drop(ids);
            drop(a);
            let mut sst = app.scene.lock().unwrap();
            sst.scene = s;
            sst.title = format!("keyframe {}", i + 1);
            drop(sst);
            app.restart();
            return Ok(());
        }
        "edit_frame" => {
            // an interpolated frame into the editor (light settings baked)
            let f = q.get("f").and_then(|v| v.parse::<usize>().ok()).ok_or("no frame")?;
            let mut s = a.frame_scene(f)?;
            if a.scale > 1 {
                s.scale_image(1.0 / a.scale as f64);
            }
            crate::anim::bake_light_blend(&mut s);
            let title = format!("frame {:06}", a.file_index(f));
            drop(ids);
            drop(a);
            let mut sst = app.scene.lock().unwrap();
            sst.scene = s;
            sst.title = title;
            drop(sst);
            app.restart();
            return Ok(());
        }
        _ => return Err(format!("unknown keyframe operation '{op}'")),
    }
    drop(ids);
    drop(a);
    st.changed();
    ensure_thumbs(app);
    Ok(())
}

/// Animation settings from the form.
pub(super) fn set_settings(app: &Arc<App>, q: &HashMap<String, String>) -> Result<(), String> {
    let st = &app.anim;
    let mut a = st.anim.lock().unwrap();
    let int = |k: &str| q.get(k).map(|v| v.trim().parse::<i64>().map_err(|_| format!("{k}: bad number '{v}'"))).transpose();
    let flag = |k: &str| q.get(k).map(|v| v == "true" || v == "1" || v == "on");
    let mut resize = false;
    if let Some(v) = int("width")? {
        resize |= v as i32 != a.width;
        a.width = v.clamp(1, 65535) as i32;
    }
    if let Some(v) = int("height")? {
        resize |= v as i32 != a.height;
        a.height = v.clamp(1, 65535) as i32;
    }
    if let Some(v) = int("aa")? {
        a.scale = v.clamp(1, 8) as u32;
    }
    if let Some(v) = q.get("interp") {
        a.interpolation = if v == "linear" { Interpolation::Linear } else { Interpolation::Bezier };
    }
    if let Some(v) = flag("loop") {
        a.looped = v;
    }
    if let Some(v) = q.get("name") {
        let n: String = v.trim().chars().filter(|c| !"/\\:*?\"<>|".contains(*c)).collect();
        a.name = if n.is_empty() { "anim".into() } else { n };
    }
    if let Some(v) = q.get("output") {
        a.output_folder = v.trim().to_string();
    }
    if let Some(v) = q.get("format") {
        a.format = OutputFormat::parse(v)?;
        if a.format == OutputFormat::Jpg {
            a.format = OutputFormat::Png;
        }
    }
    if let Some(v) = int("start_index")? {
        a.start_index = v;
    }
    if let Some(v) = int("index_step")? {
        a.index_step = v.max(1);
    }
    if let Some(v) = flag("overwrite") {
        a.overwrite = v;
    }
    if let Some(v) = flag("depth") {
        a.save_depth = v;
    }
    if let Some(v) = q.get("stereo") {
        a.stereo_bits = match v.as_str() {
            "pair" => 0x40,
            "very_left" => 0xC0,
            _ => 0,
        };
    }
    if let Some(v) = int("default_frames")? {
        st.default_frames.store(v.clamp(0, 100_000) as u32, Ordering::Relaxed);
    }
    drop(a);
    if resize {
        // keyframe images have the aspect of the animation
        st.thumbs.lock().unwrap().clear();
        ensure_thumbs(app);
    }
    st.changed();
    Ok(())
}

/// Starts the flipbook preview: every `every`-th frame at `width` pixels.
pub(super) fn start_flipbook(app: &Arc<App>, q: &HashMap<String, String>) -> Result<(), String> {
    let st = &app.anim;
    let a = st.anim.lock().unwrap().clone();
    if a.keyframes.len() < 2 {
        return Err("add at least two keyframes".into());
    }
    let width = q.get("width").and_then(|v| v.parse::<usize>().ok()).unwrap_or(200).clamp(32, 1024);
    let every = q.get("every").and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).max(1);
    let fast = q.get("fast").is_none_or(|v| v == "true");
    let run = FrameRun { every, preview: width as f64 / a.width.max(1) as f64, ..Default::default() };
    let frames = run.frames(&a);
    let gen = st.flip_gen.fetch_add(1, Ordering::SeqCst) + 1;
    *st.flip.lock().unwrap() = Flipbook {
        gen,
        frames: vec![None; frames.len()],
        indices: frames.iter().map(|&f| a.file_index(f)).collect(),
        width,
        running: true,
        error: String::new(),
    };
    st.changed();
    let app = app.clone();
    std::thread::spawn(move || {
        let st = &app.anim;
        let cancel = || st.flip_gen.load(Ordering::SeqCst) != gen;
        for (n, &f) in frames.iter().enumerate() {
            if cancel() {
                return;
            }
            st.flip_progress.store(0, Ordering::Relaxed);
            // map sequences follow the frame number (AniPreviewWindow)
            crate::maps::set_current_frame(f as i32 + 1);
            let r = crate::frames::frame_render_scene(&a, f, &run).and_then(|mut s| {
                if fast {
                    // like MB3D's "fast + inaccurate" preview
                    s.shadows = None;
                    s.mc.reflections = false;
                    s.vol_light = None;
                    s.deao = None;
                    if let Some(ao) = s.ao.as_mut() {
                        ao.random = 0;
                    }
                }
                let progress = |d: usize, t: usize| st.flip_progress.store((d * 1000 / t.max(1)) as u32, Ordering::Relaxed);
                crate::frames::render_scaled(&s, 1, false, &progress, &cancel)
            });
            let mut fl = st.flip.lock().unwrap();
            if fl.gen != gen {
                return;
            }
            match r {
                Ok(img) => fl.frames[n] = Some(Arc::new(crate::png::encode_rgb(img.width, img.height, &img.rgb))),
                Err(e) => {
                    if e != "cancelled" {
                        fl.error = e;
                    }
                    fl.running = false;
                    return;
                }
            }
            drop(fl);
        }
        st.flip.lock().unwrap().running = false;
    });
    Ok(())
}

pub(super) fn flip_frame(app: &App, i: usize) -> Option<Arc<Vec<u8>>> {
    app.anim.flip.lock().unwrap().frames.get(i).cloned().flatten()
}

pub(super) fn thumb(app: &App, id: u64) -> Option<Vec<u8>> {
    let t = app.anim.thumbs.lock().unwrap();
    t.get(&id).map(|p| crate::png::encode_rgb(p.width, p.height, &p.rgb))
}

/// Renders the frames into files in the background (the animation as it
/// is when the job starts).
pub(super) fn start_render(app: &Arc<App>, q: &HashMap<String, String>) -> Result<(), String> {
    let st = &app.anim;
    // lock order: anim before job (as in anim_json)
    let mut a = st.anim.lock().unwrap().clone();
    let mut job = st.job.lock().unwrap();
    if job.running {
        return Err("the frames are being rendered already".into());
    }
    if a.keyframes.len() < 2 {
        return Err("add at least two keyframes".into());
    }
    let base = std::env::current_dir().unwrap_or_default();
    a.output_folder = crate::animfile::resolve_output(&a, &base).to_string_lossy().into_owned();
    let int = |k: &str| q.get(k).and_then(|v| v.trim().parse::<i64>().ok());
    let run = FrameRun { from_index: int("from"), to_index: int("to"), ..Default::default() };
    let frames = run.frames(&a);
    if frames.is_empty() {
        return Err("no frames in this range".into());
    }
    st.stop.store(false, Ordering::SeqCst);
    *job = FileJob { running: true, total: frames.len(), folder: a.output_folder.clone(), ..Default::default() };
    drop(job);
    st.changed();
    let app = app.clone();
    std::thread::spawn(move || {
        let st = &app.anim;
        let t0 = Instant::now();
        let cancel = || st.stop.load(Ordering::SeqCst);
        for (n, &f) in frames.iter().enumerate() {
            if cancel() {
                break;
            }
            st.job.lock().unwrap().current = n + 1;
            st.job_progress.store(0, Ordering::Relaxed);
            let progress = |d: usize, t: usize| st.job_progress.store((d * 1000 / t.max(1)) as u32, Ordering::Relaxed);
            let r = crate::frames::render_frame_to_file(&a, f, &run, &progress, &cancel);
            let mut j = st.job.lock().unwrap();
            match r {
                Ok(FrameResult::Written { path, .. }) => {
                    j.done += 1;
                    j.last = path.to_string_lossy().into_owned();
                }
                Ok(FrameResult::Skipped { .. }) => j.skipped += 1,
                Err(e) => {
                    if e != "cancelled" {
                        j.error = e;
                    }
                    break;
                }
            }
            j.seconds = t0.elapsed().as_secs_f64();
        }
        let mut j = st.job.lock().unwrap();
        j.running = false;
        j.seconds = t0.elapsed().as_secs_f64();
        if cancel() && j.error.is_empty() {
            j.error = "stopped".into();
        }
        drop(j);
        st.changed();
    });
    Ok(())
}

pub(super) fn stop(app: &App) {
    app.anim.stop.store(true, Ordering::SeqCst);
    app.anim.flip_gen.fetch_add(1, Ordering::SeqCst);
    let mut f = app.anim.flip.lock().unwrap();
    f.running = false;
    drop(f);
    app.anim.changed();
}

/// The animation as a file for download: (contents, extension).
pub(super) fn save(app: &Arc<App>, fmt: &str) -> (Vec<u8>, &'static str, String) {
    let st = &app.anim;
    let a = st.anim.lock().unwrap().clone();
    if fmt == "m3a" {
        let ids = st.ids.lock().unwrap().clone();
        let t = st.thumbs.lock().unwrap();
        let previews: Vec<Option<Preview>> = ids.iter().map(|id| t.get(id).cloned()).collect();
        (crate::animfile::write_m3a(&a, &previews), "m3a", a.name.clone())
    } else {
        // self-contained: all keyframes inline
        let mut a2 = a.clone();
        for k in a2.keyframes.iter_mut() {
            k.source = None;
        }
        (crate::animfile::write_m3k(&a2, None).into_bytes(), "m3k", a.name.clone())
    }
}

/// Opens an uploaded `.m3a` / `.m3k` file.  Returns the loader notes.
pub(super) fn open(app: &Arc<App>, name: &str, data: Vec<u8>) -> Result<Vec<String>, String> {
    let stem = std::path::Path::new(name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "anim".into());
    let is_text = name.to_ascii_lowercase().ends_with(".m3k") || std::str::from_utf8(&data).is_ok_and(|t| t.contains("[keyframe]"));
    let f = if is_text {
        let text = String::from_utf8(data).map_err(|_| "not a text file".to_string())?;
        crate::animfile::parse_m3k(&text, std::path::Path::new("."), &stem)?
    } else {
        crate::animfile::read_m3a(&data, &stem)?
    };
    let mut a = f.anim;
    let mut notes = f.warnings;
    let foreign = a.output_folder.contains('\\') || a.output_folder.get(1..2) == Some(":");
    if foreign && !cfg!(windows) {
        notes.push(format!("output folder '{}' of the file is not usable here, set to 'frames'", a.output_folder));
        a.output_folder = "frames".into();
    }
    if a.format == OutputFormat::Jpg {
        a.format = OutputFormat::Png;
    }
    app.anim.set_all(a, f.previews);
    ensure_thumbs(app);
    Ok(notes)
}

pub(super) fn notes_json(n: &[String]) -> String {
    json_list(n)
}
