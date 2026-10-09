//! Browser GUI: `mb3d gui` starts a small HTTP server (std only) that serves
//! a single-page editor and renders progressive previews in the background.
//!
//! The page edits the scene as `.m3s` text (the same keys as the command
//! line), navigates the camera with the mouse and keyboard (moves scaled by
//! the distance estimate at the camera, like MB3D's navigator) and renders
//! the final image.  All state lives in the server process; the page polls
//! `/api/status` and reloads the preview image when a new pass is ready.

use crate::calc::CalcParams;
use crate::gbuffer::SiLight;
use crate::math::{dot, normalize, Mat3, Vec3};
use crate::scene::Scene;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

mod anim;
mod tools;

const INDEX_HTML: &str = include_str!("gui/index.html");

#[derive(Clone, Copy, PartialEq)]
enum Job {
    Preview { gen: u64, view_w: u32 },
    Final { gen: u64, aa: u32 },
}

struct SceneState {
    scene: Scene,
    title: String,
    notes: Vec<String>,
}

/// The last calculated preview pass (for picking points in the image).
struct Pick {
    scene: Scene,
    params: CalcParams,
    gbuf: Vec<SiLight>,
    w: usize,
    h: usize,
}

#[derive(Default)]
struct Output {
    png: Arc<Vec<u8>>,
    img_ver: u64,
    w: usize,
    h: usize,
    stage: String,
    error: String,
    seconds: f64,
    final_png: Option<Arc<Vec<u8>>>,
    final_ver: u64,
    rendering: bool,
}

struct App {
    scene: Mutex<SceneState>,
    /// bumped on every scene change; running jobs of older generations stop
    gen: AtomicU64,
    job: Mutex<Option<Job>>,
    job_cv: Condvar,
    out: Mutex<Output>,
    pick: Mutex<Option<Pick>>,
    /// progress of the running pass in 1/1000
    progress: AtomicU32,
    view_w: AtomicU32,
    /// the animation (keyframes, previews, frame rendering)
    anim: anim::AnimState,
    /// MutaGen and the voxel / mesh export
    tools: tools::ToolsState,
}

impl App {
    fn set_scene(&self, sc: Scene) {
        self.scene.lock().unwrap().scene = sc;
        self.restart();
    }

    fn restart(&self) {
        let gen = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        let view_w = self.view_w.load(Ordering::Relaxed);
        *self.job.lock().unwrap() = Some(Job::Preview { gen, view_w });
        self.job_cv.notify_all();
    }
}

/// `mb3d gui [--port N] [--host ADDR] [--formulas DIR] [--maps DIR] [FILE]`
pub fn run(args: &[String]) -> Result<(), String> {
    let mut port: u16 = 8080;
    let mut host = "127.0.0.1".to_string();
    let mut file: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = || it.next().cloned().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--port" => port = val()?.parse().map_err(|_| "bad --port")?,
            "--host" => host = val()?,
            "--formulas" => crate::formulas::add_formula_dir(val()?.into()),
            "--maps" => crate::maps::add_map_dir(val()?.into()),
            "-h" | "--help" => {
                println!("mb3d gui [--port 8080] [--host 127.0.0.1] [--formulas DIR] [--maps DIR] [FILE]");
                return Ok(());
            }
            s if s.starts_with('-') => return Err(format!("unknown gui option {s}")),
            s => file = Some(s.to_string()),
        }
    }
    let mut st = SceneState { scene: Scene::preset("Integer Power")?, title: "untitled".into(), notes: Vec::new() };
    if let Some(f) = &file {
        let data = std::fs::read(f).map_err(|e| format!("{f}: {e}"))?;
        st = load_bytes(f, data)?;
    }
    let app = Arc::new(App {
        scene: Mutex::new(st),
        gen: AtomicU64::new(0),
        job: Mutex::new(None),
        job_cv: Condvar::new(),
        out: Mutex::new(Output::default()),
        pick: Mutex::new(None),
        progress: AtomicU32::new(0),
        view_w: AtomicU32::new(640),
        anim: anim::AnimState::new(),
        tools: tools::ToolsState::new(),
    });
    let listener = TcpListener::bind((host.as_str(), port)).map_err(|e| format!("cannot listen on {host}:{port}: {e}"))?;
    eprintln!("mb3d gui: open http://{host}:{port}/ in your browser (Ctrl+C to stop)");
    {
        let app = app.clone();
        std::thread::spawn(move || worker(app));
    }
    app.restart();
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let app = app.clone();
        std::thread::spawn(move || {
            let _ = handle(&app, stream);
        });
    }
    Ok(())
}

/// Loads a parameter file or scene from memory (detected by content).
fn load_bytes(name: &str, data: Vec<u8>) -> Result<SceneState, String> {
    let lower = name.to_ascii_lowercase();
    let text = String::from_utf8_lossy(&data);
    let title = std::path::Path::new(name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    if lower.ends_with(".m3p") || lower.ends_with(".m3i") || text.contains("Mandelbulb3Dv") {
        let (raw, t) = crate::m3p::raw_from_bytes(data, lower.ends_with(".m3i"))?;
        let m = crate::m3p::parse(&raw)?;
        Ok(SceneState { scene: m.scene, title: t.filter(|t| !t.is_empty()).unwrap_or(title), notes: m.warnings })
    } else {
        Ok(SceneState { scene: Scene::parse(&text)?, title, notes: Vec::new() })
    }
}

// ---------------------------------------------------------------------------
// render worker
// ---------------------------------------------------------------------------

/// A preview copy: smaller image with the DE stop kept in pixels of the
/// preview (coarse and fast, like MB3D's navigator).
fn preview_scene(sc: &Scene, w: u32, light: bool) -> Scene {
    let mut s = sc.clone();
    s.tiling = None;
    s.calc_rect = None;
    let f = w as f64 / s.width.max(1) as f64;
    s.width = (w as i32).max(16);
    s.height = ((sc.height as f64 * f).round() as i32).max(12);
    if let Some(d) = s.dof.as_mut() {
        d.clip_r = (d.clip_r * f as f32).max(0.1);
    }
    if light {
        s.shadows = None;
        s.vol_light = None;
        if s.deao.is_some() {
            s.deao = None;
        }
        if let Some(a) = s.ao.as_mut() {
            a.random = 0;
        }
    }
    s
}

fn worker(app: Arc<App>) {
    loop {
        let job = {
            let mut j = app.job.lock().unwrap();
            while j.is_none() {
                j = app.job_cv.wait(j).unwrap();
            }
            j.take().unwrap()
        };
        let sc = app.scene.lock().unwrap().scene.clone();
        match job {
            Job::Preview { gen, view_w } => {
                let full = view_w.min(sc.width.max(16) as u32).max(16);
                let mut widths: Vec<u32> = [full / 8, full / 4, full / 2].into_iter().filter(|&w| w >= 40).collect();
                widths.push(full);
                let n = widths.len();
                for (i, w) in widths.into_iter().enumerate() {
                    if app.gen.load(Ordering::SeqCst) != gen {
                        break;
                    }
                    let last = i + 1 == n;
                    let ps = preview_scene(&sc, w, !last && n > 1);
                    let stage = if last { format!("preview {}x{}", ps.width, ps.height) } else { format!("pass {} of {}", i + 1, n) };
                    if !render_pass(&app, gen, &ps, &stage, 1, false) {
                        break;
                    }
                }
            }
            Job::Final { gen, aa } => {
                let mut s = sc.clone();
                if aa > 1 {
                    s.scale_image(aa as f64);
                }
                let stage = format!("final {}x{}{}", sc.width, sc.height, if aa > 1 { format!(", anti-aliasing {aa}x") } else { String::new() });
                render_pass(&app, gen, &s, &stage, aa as usize, true);
            }
        }
    }
}

/// Renders one pass; false when cancelled or failed.
fn render_pass(app: &App, gen: u64, sc: &Scene, stage: &str, aa: usize, final_img: bool) -> bool {
    {
        let mut o = app.out.lock().unwrap();
        o.rendering = true;
        o.stage = stage.to_string();
        o.error.clear();
    }
    app.progress.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let cancel = || app.gen.load(Ordering::SeqCst) != gen;
    let progress = |d: usize, t: usize| app.progress.store((d * 1000 / t.max(1)) as u32, Ordering::Relaxed);
    let res = if sc.tiling.is_some() && final_img {
        crate::render::render_tiled(sc, false, &progress, &|_, _, _| {}).map(|r| (r.rgb, r.width, r.height, None))
    } else {
        crate::render::calculate_cancellable(sc, &progress, &cancel).map(|(p, g)| {
            let rgb = crate::render::paint(sc, &p, &g);
            (rgb, sc.width as usize, sc.height as usize, Some((p, g)))
        })
    };
    let mut o = app.out.lock().unwrap();
    o.rendering = false;
    match res {
        Ok((rgb, w, h, pg)) => {
            if cancel() && !final_img {
                return false;
            }
            let (rgb, w, h) = if aa > 1 { crate::render::downsample(&rgb, w, h, aa) } else { (rgb, w, h) };
            let png = Arc::new(crate::png::encode_rgb(w, h, &rgb));
            o.png = png.clone();
            o.img_ver += 1;
            o.w = w;
            o.h = h;
            o.seconds = t0.elapsed().as_secs_f64();
            o.stage = format!("{stage} — {:.1} s", o.seconds);
            if final_img {
                o.final_png = Some(png);
                o.final_ver += 1;
            }
            drop(o);
            if let (Some((p, g)), false) = (pg, final_img) {
                *app.pick.lock().unwrap() = Some(Pick { scene: sc.clone(), params: p, gbuf: g, w, h });
            }
            true
        }
        Err(e) => {
            if e != "cancelled" {
                o.error = e;
                o.stage = "error".into();
            } else {
                o.stage = "cancelled".into();
            }
            false
        }
    }
}

// ---------------------------------------------------------------------------
// navigation
// ---------------------------------------------------------------------------

fn cross(a: &Vec3, b: &Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn unit_rows(m: &Mat3) -> Mat3 {
    [normalize(m[0]), normalize(m[1]), normalize(m[2])]
}

/// Camera position: `mid + (z_start - mid_z) * view direction`.
fn camera_pos(sc: &Scene) -> Vec3 {
    let vz = normalize(sc.vgrads[2]);
    let d = sc.z_start - sc.mid[2];
    [sc.mid[0] + vz[0] * d, sc.mid[1] + vz[1] * d, sc.mid[2] + vz[2] * d]
}

/// Moves the camera (and the scene middle) by `v`; the start and end planes
/// follow the middle's z (navigator: "transform also dZstart").
fn move_by(sc: &mut Scene, v: Vec3) {
    for (k, vk) in v.iter().enumerate() {
        sc.mid[k] += vk;
    }
    sc.z_start += v[2];
    sc.z_end += v[2];
}

/// Puts the middle onto the camera so that rotations turn around the
/// camera (navigator: "step back so that midpoint becomes startpoint").
fn mid_to_camera(sc: &mut Scene) {
    let vz = normalize(sc.vgrads[2]);
    let d = sc.z_start - sc.mid[2];
    for k in 0..3 {
        sc.mid[k] += vz[k] * d;
    }
    sc.z_end = sc.z_end - sc.z_start + sc.mid[2];
    sc.z_start = sc.mid[2];
}

/// Absolute distance estimate at the camera, None if inside / invalid.
fn camera_de(sc: &Scene) -> Option<f64> {
    let p = CalcParams::new(sc).ok()?;
    let (d, its) = crate::calc::de_at(&p, camera_pos(sc));
    (d.is_finite() && d > 0.0 && its < sc.iterations).then_some(d)
}

/// Rotation of the camera axes: rows `a` and `b` of the view matrix turn by
/// `deg` towards each other.
fn rotate_rows(sc: &mut Scene, a: usize, b: usize, deg: f64) {
    mid_to_camera(sc);
    let m = sc.vgrads;
    let (s, c) = deg.to_radians().sin_cos();
    let mut r = m;
    for k in 0..3 {
        r[a][k] = m[a][k] * c + m[b][k] * s;
        r[b][k] = m[b][k] * c - m[a][k] * s;
    }
    sc.vgrads = r;
}

/// Turns the camera to look along `dir` (keeps the roll as far as possible).
fn look_along(sc: &mut Scene, dir: Vec3) {
    mid_to_camera(sc);
    let len = |v: &Vec3| dot(v, v).sqrt();
    let m = sc.vgrads;
    let (lx, ly, lz) = (len(&m[0]), len(&m[1]), len(&m[2]));
    let u = unit_rows(&m);
    let z = normalize(dir);
    let mut x = u[0];
    let d = dot(&x, &z);
    for k in 0..3 {
        x[k] -= z[k] * d;
    }
    let x = normalize(x);
    // keep the handedness of the original matrix
    let s = if dot(&cross(&u[2], &u[0]), &u[1]) < 0.0 { -1.0 } else { 1.0 };
    let y = cross(&z, &x).map(|v| v * s);
    sc.vgrads = [x.map(|v| v * lx), y.map(|v| v * ly), z.map(|v| v * lz)];
}

/// Pixel under the normalised image coordinates of the last preview.
fn pick_at(app: &App, u: f64, v: f64) -> Option<(Option<Vec3>, Vec3)> {
    let pk = app.pick.lock().unwrap();
    let pk = pk.as_ref()?;
    let x = ((u * pk.w as f64) as i64).clamp(0, pk.w as i64 - 1) as i32;
    let y = ((v * pk.h as f64) as i64).clamp(0, pk.h as i64 - 1) as i32;
    let cam = crate::render::paint_camera(&pk.scene, &pk.params);
    let si = &pk.gbuf[y as usize * pk.w + x as usize];
    let pos = cam.object_pos(si, x, y).map(|p| [p[0] + pk.scene.mid[0], p[1] + pk.scene.mid[1], p[2] + pk.scene.mid[2]]);
    Some((pos, cam.pixel_dir(x as f32, y as f32)))
}

fn navigate(app: &App, q: &HashMap<String, String>) -> Result<(), String> {
    let num = |k: &str, d: f64| q.get(k).and_then(|v| v.parse::<f64>().ok()).unwrap_or(d);
    let op = q.get("op").map(String::as_str).unwrap_or("");
    let mut sc = app.scene.lock().unwrap().scene.clone();
    let step_de = || camera_de(&sc).unwrap_or(sc.step_width() * sc.width as f64 * 0.05);
    match op {
        "move" => {
            let axis = (num("axis", 2.0) as usize).min(2);
            let d = num("amount", 0.5) * step_de();
            let dir = normalize(sc.vgrads[axis]);
            move_by(&mut sc, dir.map(|v| v * d));
        }
        "rotate" => {
            let deg = num("deg", 5.0);
            match num("axis", 1.0) as i32 {
                0 => rotate_rows(&mut sc, 1, 2, deg), // pitch
                1 => rotate_rows(&mut sc, 0, 2, -deg), // yaw (positive = turn right)
                _ => rotate_rows(&mut sc, 0, 1, deg), // roll
            }
        }
        "zoom" => sc.zoom = (sc.zoom * num("factor", 1.25)).clamp(1e-12, 1e12),
        "fly" | "look" => {
            let (pos, dir) = pick_at(app, num("u", 0.5), num("v", 0.5)).ok_or("no preview to pick from yet")?;
            if op == "look" {
                look_along(&mut sc, dir);
            } else {
                let cam = camera_pos(&sc);
                let f = num("amount", 0.5);
                let v = match pos {
                    Some(p) => [(p[0] - cam[0]) * f, (p[1] - cam[1]) * f, (p[2] - cam[2]) * f],
                    None => dir.map(|c| c * step_de() * f),
                };
                move_by(&mut sc, v);
            }
        }
        "focus" => {
            // depth of field focus on the picked point (fraction of the image width)
            let (pos, _) = pick_at(app, num("u", 0.5), num("v", 0.5)).ok_or("no preview yet")?;
            let p = pos.ok_or("background picked")?;
            let cam = camera_pos(&sc);
            let vz = normalize(sc.vgrads[2]);
            let z = (p[0] - cam[0]) * vz[0] + (p[1] - cam[1]) * vz[1] + (p[2] - cam[2]) * vz[2];
            let f = z / (sc.step_width() * sc.width as f64);
            sc = sc.apply(&format!("dof_focus = {f}"))?;
            if sc.dof.is_none() {
                sc = sc.apply("dof = sorted")?;
            }
        }
        _ => return Err(format!("unknown navigation '{op}'")),
    }
    app.set_scene(sc);
    Ok(())
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

struct Request {
    method: String,
    path: String,
    query: HashMap<String, String>,
    body: Vec<u8>,
}

fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() && hex(b[i + 1]).is_some() && hex(b[i + 2]).is_some() => {
                out.push((hex(b[i + 1]).unwrap() * 16 + hex(b[i + 2]).unwrap()) as u8);
                i += 2;
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_form(s: &str) -> HashMap<String, String> {
    s.split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (url_decode(k), url_decode(v))
        })
        .collect()
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<Request> {
    let mut r = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    let mut len = 0usize;
    loop {
        let mut h = String::new();
        if r.read_line(&mut h)? == 0 || h == "\r\n" || h == "\n" {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                len = v.trim().parse().unwrap_or(0);
            }
        }
    }
    if len > 64 << 20 {
        return Err(std::io::Error::other("request too large"));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    let (path, q) = target.split_once('?').unwrap_or((&target, ""));
    Ok(Request { method, path: path.to_string(), query: parse_form(q), body })
}

fn respond(stream: &mut TcpStream, status: &str, ctype: &str, body: &[u8], extra: &str) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n{extra}\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn json_list(v: &[String]) -> String {
    format!("[{}]", v.iter().map(|s| json_str(s)).collect::<Vec<_>>().join(","))
}

fn state_json(app: &App) -> String {
    let st = app.scene.lock().unwrap();
    format!(
        "{{\"gen\":{},\"title\":{},\"text\":{},\"notes\":{},\"view_w\":{}}}",
        app.gen.load(Ordering::SeqCst),
        json_str(&st.title),
        json_str(&st.scene.to_text()),
        json_list(&st.notes),
        app.view_w.load(Ordering::Relaxed)
    )
}

fn status_json(app: &App) -> String {
    let o = app.out.lock().unwrap();
    format!(
        "{{\"gen\":{},\"img_ver\":{},\"w\":{},\"h\":{},\"rendering\":{},\"progress\":{},\"stage\":{},\"error\":{},\"final_ver\":{}}}",
        app.gen.load(Ordering::SeqCst),
        o.img_ver,
        o.w,
        o.h,
        o.rendering,
        app.progress.load(Ordering::Relaxed) as f64 / 10.0,
        json_str(&o.stage),
        json_str(&o.error),
        o.final_ver
    )
}

fn safe_name(s: &str) -> String {
    let n: String = s.chars().map(|c| if c.is_alphanumeric() || " -_.()".contains(c) { c } else { '_' }).collect();
    if n.trim().is_empty() {
        "mb3d".into()
    } else {
        n.trim().to_string()
    }
}

fn handle(app: &Arc<App>, mut stream: TcpStream) -> std::io::Result<()> {
    let req = read_request(&mut stream)?;
    let ok_json = |s: &mut TcpStream, j: String| respond(s, "200 OK", "application/json", j.as_bytes(), "");
    let err_json = |s: &mut TcpStream, e: String| {
        respond(s, "400 Bad Request", "application/json", format!("{{\"error\":{}}}", json_str(&e)).as_bytes(), "")
    };
    let form = || parse_form(&String::from_utf8_lossy(&req.body));
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") | ("GET", "/index.html") => {
            // MB3D_GUI_HTML=path serves the page from a file (development)
            let page = std::env::var("MB3D_GUI_HTML").ok().and_then(|p| std::fs::read(p).ok());
            respond(&mut stream, "200 OK", "text/html; charset=utf-8", page.as_deref().unwrap_or(INDEX_HTML.as_bytes()), "")
        }
        ("GET", "/api/state") => ok_json(&mut stream, state_json(app)),
        ("GET", "/api/status") => ok_json(&mut stream, status_json(app)),
        ("GET", "/api/image") => {
            let png = app.out.lock().unwrap().png.clone();
            respond(&mut stream, "200 OK", "image/png", &png, "")
        }
        ("GET", "/api/final.png") => {
            let (png, title) = (app.out.lock().unwrap().final_png.clone(), app.scene.lock().unwrap().title.clone());
            match png {
                Some(p) => respond(
                    &mut stream,
                    "200 OK",
                    "image/png",
                    &p,
                    &format!("Content-Disposition: attachment; filename=\"{}.png\"\r\n", safe_name(&title)),
                ),
                None => respond(&mut stream, "404 Not Found", "text/plain", b"no final image", ""),
            }
        }
        ("POST", "/api/scene") => {
            let text = String::from_utf8_lossy(&req.body).into_owned();
            match Scene::parse(&text) {
                Ok(sc) => {
                    app.set_scene(sc);
                    ok_json(&mut stream, state_json(app))
                }
                Err(e) => err_json(&mut stream, e),
            }
        }
        ("POST", "/api/nav") => match navigate(app, &form()) {
            Ok(()) => ok_json(&mut stream, state_json(app)),
            Err(e) => err_json(&mut stream, e),
        },
        ("POST", "/api/view") => {
            let w = form().get("w").and_then(|v| v.parse::<u32>().ok()).unwrap_or(640).clamp(64, 4096);
            app.view_w.store(w, Ordering::Relaxed);
            app.restart();
            ok_json(&mut stream, state_json(app))
        }
        ("POST", "/api/render") => {
            let aa = form().get("aa").and_then(|v| v.parse::<u32>().ok()).unwrap_or(1).clamp(1, 4);
            let gen = app.gen.fetch_add(1, Ordering::SeqCst) + 1;
            *app.job.lock().unwrap() = Some(Job::Final { gen, aa });
            app.job_cv.notify_all();
            ok_json(&mut stream, status_json(app))
        }
        ("POST", "/api/cancel") => {
            app.gen.fetch_add(1, Ordering::SeqCst);
            ok_json(&mut stream, status_json(app))
        }
        ("POST", "/api/refresh") => {
            app.restart();
            ok_json(&mut stream, status_json(app))
        }
        ("POST", "/api/open") => {
            let name = req.query.get("name").cloned().unwrap_or_else(|| "pasted.txt".into());
            match load_bytes(&name, req.body.clone()) {
                Ok(st) => {
                    {
                        let mut s = app.scene.lock().unwrap();
                        *s = st;
                    }
                    app.out.lock().unwrap().final_png = None;
                    app.restart();
                    ok_json(&mut stream, state_json(app))
                }
                Err(e) => err_json(&mut stream, e),
            }
        }
        ("POST", "/api/preset") => {
            let name = form().get("name").cloned().unwrap_or_default();
            match Scene::preset(&name) {
                Ok(sc) => {
                    {
                        let mut s = app.scene.lock().unwrap();
                        *s = SceneState { scene: sc, title: name.clone(), notes: Vec::new() };
                    }
                    app.restart();
                    ok_json(&mut stream, state_json(app))
                }
                Err(e) => err_json(&mut stream, e),
            }
        }
        ("GET", "/api/formulas") => {
            let builtin: Vec<String> = crate::formulas::Formula::all_names().iter().map(|s| s.to_string()).collect();
            let custom = crate::formulas::list_custom();
            ok_json(&mut stream, format!("{{\"builtin\":{},\"custom\":{}}}", json_list(&builtin), json_list(&custom)))
        }
        ("GET", "/api/save") => {
            let fmt = req.query.get("fmt").cloned().unwrap_or_else(|| "m3s".into());
            let (sc, title) = {
                let s = app.scene.lock().unwrap();
                (s.scene.clone(), s.title.clone())
            };
            let name = safe_name(&title);
            let (body, ext, ctype) = match fmt.as_str() {
                "m3p" => (crate::m3p::write(&sc), "m3p", "application/octet-stream"),
                "txt" => (crate::m3p::raw_to_text(&crate::m3p::write(&sc), &name).into_bytes(), "txt", "text/plain; charset=utf-8"),
                _ => (sc.to_text().into_bytes(), "m3s", "text/plain; charset=utf-8"),
            };
            respond(
                &mut stream,
                "200 OK",
                ctype,
                &body,
                &format!("Content-Disposition: attachment; filename=\"{name}.{ext}\"\r\n"),
            )
        }
        // ---- animation
        ("GET", "/api/anim") => ok_json(&mut stream, anim::anim_json(app)),
        ("POST", "/api/anim/key") => match anim::key_op(app, &form()) {
            Ok(()) => ok_json(&mut stream, format!("{{\"anim\":{},\"state\":{}}}", anim::anim_json(app), state_json(app))),
            Err(e) => err_json(&mut stream, e),
        },
        ("POST", "/api/anim/set") => match anim::set_settings(app, &form()) {
            Ok(()) => ok_json(&mut stream, anim::anim_json(app)),
            Err(e) => err_json(&mut stream, e),
        },
        ("POST", "/api/anim/preview") => match anim::start_flipbook(app, &form()) {
            Ok(()) => ok_json(&mut stream, anim::anim_json(app)),
            Err(e) => err_json(&mut stream, e),
        },
        ("POST", "/api/anim/render") => match anim::start_render(app, &form()) {
            Ok(()) => ok_json(&mut stream, anim::anim_json(app)),
            Err(e) => err_json(&mut stream, e),
        },
        ("POST", "/api/anim/stop") => {
            anim::stop(app);
            ok_json(&mut stream, anim::anim_json(app))
        }
        ("GET", "/api/anim/frame") => {
            let i = req.query.get("i").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
            match anim::flip_frame(app, i) {
                Some(p) => respond(&mut stream, "200 OK", "image/png", &p, ""),
                None => respond(&mut stream, "404 Not Found", "text/plain", b"not rendered yet", ""),
            }
        }
        ("GET", "/api/anim/thumb") => {
            let id = req.query.get("id").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            match anim::thumb(app, id) {
                Some(p) => respond(&mut stream, "200 OK", "image/png", &p, ""),
                None => respond(&mut stream, "404 Not Found", "text/plain", b"no image yet", ""),
            }
        }
        ("GET", "/api/anim/save") => {
            let fmt = req.query.get("fmt").cloned().unwrap_or_else(|| "m3k".into());
            let (body, ext, name) = anim::save(app, &fmt);
            respond(
                &mut stream,
                "200 OK",
                if ext == "m3k" { "text/plain; charset=utf-8" } else { "application/octet-stream" },
                &body,
                &format!("Content-Disposition: attachment; filename=\"{}.{ext}\"\r\n", safe_name(&name)),
            )
        }
        ("POST", "/api/anim/open") => {
            let name = req.query.get("name").cloned().unwrap_or_else(|| "anim.m3a".into());
            match anim::open(app, &name, req.body.clone()) {
                Ok(notes) => ok_json(&mut stream, format!("{{\"anim\":{},\"notes\":{}}}", anim::anim_json(app), anim::notes_json(&notes))),
                Err(e) => err_json(&mut stream, e),
            }
        }
        // ---- MutaGen
        ("GET", "/api/muta") => ok_json(&mut stream, tools::muta_json(app)),
        ("POST", "/api/muta/start") => match tools::muta_start(app, &form()) {
            Ok(()) => ok_json(&mut stream, tools::muta_json(app)),
            Err(e) => err_json(&mut stream, e),
        },
        ("POST", "/api/muta/use") => {
            let i = form().get("i").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
            match tools::muta_use(app, i) {
                Ok(()) => ok_json(&mut stream, state_json(app)),
                Err(e) => err_json(&mut stream, e),
            }
        }
        ("POST", "/api/muta/nav") => {
            let d = form().get("d").and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
            tools::muta_nav(app, d);
            ok_json(&mut stream, tools::muta_json(app))
        }
        ("POST", "/api/muta/stop") => {
            tools::muta_stop(app);
            ok_json(&mut stream, tools::muta_json(app))
        }
        ("GET", "/api/muta/img") => {
            let g = req.query.get("g").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
            let i = req.query.get("i").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
            match tools::muta_img(app, g, i) {
                Some(p) => respond(&mut stream, "200 OK", "image/png", &p, ""),
                None => respond(&mut stream, "404 Not Found", "text/plain", b"no image", ""),
            }
        }
        // ---- Monte Carlo
        ("GET", "/api/mc") => ok_json(&mut stream, tools::mc_json(app)),
        ("POST", "/api/mc/start") => match tools::mc_start(app, &form()) {
            Ok(()) => ok_json(&mut stream, tools::mc_json(app)),
            Err(e) => err_json(&mut stream, e),
        },
        ("POST", "/api/mc/stop") => {
            tools::mc_stop(app);
            ok_json(&mut stream, tools::mc_json(app))
        }
        ("GET", "/api/mc/img") => match tools::mc_png(app) {
            Some(p) => respond(&mut stream, "200 OK", "image/png", &p, ""),
            None => respond(&mut stream, "404 Not Found", "text/plain", b"no image", ""),
        },
        ("GET", "/api/mc/m3c") => match tools::mc_m3c(app) {
            Some((name, d)) => respond(
                &mut stream,
                "200 OK",
                "application/octet-stream",
                &d,
                &format!("Content-Disposition: attachment; filename=\"{}\"\r\n", safe_name(&name)),
            ),
            None => respond(&mut stream, "404 Not Found", "text/plain", b"nothing rendered", ""),
        },
        ("POST", "/api/mc/open") => {
            let name = req.query.get("name").cloned().unwrap_or_else(|| "mc.m3c".into());
            match tools::mc_open(app, &name, &req.body) {
                Ok(()) => ok_json(&mut stream, state_json(app)),
                Err(e) => err_json(&mut stream, e),
            }
        }
        // ---- voxel / mesh export
        ("GET", "/api/export") => ok_json(&mut stream, tools::export_json(app)),
        ("POST", "/api/export/start") => match tools::export_start(app, &form()) {
            Ok(()) => ok_json(&mut stream, tools::export_json(app)),
            Err(e) => err_json(&mut stream, e),
        },
        ("POST", "/api/export/stop") => {
            tools::export_stop(app);
            ok_json(&mut stream, tools::export_json(app))
        }
        ("GET", "/api/export/voxpreview") => match tools::voxel_preview(app, &req.query) {
            Ok(p) => respond(&mut stream, "200 OK", "image/png", &p, ""),
            Err(e) => respond(&mut stream, "400 Bad Request", "text/plain", e.as_bytes(), ""),
        },
        ("GET", "/api/export/file") => match tools::export_file(app) {
            Some((name, d)) => respond(
                &mut stream,
                "200 OK",
                "application/octet-stream",
                &d,
                &format!("Content-Disposition: attachment; filename=\"{}\"\r\n", safe_name(&name)),
            ),
            None => respond(&mut stream, "404 Not Found", "text/plain", b"no mesh yet", ""),
        },
        ("POST", "/api/title") => {
            app.scene.lock().unwrap().title = String::from_utf8_lossy(&req.body).trim().to_string();
            ok_json(&mut stream, state_json(app))
        }
        _ => respond(&mut stream, "404 Not Found", "text/plain", b"not found", ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_moves_with_middle() {
        let mut sc = Scene::preset("Integer Power").unwrap();
        let c0 = camera_pos(&sc);
        move_by(&mut sc, [0.1, -0.2, 0.3]);
        let c1 = camera_pos(&sc);
        for k in 0..3 {
            assert!((c1[k] - c0[k] - [0.1, -0.2, 0.3][k]).abs() < 1e-12);
        }
        let c = camera_pos(&sc);
        rotate_rows(&mut sc, 0, 2, 20.0);
        let c2 = camera_pos(&sc);
        for k in 0..3 {
            assert!((c2[k] - c[k]).abs() < 1e-9, "rotation must keep the camera in place");
        }
    }

    #[test]
    fn form_decoding() {
        let f = parse_form("op=move&name=Amazing+Box&x=%23ff");
        assert_eq!(f["name"], "Amazing Box");
        assert_eq!(f["x"], "#ff");
    }
}
