//! MutaGen and the voxel / mesh export in the browser editor.

use super::{json_str, App};
use crate::mutagen::{Member, MutationConfig, Rng, TREE};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// One member of a generation with its preview image.
struct MutaMember {
    member: Member,
    png: Arc<Vec<u8>>,
    caption: String,
}

#[derive(Default)]
struct Muta {
    generations: Vec<Vec<MutaMember>>,
    current: usize,
    running: bool,
    error: String,
}

#[derive(Default)]
struct ExportJob {
    running: bool,
    kind: String,
    message: String,
    error: String,
    /// mesh file for download (name, contents)
    file: Option<(String, Arc<Vec<u8>>)>,
    folder: String,
}

pub(super) struct ToolsState {
    muta: Mutex<Muta>,
    muta_gen: AtomicU64,
    muta_ver: AtomicU64,
    export: Mutex<ExportJob>,
    export_stop: AtomicBool,
    export_progress: AtomicU32,
}

impl ToolsState {
    pub(super) fn new() -> ToolsState {
        ToolsState {
            muta: Mutex::new(Muta::default()),
            muta_gen: AtomicU64::new(0),
            muta_ver: AtomicU64::new(1),
            export: Mutex::new(ExportJob::default()),
            export_stop: AtomicBool::new(false),
            export_progress: AtomicU32::new(0),
        }
    }
}

const PREVIEW_W: usize = 144;

fn num<T: std::str::FromStr>(q: &HashMap<String, String>, k: &str) -> Option<T> {
    q.get(k).and_then(|v| v.trim().parse::<T>().ok())
}

pub(super) fn muta_json(app: &App) -> String {
    let t = &app.tools;
    let m = t.muta.lock().unwrap();
    let members: Vec<String> = m
        .generations
        .get(m.current)
        .map(|g| {
            g.iter()
                .enumerate()
                .map(|(i, mm)| format!("{{\"label\":{},\"caption\":{},\"i\":{i}}}", json_str(TREE[i].0), json_str(&mm.caption)))
                .collect()
        })
        .unwrap_or_default();
    format!(
        "{{\"ver\":{},\"running\":{},\"current\":{},\"count\":{},\"error\":{},\"members\":[{}],\"layout\":[{}],\"parents\":[{}]}}",
        t.muta_ver.load(Ordering::SeqCst),
        m.running,
        m.current,
        m.generations.len(),
        json_str(&m.error),
        members.join(","),
        crate::mutagen::LAYOUT.iter().map(|(x, y)| format!("[{x},{y}]")).collect::<Vec<_>>().join(","),
        TREE.iter().map(|(_, p)| p.map_or("-1".to_string(), |p| p.to_string())).collect::<Vec<_>>().join(","),
    )
}

/// Starts a generation from the editor's scene or from member `i` of the
/// shown generation (`CreateMutation`).
pub(super) fn muta_start(app: &Arc<App>, q: &HashMap<String, String>) -> Result<(), String> {
    let t = &app.tools;
    let parent = match num::<usize>(q, "i") {
        Some(i) => {
            let m = t.muta.lock().unwrap();
            m.generations.get(m.current).and_then(|g| g.get(i)).map(|mm| mm.member.scene.clone()).ok_or("no such member")?
        }
        None => app.scene.lock().unwrap().scene.clone(),
    };
    let d = MutationConfig::default();
    let cfg = MutationConfig {
        formula_weight: num(q, "formula_weight").unwrap_or(d.formula_weight),
        params_weight: num(q, "params_weight").unwrap_or(d.params_weight),
        params_strength: num(q, "params_strength").unwrap_or(d.params_strength),
        julia_weight: num(q, "julia_weight").unwrap_or(d.julia_weight),
        julia_strength: num(q, "julia_strength").unwrap_or(d.julia_strength),
        iterations_weight: num(q, "its_weight").unwrap_or(d.iterations_weight),
        iterations_strength: num(q, "its_strength").unwrap_or(d.iterations_strength),
        probing: q.get("probing").is_none_or(|v| v == "true"),
        ..d
    };
    let gen = t.muta_gen.fetch_add(1, Ordering::SeqCst) + 1;
    {
        let mut m = t.muta.lock().unwrap();
        // a new generation follows the shown one; later ones are dropped
        let keep = if m.generations.is_empty() { 0 } else { m.current + 1 };
        m.generations.truncate(keep);
        m.generations.push(Vec::new());
        m.current = m.generations.len() - 1;
        m.running = true;
        m.error.clear();
    }
    t.muta_ver.fetch_add(1, Ordering::SeqCst);
    let app = app.clone();
    std::thread::spawn(move || {
        let t = &app.tools;
        let mut rng = Rng::from_time();
        let ph = ((PREVIEW_W as f64 * parent.height as f64 / parent.width.max(1) as f64).round() as usize).max(2);
        let cancel = || t.muta_gen.load(Ordering::SeqCst) != gen;
        let mut on_member = |_i: usize, m: &Member| {
            let png = crate::mutagen::preview(&m.scene, PREVIEW_W, ph, false, 0)
                .map(|(rgb, w, h)| crate::png::encode_rgb(w, h, &rgb))
                .unwrap_or_default();
            let mm = MutaMember { member: m.clone(), png: Arc::new(png), caption: crate::mutagen::caption(&m.scene) };
            let mut s = t.muta.lock().unwrap();
            if let Some(g) = s.generations.last_mut() {
                g.push(mm);
            }
            drop(s);
            t.muta_ver.fetch_add(1, Ordering::SeqCst);
        };
        let r = crate::mutagen::generation(&cfg, &parent, &mut rng, 0, &mut on_member, &cancel);
        let mut s = t.muta.lock().unwrap();
        s.running = false;
        if let Err(e) = r {
            if e != "cancelled" {
                s.error = e;
            }
        }
        drop(s);
        t.muta_ver.fetch_add(1, Ordering::SeqCst);
    });
    Ok(())
}

pub(super) fn muta_img(app: &App, g: usize, i: usize) -> Option<Arc<Vec<u8>>> {
    let m = app.tools.muta.lock().unwrap();
    m.generations.get(g).and_then(|gg| gg.get(i)).map(|mm| mm.png.clone()).filter(|p| !p.is_empty())
}

/// Loads member `i` of the shown generation into the editor.
pub(super) fn muta_use(app: &Arc<App>, i: usize) -> Result<(), String> {
    let (s, label) = {
        let m = app.tools.muta.lock().unwrap();
        let mm = m.generations.get(m.current).and_then(|g| g.get(i)).ok_or("no such member")?;
        (mm.member.scene.clone(), TREE[i].0)
    };
    let mut st = app.scene.lock().unwrap();
    st.scene = s;
    st.title = format!("mutation {label}");
    st.notes.clear();
    drop(st);
    app.restart();
    Ok(())
}

pub(super) fn muta_nav(app: &App, d: i64) {
    let mut m = app.tools.muta.lock().unwrap();
    if m.running {
        return;
    }
    let n = m.generations.len() as i64;
    if n > 0 {
        m.current = (m.current as i64 + d).clamp(0, n - 1) as usize;
    }
    drop(m);
    app.tools.muta_ver.fetch_add(1, Ordering::SeqCst);
}

pub(super) fn muta_stop(app: &App) {
    app.tools.muta_gen.fetch_add(1, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// export

pub(super) fn export_json(app: &App) -> String {
    let t = &app.tools;
    let e = t.export.lock().unwrap();
    format!(
        "{{\"running\":{},\"kind\":{},\"progress\":{},\"message\":{},\"error\":{},\"file\":{},\"folder\":{}}}",
        e.running,
        json_str(&e.kind),
        t.export_progress.load(Ordering::Relaxed) as f64 / 10.0,
        json_str(&e.message),
        json_str(&e.error),
        e.file.as_ref().map_or("null".into(), |(n, d)| format!("{{\"name\":{},\"size\":{}}}", json_str(n), d.len())),
        json_str(&e.folder),
    )
}

fn voxel_params(sc: &crate::scene::Scene, q: &HashMap<String, String>) -> crate::voxel::VoxelParams {
    let slices = num::<u32>(q, "slices").unwrap_or(100).clamp(2, 4096);
    let mut vp = crate::voxel::VoxelParams::from_scene(sc, slices);
    if q.get("test").is_some_and(|v| v == "iterations") {
        vp.test = crate::voxel::ObjectTest::Iterations;
    }
    if let Some(d) = num::<f64>(q, "de").filter(|d| *d > 0.0) {
        vp.de = d;
        vp.min_de = vp.min_de.max(d * 0.254);
    }
    if let Some(s) = num::<f64>(q, "zscale").filter(|d| *d > 0.0) {
        vp.scale[2] = s;
    }
    vp.default_orientation = q.get("axes").is_some_and(|v| v == "true");
    vp
}

fn mesh_params(q: &HashMap<String, String>) -> crate::mesh::MeshParams {
    let d = crate::mesh::MeshParams::default();
    crate::mesh::MeshParams {
        resolution: num::<usize>(q, "resolution").unwrap_or(d.resolution).clamp(8, 1024),
        sharpness: num(q, "sharpness").unwrap_or(d.sharpness),
        scale: num::<f64>(q, "scale").filter(|s| *s > 0.0).unwrap_or(d.scale),
        colors: q.get("colors").is_some_and(|v| v == "true"),
        close: q.get("close").is_some_and(|v| v == "true"),
        smooth: num(q, "smooth").unwrap_or(0u32).min(100),
        ..d
    }
}

/// A small preview of the voxel object.
pub(super) fn voxel_preview(app: &App, q: &HashMap<String, String>) -> Result<Vec<u8>, String> {
    let sc = app.scene.lock().unwrap().scene.clone();
    let vp = voxel_params(&sc, q);
    let (rgb, w, h) = crate::voxel::preview(&sc, &vp, 120, 0)?;
    Ok(crate::png::encode_rgb(w, h, &rgb))
}

pub(super) fn export_start(app: &Arc<App>, q: &HashMap<String, String>) -> Result<(), String> {
    let t = &app.tools;
    let kind = q.get("kind").cloned().unwrap_or_default();
    if kind != "voxel" && kind != "mesh" {
        return Err("export kind: voxel or mesh".into());
    }
    let (sc, title) = {
        let s = app.scene.lock().unwrap();
        (s.scene.clone(), s.title.clone())
    };
    let name: String = title.chars().map(|c| if c.is_alphanumeric() || "-_".contains(c) { c } else { '_' }).collect();
    let name = if name.trim_matches('_').is_empty() { "mb3d".to_string() } else { name };
    let mut e = t.export.lock().unwrap();
    if e.running {
        return Err("an export is running".into());
    }
    t.export_stop.store(false, Ordering::SeqCst);
    t.export_progress.store(0, Ordering::Relaxed);
    *e = ExportJob { running: true, kind: kind.clone(), ..Default::default() };
    drop(e);
    let q = q.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let t = &app.tools;
        let cancel = || t.export_stop.load(Ordering::SeqCst);
        let r: Result<(String, Option<(String, Arc<Vec<u8>>)>, String), String> = if kind == "voxel" {
            let vp = voxel_params(&sc, &q);
            let base = q.get("folder").map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "voxels".into());
            let dir = std::env::current_dir().unwrap_or_default().join(base).join(&name);
            let progress = |i: u32, n: u32| t.export_progress.store(i * 1000 / n.max(1), Ordering::Relaxed);
            crate::voxel::export(&sc, &vp, &dir, &name, 0, &progress, &cancel).map(|n| {
                let (w, h) = vp.size();
                (format!("{n} slices of {w}x{h} written"), None, dir.to_string_lossy().into_owned())
            })
        } else {
            let mp = mesh_params(&q);
            let fmt = q.get("format").map(|s| s.as_str()).filter(|f| ["obj", "ply", "stl"].contains(f)).unwrap_or("stl").to_string();
            let progress = |i: usize, n: usize| t.export_progress.store((i * 1000 / n.max(1)) as u32, Ordering::Relaxed);
            crate::mesh::trace(&sc, &mp, 0, &progress, &cancel).and_then(|m| {
                if m.faces.is_empty() {
                    return Err("no surface found in the cube (scale, sharpness)".into());
                }
                let mut buf = Vec::new();
                let w = match fmt.as_str() {
                    "obj" => m.write_obj(&mut buf),
                    "ply" => m.write_ply(&mut buf),
                    _ => m.write_stl(&mut buf),
                };
                w.map_err(|e| e.to_string())?;
                let (edges, open, _) = m.edge_stats();
                let msg = format!(
                    "{} vertices, {} triangles{}",
                    m.vertices.len(),
                    m.faces.len(),
                    if open == 0 { ", closed".to_string() } else { format!(", {open} of {edges} edges open") }
                );
                Ok((msg, Some((format!("{name}.{fmt}"), Arc::new(buf))), String::new()))
            })
        };
        let mut e = t.export.lock().unwrap();
        e.running = false;
        match r {
            Ok((msg, file, folder)) => {
                e.message = msg;
                e.file = file;
                e.folder = folder;
                t.export_progress.store(1000, Ordering::Relaxed);
            }
            Err(err) => e.error = if err == "cancelled" { "stopped".into() } else { err },
        }
    });
    Ok(())
}

pub(super) fn export_stop(app: &App) {
    app.tools.export_stop.store(true, Ordering::SeqCst);
}

pub(super) fn export_file(app: &App) -> Option<(String, Arc<Vec<u8>>)> {
    app.tools.export.lock().unwrap().file.clone()
}
