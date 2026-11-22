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

fn book_kind(c: crate::formula_book::Category) -> u8 {
    use crate::formula_book::Category as C;
    match c {
        C::Escape3d => 0,
        C::Escape3dAnalytic => 1,
        C::Escape4d => 2,
        C::Escape4dAnalytic => 3,
        C::Transformation => 4,
        C::DifsShape => 5,
        C::DifsTransformation => 6,
    }
}

/// The Formula book in short: every formula the Formulas window lists with
/// its kind (0 a 3D escape time formula, 1 the same with an analytic DE, 2
/// a 4D formula, 3 the same with an analytic DE, 4 a transformation, 5 a
/// dIFS shape, 6 a dIFS transformation) and its description: the author's
/// text from the formula file, for a built-in formula the label of its
/// text (`@MB:...`, to look up in the language files).
pub fn formula_book() -> Vec<(String, u8, String)> {
    crate::formula_book::topics().into_iter().map(|t| (t.name, book_kind(t.category), t.description)).collect()
}

/// A formula's page of the Formula book. The texts the book has in every
/// language come as their labels (`@MB:...`).
pub struct FormulaPage {
    pub kind: u8,
    /// the kind's name and the sentence that says what it means
    pub kind_label: &'static str,
    pub explanation_label: &'static str,
    /// the author's text, or a built-in formula's label
    pub description: String,
    /// the values with their defaults and, for names that follow the
    /// usual conventions, what they do
    pub values: Vec<(String, f64, Option<&'static str>)>,
    pub hints: Vec<&'static str>,
}

/// The page of the formula `name`; `examples`: how many example files use
/// it (the hints say to start from one).
pub fn formula_page(name: &str, examples: usize) -> Option<FormulaPage> {
    let t = crate::formula_book::topics().into_iter().find(|t| t.name.eq_ignore_ascii_case(name))?;
    Some(FormulaPage {
        kind: book_kind(t.category),
        kind_label: t.category.label(),
        explanation_label: t.category.explanation(),
        hints: crate::formula_book::hints(&t, examples),
        values: t.options.iter().map(|(k, v)| (k.clone(), *v, crate::formula_book::option_explanation(k))).collect(),
        description: t.description,
    })
}

/// The example files of `folder` by the formulas they use (lower-case
/// formula name, the files' names without extension).
pub fn formula_examples(folder: &Path) -> Vec<(String, Vec<String>)> {
    let mut v: Vec<(String, Vec<String>)> = crate::formula_book::example_index(&[folder.to_path_buf()])
        .into_iter()
        .map(|(k, files)| (k, files.iter().map(|p| crate::formula_book::file_title(p)).collect()))
        .collect();
    v.sort();
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
        self.variations_with(seed, 1.0, opt)
    }

    /// [`Fractal::variations`] with the changes `strength` times MutaGen's
    /// own (0.1 to 3: the values, the Julia constant and the iterations
    /// change that much less or more; which formulas change stays).
    pub fn variations_with(&self, seed: u64, strength: f64, opt: &RenderOptions) -> Result<Vec<(String, Fractal)>, String> {
        let k = if strength.is_finite() { strength.clamp(0.1, 3.0) } else { 1.0 };
        let d = crate::mutagen::MutationConfig::default();
        let cfg = [
            d.formula_weight,
            d.params_weight,
            d.params_strength * k,
            d.julia_weight,
            d.julia_strength * k,
            d.iterations_weight,
            d.iterations_strength * k,
        ];
        self.variations_mutation(seed, cfg, opt)
    }

    /// [`Fractal::variations`] with the MutaGen window's own settings:
    /// how likely a child gets other formulas (0 to 1), then for the
    /// formulas' values, the Julia constant and the iteration counts how
    /// likely they change (0 to 1) and how much (0 to 3; MutaGen's own
    /// are 0.75, 1, 1, 0.5, 1, 0.5, 1).
    pub fn variations_mutation(&self, seed: u64, mutation: [f64; 7], opt: &RenderOptions) -> Result<Vec<(String, Fractal)>, String> {
        let d = crate::mutagen::MutationConfig::default();
        let fix = |v: f64, max: f64, own: f64| if v.is_finite() { v.clamp(0.0, max) } else { own };
        let cfg = crate::mutagen::MutationConfig {
            formula_weight: fix(mutation[0], 1.0, d.formula_weight),
            params_weight: fix(mutation[1], 1.0, d.params_weight),
            params_strength: fix(mutation[2], 3.0, d.params_strength),
            julia_weight: fix(mutation[3], 1.0, d.julia_weight),
            julia_strength: fix(mutation[4], 3.0, d.julia_strength),
            iterations_weight: fix(mutation[5], 1.0, d.iterations_weight),
            iterations_strength: fix(mutation[6], 3.0, d.iterations_strength),
            ..d
        };
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

/// An animation (the Animation window): keyframes of one size and the
/// frames between them, interpolated as MB3D does (or through every
/// keyframe, Catmull-Rom).
#[derive(Clone)]
pub struct Film {
    anim: crate::anim::Animation,
}

impl Film {
    /// No keyframes yet; `interpolation` 0 linear, 1 Bezier (MB3D's), 2
    /// Catmull-Rom.
    pub fn new(looped: bool, interpolation: i32) -> Film {
        use crate::anim::Interpolation;
        let anim = crate::anim::Animation {
            looped,
            interpolation: match interpolation {
                0 => Interpolation::Linear,
                1 => Interpolation::Bezier,
                _ => Interpolation::CatmullRom,
            },
            ..Default::default()
        };
        Film { anim }
    }

    /// A turntable: `steps` keyframes with the camera around the middle of
    /// the view, `frames` frames from each to the next, a whole turn.
    pub fn orbit(f: &Fractal, steps: usize, frames: u32) -> Film {
        let mut film = Film::new(true, 2);
        film.anim.keyframes = crate::anim::orbit_keyframes(&f.scene, steps.max(2), frames.max(1));
        film.anim.set_size_from(&f.scene);
        film
    }

    /// A keyframe (a copy of `f`) with `frames` frames to the next.
    pub fn add(&mut self, f: &Fractal, frames: u32) {
        if self.anim.keyframes.is_empty() {
            self.anim.set_size_from(&f.scene);
        }
        self.anim.keyframes.push(crate::anim::Keyframe::new(f.scene.clone(), frames.max(1)));
    }

    pub fn keyframes(&self) -> usize {
        self.anim.keyframes.len()
    }

    pub fn keyframe(&self, i: usize) -> Result<Fractal, String> {
        let k = self.anim.keyframes.get(i).ok_or_else(|| format!("there is no keyframe {}", i + 1))?;
        Ok(Fractal { scene: k.scene.clone(), notes: Vec::new() })
    }

    /// The number of frames.
    pub fn frames(&self) -> usize {
        self.anim.frame_count()
    }

    /// The scene of frame `i` (from 0).
    pub fn frame(&self, i: usize) -> Result<Fractal, String> {
        Ok(Fractal { scene: self.anim.frame_scene(i)?, notes: Vec::new() })
    }

    /// The scene `offset` (0..1) of the way from frame `i` to the next: the
    /// pictures a motion blur averages.
    pub fn frame_at(&self, i: usize, offset: f64) -> Result<Fractal, String> {
        Ok(Fractal { scene: self.anim.frame_scene_at(i, offset)?, notes: Vec::new() })
    }

    /// The short names of the values the curves show (`zoom`, `x`, ...).
    pub fn curve_names() -> Vec<&'static str> {
        crate::curves::PARAMS.iter().map(|p| p.key).collect()
    }

    /// Value `param` (an index into [`Film::curve_names`]) of every frame
    /// (NaN where a frame has none) and of every keyframe with its frame.
    pub fn curve(&self, param: usize) -> Option<(Vec<f64>, Vec<(usize, f64)>)> {
        crate::curves::PARAMS.get(param).map(|p| crate::curves::curve(&self.anim, p))
    }

    /// Sets value `param` of keyframe `key` (read it back with
    /// [`Film::keyframe`]).
    pub fn set_curve_key(&mut self, param: usize, key: usize, value: f64) {
        if let Some(p) = crate::curves::PARAMS.get(param) {
            crate::curves::set_key(&mut self.anim, p, key, value);
        }
    }
}

/// The object as a triangle mesh (the BulbTracer), for 3D printing, other
/// programs and AR.
pub struct Mesh3D {
    mesh: crate::mesh::Mesh,
    normals: Vec<[f32; 3]>,
}

impl Fractal {
    /// Traces the object in a `resolution`³ grid of the BulbTracer's
    /// default cube, with the palette's colours when `colors`, `smooth`
    /// smoothing passes; centred with its largest side `size` long when
    /// `size` > 0.
    pub fn mesh(&self, resolution: usize, colors: bool, smooth: u32, size: f64, opt: &RenderOptions) -> Result<Mesh3D, String> {
        let mp = crate::mesh::MeshParams { resolution: resolution.clamp(8, 2048), colors, smooth, ..Default::default() };
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let progress = |done: usize, total: usize| {
            if let Some(p) = opt.progress {
                p("Tracing", done, total);
            }
        };
        let cancel = || opt.cancel.is_some_and(|c| c());
        let mut mesh = crate::mesh::trace(&self.scene, &mp, threads, &progress, &cancel)?;
        if cancel() {
            return Err("cancelled".into());
        }
        if size > 0.0 {
            mesh.center(size);
        }
        let normals = mesh.normals();
        Ok(Mesh3D { mesh, normals })
    }
}

impl Mesh3D {
    pub fn vertices(&self) -> &[[f32; 3]] {
        &self.mesh.vertices
    }

    pub fn faces(&self) -> &[[u32; 3]] {
        &self.mesh.faces
    }

    pub fn normals(&self) -> &[[f32; 3]] {
        &self.normals
    }

    /// Red, green, blue (0..1) and the palette position of each vertex.
    pub fn colors(&self) -> Option<&[[f32; 4]]> {
        self.mesh.colors.as_deref().filter(|c| c.len() == self.mesh.vertices.len())
    }

    /// Writes OBJ, PLY or STL (by the extension).
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.mesh.save(path)
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

    /// The scene's directions of the picture's right, down and depth (each
    /// of length 1), for a compass.
    pub fn axes(&self) -> [[f64; 3]; 3] {
        self.nav.axes()
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
        let weak = f.variations_with(7, 0.1, &RenderOptions::default()).unwrap();
        assert_eq!(weak.len(), 15);
        assert!(weak.iter().zip(&v).any(|(a, b)| a.1.text() != b.1.text()), "the strength changes the children");
        // nothing likely to change: every child is the scene
        let same = f.variations_mutation(7, [0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0], &RenderOptions::default()).unwrap();
        assert!(same.iter().all(|(_, g)| g.text() == f.text()), "weights of 0 change nothing");
        let book = formula_book();
        assert!(book.iter().any(|(n, k, _)| n == "Integer Power" && *k <= 3), "the book knows the bulb");
        let page = formula_page("Integer Power", 1).expect("the bulb has a page");
        assert!(page.kind_label.starts_with("@MB:") && !page.values.is_empty(), "{:?}", page.values);
        let mut cam = f.flight();
        let a = cam.axes();
        for row in a {
            let n = (row[0] * row[0] + row[1] * row[1] + row[2] * row[2]).sqrt();
            assert!((n - 1.0).abs() < 1e-9, "{a:?}");
        }
        cam.turn(0.0, 30.0, 0.0);
        assert!(cam.axes() != a, "a turn moves the compass");
        let mut mc = f.monte_carlo();
        mc.pass(&RenderOptions::default()).unwrap();
        mc.pass(&RenderOptions::default()).unwrap();
        assert_eq!(mc.passes(), 2);
        let p = mc.picture();
        assert_eq!((p.width, p.height, p.rgb.len()), (64, 48, 64 * 48 * 3));
        assert!(p.rgb.iter().any(|&v| v > 0), "the Monte Carlo picture shows something");
        let mut film = Film::new(false, 2);
        film.add(&f, 3);
        let mut g = f.clone();
        g.set("zoom = 0.5").unwrap();
        film.add(&g, 3);
        assert_eq!(film.frames(), 4);
        assert!(film.frame(1).unwrap().text() != f.text() && film.frame(3).is_ok() && film.frame(4).is_err());
        // the zoom's curve: the keyframes' values at frames 0 and 3, set
        // through a keyframe
        let zi = Film::curve_names().iter().position(|&n| n == "zoom").unwrap();
        let (values, keys) = film.curve(zi).unwrap();
        assert_eq!(values.len(), 4);
        assert_eq!(keys.iter().map(|k| k.0).collect::<Vec<_>>(), [0, 3]);
        assert!((keys[1].1 - 0.5).abs() < 1e-9);
        film.set_curve_key(zi, 1, 2.0);
        assert!(film.keyframe(1).unwrap().text().contains("zoom = 2"));
        let mid = film.frame_at(1, 0.5).unwrap();
        assert!(mid.text() != film.frame(1).unwrap().text() && mid.text() != film.frame(2).unwrap().text(), "half way between frames");
        let orbit = Film::orbit(&f, 4, 2);
        assert_eq!((orbit.keyframes(), orbit.frames()), (4, 8));
        let m = f.mesh(24, true, 1, 0.3, &RenderOptions::default()).unwrap();
        assert!(!m.faces().is_empty() && m.normals().len() == m.vertices().len() && m.colors().is_some());
        assert!(m.vertices().iter().all(|v| v.iter().all(|x| x.abs() <= 0.16)), "centred in 0.3");
        let t = f.mb3d_text("t");
        assert!(t.starts_with("Mandelbulb3Dv18{"));
        assert!(Fractal::from_text(&t).is_ok(), "the text opens again");
    }
}
