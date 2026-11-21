//! The engine for other programs: a small surface over the renderer, meant
//! to stay the same while the modules behind it change.  EXPERIMENTAL: it
//! may still change between versions until it is declared stable.
//!
//! ```no_run
//! let mut f = mb3d::api::Fractal::preset("Integer Power")?;
//! f.set("iterations = 14\nzoom = 1.1")?;
//! f.resize(640, 480);
//! let pic = f.render(&mb3d::api::RenderOptions::default())?;
//! std::fs::write("bulb.png", pic.png()).map_err(|e| e.to_string())?;
//! # Ok::<(), String>(())
//! ```
//!
//! A scene is changed with the `key = value` lines of the scene text
//! ([`Fractal::text`] shows them); `mb3d serve` ([`crate::serve`]) offers
//! the same to programs in other languages.

use crate::scene::Scene;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

/// The version of the engine.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Calculates on the graphics card when the scene allows it, or on the
/// processor only (how a library starts; `mb3d serve` switches it on).
pub fn use_gpu(on: bool) {
    #[cfg(feature = "gpu")]
    crate::gpu::set_enabled(on);
    #[cfg(not(feature = "gpu"))]
    let _ = on;
}

/// To call before the program ends when the graphics card was used: waits
/// until no formula is being compiled for the card any more (a process
/// that ends inside the driver crashes there, and the compile would be lost
/// for the next run).  `MB3D_COMPILE_WAIT_SECS` limits the wait.
pub fn finish() {
    #[cfg(feature = "gpu")]
    crate::gpu::wait_compiles(crate::gpu::compile_wait_limit());
}

/// A further folder with formula files (.m3f).
pub fn add_formula_folder(dir: &Path) {
    crate::formulas::add_formula_dir(dir.to_path_buf());
}

/// A further folder with map pictures.
pub fn add_map_folder(dir: &Path) {
    crate::maps::add_map_dir(dir.to_path_buf());
}

/// The names of all formulas: the built-in ones, then those of the formula
/// folders.
pub fn formula_names() -> Vec<String> {
    let mut v: Vec<String> = crate::formulas::Formula::all_names().iter().map(|n| n.to_string()).collect();
    v.extend(crate::formulas::list_custom());
    v
}

/// The settings of a formula with their default values (the keys of a
/// `[formula]` section).
pub fn formula_settings(name: &str) -> Result<Vec<(String, f64)>, String> {
    Ok(crate::formulas::lookup(name)?.options())
}

/// A fractal scene: formulas, camera, lights, colours, quality.
#[derive(Clone)]
pub struct Fractal {
    scene: Scene,
    notes: Vec<String>,
}

impl Fractal {
    /// A parameter file: `.m3p`, `.m3i`, MB3D text parameters or a scene
    /// text.
    pub fn open(path: &Path) -> Result<Fractal, String> {
        let (scene, notes) = crate::animfile::load_scene(path)?;
        Ok(Fractal { scene, notes })
    }

    /// The contents of a parameter file; `name` tells the kind (its
    /// extension), like a file name.
    pub fn from_bytes(name: &str, data: Vec<u8>) -> Result<Fractal, String> {
        let (scene, notes) = crate::animfile::scene_from_bytes(name, data)?;
        Ok(Fractal { scene, notes })
    }

    /// A scene text (what [`Fractal::text`] gives) or MB3D text parameters.
    pub fn from_text(text: &str) -> Result<Fractal, String> {
        Fractal::from_bytes("scene.m3s", text.as_bytes().to_vec())
    }

    /// A formula by name with a camera that shows it.
    pub fn preset(formula: &str) -> Result<Fractal, String> {
        Ok(Fractal { scene: Scene::preset(formula)?, notes: Vec::new() })
    }

    /// What the loader had to say (settings it could not carry over).
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// The scene as `key = value` lines.
    pub fn text(&self) -> String {
        self.scene.to_text()
    }

    /// Changes settings: `key = value` lines of global keys (see
    /// [`Fractal::text`]).  The scene stays as it was when a line is wrong.
    pub fn set(&mut self, changes: &str) -> Result<(), String> {
        self.scene = self.scene.clone().apply(changes)?;
        Ok(())
    }

    /// Picture size in pixels.
    pub fn size(&self) -> (usize, usize) {
        (self.scene.width.max(1) as usize, self.scene.height.max(1) as usize)
    }

    /// Another picture size; the look of the picture is kept (the detail
    /// follows the width).
    pub fn resize(&mut self, width: usize, height: usize) {
        let (w, h) = (width.clamp(16, 32768), height.clamp(16, 32768));
        self.scene.scale_image(w as f64 / self.scene.width.max(1) as f64);
        self.scene.width = w as i32;
        self.scene.height = h as i32;
    }

    /// The view as the angles of the `rotation` key, in degrees (None for
    /// a view they cannot give back).
    pub fn rotation(&self) -> Option<[f64; 3]> {
        crate::mcp::view_angles(&self.scene)
    }

    /// The formulas in use: name and iterations.
    pub fn formulas(&self) -> Vec<(String, i32)> {
        self.scene.formulas.iter().filter(|f| f.iterations > 0).map(|f| (f.formula.name(), f.iterations)).collect()
    }

    /// The formulas in use with their place in the scene and their own
    /// values.
    pub fn formula_list(&self) -> Vec<FormulaInfo> {
        let used = self.scene.formulas.iter().enumerate().filter(|(_, f)| f.iterations > 0);
        used.map(|(slot, f)| FormulaInfo { slot, name: f.formula.name(), iterations: f.iterations, settings: f.formula.options() }).collect()
    }

    /// Changes a formula's own values (by name, as [`Fractal::formula_list`]
    /// gives them) and, if given, its iterations.  `slot` is its place in
    /// the scene.  The scene stays as it was when a name is wrong.
    pub fn set_formula(&mut self, slot: usize, settings: &[(&str, f64)], iterations: Option<i32>) -> Result<(), String> {
        let entry = self.scene.formulas.get(slot).ok_or_else(|| format!("the scene has no formula at place {slot}"))?;
        let mut changed = entry.clone();
        for (key, value) in settings {
            changed.formula.set_option(key, *value)?;
        }
        if let Some(n) = iterations {
            if n < 0 {
                return Err(format!("iterations = {n}: not below 0"));
            }
            changed.iterations = n;
        }
        self.scene.formulas[slot] = changed;
        Ok(())
    }

    /// The scene as a Mandelbulb3D parameter file (.m3p).
    pub fn m3p(&self) -> Vec<u8> {
        crate::m3p::write(&self.scene)
    }

    /// The parameters as Mandelbulb3D text (`Mandelbulb3Dv18{...}`), as its
    /// "To Clipboard" gives them for forums and other programs.
    pub fn mb3d_text(&self, title: &str) -> String {
        crate::m3p::raw_to_text(&crate::m3p::write(&self.scene), title)
    }

    /// MutaGen's generation of 15 variations (the first is this scene,
    /// the others its children and grandchildren: formulas, their values,
    /// Julia, iterations changed), with the tree labels ("1", "1.1", ...).
    /// `seed` makes it repeatable; progress reports each member done.
    pub fn variations(&self, seed: u64, opt: &RenderOptions) -> Result<Vec<(String, Fractal)>, String> {
        let cfg = crate::mutagen::MutationConfig::default();
        let mut rng = crate::mutagen::Rng::new(seed);
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let total = crate::mutagen::TREE.len();
        let mut report = |i: usize, _: &crate::mutagen::Member| {
            if let Some(p) = opt.progress {
                p("Variations", i, total);
            }
        };
        let cancel = || opt.cancel.is_some_and(|c| c());
        let members = crate::mutagen::generation(&cfg, &self.scene, &mut rng, threads, &mut report, &cancel)?;
        Ok(members
            .into_iter()
            .zip(crate::mutagen::TREE.iter())
            .map(|(m, (label, _))| (label.to_string(), Fractal { scene: m.scene, notes: Vec::new() }))
            .collect())
    }

    /// What the picture shows at pixel (`x`, `y`): the surface point and
    /// its depth as `dof_focus` takes it (the Post-process window's "Get
    /// Z" on a clicked point); None on the background.
    pub fn pick(&self, x: usize, y: usize) -> Option<([f64; 3], f64)> {
        let sc = &self.scene;
        if x >= sc.width.max(0) as usize || y >= sc.height.max(0) as usize {
            return None;
        }
        let p = crate::calc::CalcParams::new(sc).ok()?;
        let mut m = crate::calc::Marcher::new(&p, 0);
        m.seed = crate::calc::pixel_seed(y as i64, x as i64);
        let si = m.march_pixel(x as i32, y as i32);
        let cam = crate::render::paint_camera(sc, &p);
        let q = cam.object_pos(&si, x as i32, y as i32)?;
        let pos = [q[0] + sc.mid[0], q[1] + sc.mid[1], q[2] + sc.mid[2]];
        let vz = crate::math::normalize(sc.vgrads[2]);
        let d = sc.z_start - sc.mid[2];
        let cam_pos = [sc.mid[0] + vz[0] * d, sc.mid[1] + vz[1] * d, sc.mid[2] + vz[2] * d];
        let z = (pos[0] - cam_pos[0]) * vz[0] + (pos[1] - cam_pos[1]) * vz[1] + (pos[2] - cam_pos[2]) * vz[2];
        Some((pos, z / (sc.step_width() * sc.width as f64)))
    }

    /// Calculates the picture, with the scene's shadows, ambient occlusion
    /// and other effects.  Stopped by `cancel` it returns `Err("cancelled")`.
    pub fn render(&self, opt: &RenderOptions) -> Result<Picture, String> {
        let t0 = Instant::now();
        let stage_now = Mutex::new(String::new());
        let progress = |done: usize, total: usize| {
            if let Some(p) = opt.progress {
                let name = stage_now.lock().map(|s| s.clone()).unwrap_or_default();
                p(&name, done, total);
            }
        };
        let stage = |name: &str| {
            crate::render::no_stage(name);
            if let Ok(mut s) = stage_now.lock() {
                *s = name.to_string();
            }
        };
        let cancel = || opt.cancel.is_some_and(|c| c());
        let (p, g) = crate::render::calculate_with(&self.scene, &progress, &stage, &cancel, 2, None, None)?;
        if cancel() {
            return Err("cancelled".into());
        }
        let rgb = crate::render::paint_cancellable(&self.scene, &p, &g, &cancel)?;
        let (width, height) = (p.rect[2] as usize, p.rect[3] as usize);
        let (depth, _, _) = crate::frames::depth_of(&g, width, height, 1);
        #[cfg(feature = "gpu")]
        let device = if crate::gpu::enabled() { crate::gpu::last_status() } else { "CPU".to_string() };
        #[cfg(not(feature = "gpu"))]
        let device = "CPU".to_string();
        Ok(Picture { width, height, rgb, depth, seconds: t0.elapsed().as_secs_f64(), device })
    }
}

/// A Monte Carlo render (the M.C. window): soft light, reflections and
/// depth of field by random rays; each pass makes the picture less noisy.
pub struct MonteCarlo {
    scene: Scene,
    img: crate::mc::McImage,
}

impl Fractal {
    /// A Monte Carlo render of the scene, no pass done yet.
    pub fn monte_carlo(&self) -> MonteCarlo {
        let img = crate::mc::McImage::new(self.scene.width.max(1) as usize, self.scene.height.max(1) as usize);
        MonteCarlo { scene: self.scene.clone(), img }
    }
}

impl MonteCarlo {
    /// One more pass (the first shoots 4 rays a pixel, the next ones add
    /// rays where the noise is high).
    pub fn pass(&mut self, opt: &RenderOptions) -> Result<(), String> {
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let pass = self.img.passes + 1;
        let progress = |done: usize, total: usize| {
            if let Some(p) = opt.progress {
                p(&format!("Pass {pass}"), done, total);
            }
        };
        let cancel = || opt.cancel.is_some_and(|c| c());
        crate::mc::pass(&self.scene, &mut self.img, threads, &progress, &cancel)?;
        if cancel() {
            return Err("cancelled".into());
        }
        Ok(())
    }

    /// The passes done.
    pub fn passes(&self) -> u32 {
        self.img.passes
    }

    /// The picture so far, with the scene's finishing.
    pub fn picture(&self) -> Picture {
        let (width, height) = (self.img.width, self.img.height);
        let rgb = crate::mc::paint_finished(&self.img, &self.scene);
        Picture { width, height, rgb, depth: vec![0; width * height], seconds: self.img.seconds, device: "CPU".into() }
    }
}

/// A camera flying through a scene: the Navigator without its window
/// ([`crate::navigate::Navigator`]), for gestures.
#[derive(Clone)]
pub struct Flight {
    nav: crate::navigate::Navigator,
    notes: Vec<String>,
}

impl Fractal {
    /// A camera starting at this scene's view.
    pub fn flight(&self) -> Flight {
        Flight { nav: crate::navigate::Navigator::new(&self.scene), notes: self.notes.clone() }
    }
}

impl Flight {
    /// Turns the camera where it stands (degrees): up/down, left/right,
    /// about the view axis.
    pub fn turn(&mut self, pitch: f64, yaw: f64, roll: f64) {
        self.nav.turn(pitch, yaw, roll);
    }

    /// Slides sideways by a drag of `(dx, dy)` picture heights.
    pub fn slide(&mut self, dx: f64, dy: f64) {
        self.nav.slide(dx, dy);
    }

    /// Flies forward (backward when negative) by that percentage of the
    /// distance to the surface (at most 90).
    pub fn fly(&mut self, percent: f64) {
        self.nav.fly(percent);
    }

    /// The field of view in degrees (the picture's height).
    pub fn fov(&self) -> f64 {
        self.nav.fov()
    }

    /// The quick scene for the picture while the camera moves.
    pub fn preview(&self, width: usize, height: usize) -> Fractal {
        Fractal { scene: self.nav.preview(width as i32, height as i32), notes: self.notes.clone() }
    }

    /// The scene with this view as the starting scene calculates it.
    pub fn view(&self, width: usize, height: usize) -> Fractal {
        Fractal { scene: self.nav.view(width as i32, height as i32), notes: self.notes.clone() }
    }
}

/// A formula of a scene.
pub struct FormulaInfo {
    /// Its place in the scene (for [`Fractal::set_formula`]).
    pub slot: usize,
    pub name: String,
    pub iterations: i32,
    /// Its own values by name.
    pub settings: Vec<(String, f64)>,
}

/// Called from the worker threads with the stage ("" for the march, then
/// "hard shadows", ...) and its work done and to do.
pub type Progress<'a> = &'a (dyn Fn(&str, usize, usize) + Sync);
/// Asked again and again: true stops the calculation.
pub type Cancel<'a> = &'a (dyn Fn() -> bool + Sync);

/// What a calculation reports while it runs.
#[derive(Default, Clone, Copy)]
pub struct RenderOptions<'a> {
    pub progress: Option<Progress<'a>>,
    pub cancel: Option<Cancel<'a>>,
}

/// A calculated picture.
pub struct Picture {
    pub width: usize,
    pub height: usize,
    /// Red, green, blue: a byte each, row after row from the top.
    pub rgb: Vec<u8>,
    /// The depth of every pixel: 65535 nearest, 0 the background.
    pub depth: Vec<u16>,
    /// How long the calculation took.
    pub seconds: f64,
    /// Where it ran: the graphics card's name and mode, or "CPU".
    pub device: String,
}

impl Picture {
    /// The picture as a PNG file.
    pub fn png(&self) -> Vec<u8> {
        crate::png::encode_rgb(self.width, self.height, &self.rgb)
    }

    /// The depth as a 16 bit greyscale PNG file.
    pub fn depth_png(&self) -> Vec<u8> {
        crate::png::encode_gray16(self.width, self.height, &self.depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preset_changes_and_renders() {
        let mut f = Fractal::preset("Integer Power").unwrap();
        f.resize(64, 48);
        assert_eq!(f.size(), (64, 48));
        f.set("iterations = 8").unwrap();
        assert!(f.text().contains("iterations = 8"));
        f.set("rotation = -40, 30, 10").unwrap();
        let r = f.rotation().unwrap();
        assert!((r[0] + 40.0).abs() < 1e-6 && (r[1] - 30.0).abs() < 1e-6 && (r[2] - 10.0).abs() < 1e-6, "{r:?}");
        assert!(f.set("no_such_key = 1").is_err());
        assert!(f.text().contains("iterations = 8"), "a wrong line leaves the scene as it was");
        let first = f.formula_list().remove(0);
        assert_eq!(first.name, "Integer Power");
        let (key, value) = first.settings[0].clone();
        f.set_formula(first.slot, &[(&key, value - 1.0)], Some(2)).unwrap();
        let now = f.formula_list().remove(0);
        assert_eq!((now.settings[0].1, now.iterations), (value - 1.0, 2));
        assert!(f.set_formula(first.slot, &[(&key, value), ("no such value", 1.0)], None).is_err());
        assert_eq!(f.formula_list()[0].settings[0].1, value - 1.0, "a wrong name leaves the formula as it was");
        assert!(f.set_formula(99, &[], None).is_err());
        let again = Fractal::from_text(&f.text()).unwrap();
        assert_eq!(again.text(), f.text());
        let from_m3p = Fractal::from_bytes("x.m3p", f.m3p()).unwrap();
        assert_eq!(from_m3p.size(), (64, 48));
        let stages = Mutex::new(0usize);
        let progress = |_: &str, done: usize, total: usize| {
            assert!(done <= total);
            *stages.lock().unwrap() += 1;
        };
        let pic = f.render(&RenderOptions { progress: Some(&progress), cancel: None }).unwrap();
        assert_eq!((pic.width, pic.height), (64, 48));
        assert_eq!(pic.rgb.len(), 64 * 48 * 3);
        assert_eq!(pic.depth.len(), 64 * 48);
        assert!(pic.depth.iter().any(|&d| d > 0), "the object is in the picture");
        assert!(*stages.lock().unwrap() > 0);
        assert_eq!(&pic.png()[1..4], b"PNG");
        let stop = || true;
        assert_eq!(f.render(&RenderOptions { progress: None, cancel: Some(&stop) }).err().as_deref(), Some("cancelled"));
    }

    #[test]
    fn the_formulas_are_listed() {
        assert!(formula_names().iter().any(|n| n == "Integer Power"));
        assert!(!formula_settings("Integer Power").unwrap().is_empty());
        assert!(formula_settings("no such formula").is_err());
    }
}

#[cfg(test)]
mod pick_tests {
    use super::*;

    #[test]
    fn pick_finds_the_surface_in_the_middle_and_not_the_corner() {
        let mut f = Fractal::preset("Integer Power").unwrap();
        f.resize(64, 48);
        let (pos, z) = f.pick(32, 24).expect("the bulb is in the middle");
        assert!(z > 0.0 && pos.iter().all(|v| v.is_finite()), "{pos:?} {z}");
        assert!(f.pick(0, 0).is_none(), "the corner is background");
        assert!(f.pick(64, 0).is_none());
        let v = f.variations(7, &RenderOptions::default()).unwrap();
        assert_eq!(v.len(), 15);
        assert_eq!(v[0].0, "1");
        assert!(v.iter().skip(1).any(|(_, g)| g.text() != f.text()), "the children differ");
        let mut mc = f.monte_carlo();
        mc.pass(&RenderOptions::default()).unwrap();
        mc.pass(&RenderOptions::default()).unwrap();
        assert_eq!(mc.passes(), 2);
        let p = mc.picture();
        assert_eq!((p.width, p.height, p.rgb.len()), (64, 48, 64 * 48 * 3));
        assert!(p.rgb.iter().any(|&v| v > 0), "the Monte Carlo picture shows something");
        let t = f.mb3d_text("t");
        assert!(t.starts_with("Mandelbulb3Dv18{"));
        assert!(Fractal::from_text(&t).is_ok(), "the text opens again");
    }
}
