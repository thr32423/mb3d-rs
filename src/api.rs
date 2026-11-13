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

    /// The formulas in use: name and iterations.
    pub fn formulas(&self) -> Vec<(String, i32)> {
        self.scene.formulas.iter().filter(|f| f.iterations > 0).map(|f| (f.formula.name(), f.iterations)).collect()
    }

    /// The scene as a Mandelbulb3D parameter file (.m3p).
    pub fn m3p(&self) -> Vec<u8> {
        crate::m3p::write(&self.scene)
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
        assert!(f.set("no_such_key = 1").is_err());
        assert!(f.text().contains("iterations = 8"), "a wrong line leaves the scene as it was");
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
