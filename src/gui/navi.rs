//! MB3D's Navigator: a separate window with its own copy of the parameters
//! and its own fast preview. Walking there does not change the editor until
//! "View to main" (or one of the "Send values" buttons) is pressed;
//! "Parameter" takes the editor's parameters again.

use super::{json_str, nav_ops, pick_in, preview_scene, App, Pick};
use crate::scene::Scene;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

#[derive(Default)]
struct NaviOut {
    png: Arc<Vec<u8>>,
    img_ver: u64,
    w: usize,
    h: usize,
    rendering: bool,
    stage: String,
    error: String,
}

pub(super) struct NaviState {
    scene: Mutex<Option<Scene>>,
    gen: AtomicU64,
    job: Mutex<Option<u64>>,
    cv: Condvar,
    out: Mutex<NaviOut>,
    pick: Mutex<Option<Pick>>,
    progress: AtomicU32,
    /// preview width in pixels (MB3D: 640 at 100 %)
    width: AtomicU32,
    /// "HiQual": half the raystep against overstepping
    hiq: AtomicU32,
    started: Mutex<bool>,
}

impl NaviState {
    pub(super) fn new() -> NaviState {
        NaviState {
            scene: Mutex::new(None),
            gen: AtomicU64::new(0),
            job: Mutex::new(None),
            cv: Condvar::new(),
            out: Mutex::new(NaviOut::default()),
            pick: Mutex::new(None),
            progress: AtomicU32::new(0),
            width: AtomicU32::new(640),
            hiq: AtomicU32::new(0),
            started: Mutex::new(false),
        }
    }
}

fn ensure_worker(app: &Arc<App>) {
    let mut s = app.navi.started.lock().unwrap();
    if !*s {
        *s = true;
        let app = app.clone();
        std::thread::spawn(move || worker(app));
    }
}

fn restart(app: &Arc<App>) {
    ensure_worker(app);
    let n = &app.navi;
    let gen = n.gen.fetch_add(1, Ordering::SeqCst) + 1;
    *n.job.lock().unwrap() = Some(gen);
    n.cv.notify_all();
}

/// The navigator's scene; a copy of the editor's on first use.
fn scene(app: &App) -> Scene {
    let mut s = app.navi.scene.lock().unwrap();
    if s.is_none() {
        *s = Some(app.scene.lock().unwrap().scene.clone());
    }
    s.clone().unwrap()
}

fn set_scene(app: &Arc<App>, sc: Scene) {
    *app.navi.scene.lock().unwrap() = Some(sc);
    restart(app);
}

fn worker(app: Arc<App>) {
    let n = &app.navi;
    loop {
        let gen = {
            let mut j = n.job.lock().unwrap();
            while j.is_none() {
                j = n.cv.wait(j).unwrap();
            }
            j.take().unwrap()
        };
        let sc = scene(&app);
        let w = n.width.load(Ordering::Relaxed).clamp(64, 2048);
        let hiq = n.hiq.load(Ordering::Relaxed) != 0;
        // a coarse pass, then the full navigator size; simple lighting
        // (no shadows, reflections, volumetric light or DOF), as MB3D's
        // navigator renders with simpler functions
        for (i, pw) in [(w / 4).max(32), w].into_iter().enumerate() {
            if n.gen.load(Ordering::SeqCst) != gen {
                break;
            }
            let mut ps = preview_scene(&sc, pw, true);
            ps.dof = None;
            if hiq {
                ps.z_step_div *= 0.5;
            }
            {
                let mut o = n.out.lock().unwrap();
                o.rendering = true;
                o.stage = if i == 0 { "coarse".into() } else { format!("{}x{}", ps.width, ps.height) };
                o.error.clear();
            }
            n.progress.store(0, Ordering::Relaxed);
            let cancel = || n.gen.load(Ordering::SeqCst) != gen;
            let progress = |d: usize, t: usize| n.progress.store((d * 1000 / t.max(1)) as u32, Ordering::Relaxed);
            let t0 = std::time::Instant::now();
            match crate::render::calculate_cancellable(&ps, &progress, &cancel) {
                Ok((p, g)) => {
                    if cancel() {
                        break;
                    }
                    let rgb = crate::render::paint(&ps, &p, &g);
                    let (pw, ph) = (ps.width as usize, ps.height as usize);
                    let png = Arc::new(crate::png::encode_rgb(pw, ph, &rgb));
                    *n.pick.lock().unwrap() = Some(Pick { scene: ps.clone(), params: p, gbuf: Arc::new(g), w: pw, h: ph });
                    let mut o = n.out.lock().unwrap();
                    o.png = png;
                    o.img_ver += 1;
                    o.w = pw;
                    o.h = ph;
                    if i == 1 {
                        o.stage = format!("{}x{} — {:.2} s", pw, ph, t0.elapsed().as_secs_f64());
                    }
                }
                Err(e) => {
                    let mut o = n.out.lock().unwrap();
                    o.rendering = false;
                    if e != "cancelled" {
                        o.error = e;
                    }
                    break;
                }
            }
        }
        if n.gen.load(Ordering::SeqCst) == gen {
            n.out.lock().unwrap().rendering = false;
        }
    }
}

pub(super) fn status_json(app: &Arc<App>) -> String {
    ensure_worker(app);
    let n = &app.navi;
    let o = n.out.lock().unwrap();
    format!(
        "{{\"img_ver\":{},\"w\":{},\"h\":{},\"rendering\":{},\"progress\":{},\"stage\":{},\"error\":{},\"width\":{},\"hiq\":{}}}",
        o.img_ver,
        o.w,
        o.h,
        o.rendering,
        n.progress.load(Ordering::Relaxed) as f64 / 10.0,
        json_str(&o.stage),
        json_str(&o.error),
        n.width.load(Ordering::Relaxed),
        n.hiq.load(Ordering::Relaxed) != 0
    )
}

pub(super) fn state_json(app: &Arc<App>) -> String {
    let first = app.navi.out.lock().unwrap().img_ver == 0 && !app.navi.out.lock().unwrap().rendering;
    let sc = scene(app);
    if first {
        restart(app);
    }
    format!("{{\"text\":{},\"status\":{}}}", json_str(&sc.to_text()), status_json(app))
}

pub(super) fn image(app: &App) -> Arc<Vec<u8>> {
    app.navi.out.lock().unwrap().png.clone()
}

pub(super) fn navigate(app: &Arc<App>, q: &HashMap<String, String>) -> Result<(), String> {
    let mut sc = scene(app);
    nav_ops(&mut sc, q, &|u, v| pick_in(&app.navi.pick, u, v))?;
    set_scene(app, sc);
    Ok(())
}

pub(super) fn set_text(app: &Arc<App>, text: &str) -> Result<(), String> {
    set_scene(app, Scene::parse(text)?);
    Ok(())
}

pub(super) fn settings(app: &Arc<App>, f: &HashMap<String, String>) {
    if let Some(w) = f.get("width").and_then(|v| v.parse::<u32>().ok()) {
        app.navi.width.store(w.clamp(64, 2048), Ordering::Relaxed);
    }
    if let Some(h) = f.get("hiq") {
        app.navi.hiq.store((h == "true") as u32, Ordering::Relaxed);
    }
    restart(app);
}

/// The editor's scene with the navigator's view, julia values, formulas and
/// 4D rotation ("View to main"; also the scene of a navigator keyframe).
pub(super) fn view_scene(app: &App) -> Scene {
    let nv = scene(app);
    let mut sc = app.scene.lock().unwrap().scene.clone();
    copy_view(&nv, &mut sc);
    sc.julia = nv.julia;
    sc.julia_c = nv.julia_c;
    sc.formulas = nv.formulas.clone();
    sc.rot_4d = nv.rot_4d;
    sc
}

fn copy_view(from: &Scene, to: &mut Scene) {
    to.mid = from.mid;
    to.z_start = from.z_start;
    to.z_end = from.z_end;
    to.vgrads = from.vgrads;
    to.zoom = from.zoom;
    to.fov_y = from.fov_y;
    to.optic = from.optic;
}

/// "Parameter", "Light", "Formula": take values of the editor.
pub(super) fn from_main(app: &Arc<App>, what: &str) {
    let main = app.scene.lock().unwrap().scene.clone();
    let mut sc = scene(app);
    match what {
        "light" => sc.lighting = main.lighting.clone(),
        "formula" => {
            sc.formulas = main.formulas.clone();
            sc.repeat_from = main.repeat_from;
            sc.decomb = main.decomb;
            sc.interpolation = main.interpolation;
            sc.iterations = main.iterations;
            sc.rstop = main.rstop;
        }
        "julia" => {
            sc.julia = main.julia;
            sc.julia_c = main.julia_c;
        }
        "rot4d" => sc.rot_4d = main.rot_4d,
        "misc" => {
            sc.iterations = main.iterations;
            sc.rstop = main.rstop;
            sc.de_stop = main.de_stop;
            sc.dfog_on_it = main.dfog_on_it;
            sc.decomb = main.decomb;
        }
        _ => sc = main,
    }
    set_scene(app, sc);
}

/// "View to main" and the "Send values" buttons: put values into the editor.
pub(super) fn to_main(app: &Arc<App>, what: &str) {
    let nv = scene(app);
    let mut sc = app.scene.lock().unwrap().scene.clone();
    match what {
        "julia" => {
            sc.julia = nv.julia;
            sc.julia_c = nv.julia_c;
        }
        "formula" => sc.formulas = nv.formulas.clone(),
        "rot4d" => sc.rot_4d = nv.rot_4d,
        "misc" => {
            sc.iterations = nv.iterations;
            sc.rstop = nv.rstop;
            sc.de_stop = nv.de_stop;
            sc.dfog_on_it = nv.dfog_on_it;
            sc.decomb = nv.decomb;
        }
        _ => sc = view_scene(app),
    }
    app.set_scene(sc);
}
