//! The Navigator's camera without its window: turn, slide and fly through a
//! scene by steps measured with the distance estimate at the camera, as
//! app/navi_form.rs does (`TransformNHeader`, `SetZoom`, the move buttons,
//! the drags), for the API and the iPad app's gestures.
//!
//! A [`Navigator`] keeps its own copy of the scene with the camera on the
//! middle point (where MB3D's Navigator keeps it); [`Navigator::preview`]
//! and [`Navigator::view`] give scenes to calculate, with the middle back
//! in front of the camera as "View to main" puts it.

use crate::autopilot::{camera_on_middle, length_of_size, local_de, mod_rot_point, unit_rows};
use crate::math::{build_rot_matrix, mat_mul};
use crate::scene::Scene;

const PID180: f64 = std::f64::consts::PI / 180.0;

/// A camera flying through a scene.
#[derive(Clone)]
pub struct Navigator {
    /// the scene it started from (all but the view is taken from it)
    main: Scene,
    /// the navigator's scene: the camera on the middle point
    nav: Scene,
    /// the far plane in distance estimates (`Edit4`)
    far: f64,
    /// `de_mul`: eases the distance after a move
    de_mul: f64,
}

impl Navigator {
    /// `TransformNHeader`: takes over the view of `main`.
    pub fn new(main: &Scene) -> Navigator {
        let mut nav = main.clone();
        nav.bin_search_steps = nav.bin_search_steps.min(7);
        nav.de_stop = nav.de_stop.max(0.01);
        camera_on_middle(&mut nav);
        let far = match local_de(&nav) {
            Some(de) => ((nav.z_end - nav.z_start) / de).clamp(1.0, 10000.0),
            None => 50.0,
        };
        Navigator { main: main.clone(), nav, far, de_mul: 1.0 }
    }

    /// The distance estimate at the camera (absolute units).
    fn distance(&self) -> f64 {
        match local_de(&self.nav) {
            Some(d) => d * self.de_mul,
            None => length_of_size(&self.nav) * self.nav.step_width(),
        }
    }

    /// `SetZoom`: the zoom from the distance at the camera, the far plane.
    fn fit_zoom(&mut self) {
        let sc = &mut self.nav;
        let de = match local_de(sc) {
            Some(d) => {
                let d = d * self.de_mul;
                sc.zoom = length_of_size(sc) * 2.0 / (d * sc.width as f64);
                d
            }
            None => sc.step_width() * length_of_size(sc),
        };
        sc.z_end = sc.z_start + (de * self.far).min(9999.0);
        self.de_mul = (self.de_mul - 1.0) * 0.8 + 1.0;
    }

    /// Turns the camera where it stands, by angles in degrees about its
    /// horizontal axis (`pitch`), its vertical axis (`yaw`) and its view
    /// axis (`roll`), as the Navigator's drag with the left button does: a
    /// drag of `(dx, dy)` pixels turns by `(-dy * k, dx * k)` with `k` the
    /// field of view per pixel of the picture's height.
    pub fn turn(&mut self, pitch: f64, yaw: f64, roll: f64) {
        let sc = &mut self.nav;
        if pitch != 0.0 || yaw != 0.0 {
            sc.vgrads = mat_mul(&build_rot_matrix(pitch * PID180, yaw * PID180, 0.0), &sc.vgrads);
        }
        if roll != 0.0 {
            sc.vgrads = mat_mul(&build_rot_matrix(0.0, 0.0, roll * PID180), &sc.vgrads);
        }
    }

    /// The field of view, in degrees (the picture's height).
    pub fn fov(&self) -> f64 {
        self.nav.fov_y
    }

    /// Slides the camera sideways so that what is in front of it follows
    /// a drag of `(dx, dy)` picture heights (right and down positive), as
    /// the Navigator's right-button drag does.
    pub fn slide(&mut self, dx: f64, dy: f64) {
        let de = self.distance();
        let sc = &mut self.nav;
        let fov = sc.fov_y.clamp(15.0, 120.0) * PID180;
        let per_height = de * 2.0 * (fov / 2.0).tan();
        let u = unit_rows(sc);
        let zm0 = sc.mid[2];
        for k in 0..3 {
            sc.mid[k] -= (u[0][k] * dx + u[1][k] * dy) * per_height;
        }
        let dz = sc.mid[2] - zm0;
        sc.z_start += dz;
        sc.z_end += dz;
        self.fit_zoom();
    }

    /// Flies forward (backward for a negative `percent`) by that share of
    /// the distance to the surface, as the Navigator's move buttons do
    /// with their step (`Edit1`, 1 to 90 %): the step is shortened where
    /// the distance changes much, so the camera does not go through.
    pub fn fly(&mut self, percent: f64) {
        if percent == 0.0 {
            return;
        }
        let i = percent.abs().clamp(0.1, 90.0);
        let back = percent < 0.0;
        let sign = if back { -1.0 } else { 1.0 };
        let valid = local_de(&self.nav).is_some();
        let de = self.distance();
        let sc = &mut self.nav;
        let u = unit_rows(sc);
        let v0 = sc.mid;
        let (zs0, zm0) = (sc.z_start, sc.mid[2]);
        let step = |sc: &mut Scene, d: f64| {
            for k in 0..3 {
                sc.mid[k] = v0[k] + u[2][k] * d;
            }
            sc.z_start = zs0 + sc.mid[2] - zm0;
        };
        let mut d = 0.01 * i * de * sign;
        step(sc, d);
        if valid {
            if let Some(mut nd) = local_de(sc).map(|x| x * self.de_mul) {
                let min_de = (de * (100.0 - i) * 0.008).max(1e-13);
                let max_de = (de * (100.0 + i) * 0.0125).max(min_de * 1.1);
                let mut ds = 1.25;
                for _ in 0..100 {
                    if nd > min_de && nd < max_de {
                        break;
                    }
                    ds *= 0.8;
                    let de2 = 0.5 * (de + nd);
                    d = 0.01 * i * de2 * ds * sign;
                    step(sc, d);
                    match local_de(sc) {
                        Some(x) => nd = x * self.de_mul,
                        None => break,
                    }
                }
            }
        }
        let dz = sc.mid[2] - zm0;
        sc.z_end += dz;
        self.fit_zoom();
    }

    /// The scene to calculate while the camera moves: `width` × `height`,
    /// the Navigator's quicker ray steps and simpler light (no shadows,
    /// ambient occlusion, reflections, depth of field).
    pub fn preview(&self, width: i32, height: i32) -> Scene {
        let mut sc = self.sized(width, height);
        sc.z_step_div = 0.7;
        sc.raystep_limiter = sc.raystep_limiter.max(sc.z_step_div * 0.5);
        sc.shadows = None;
        sc.ao = None;
        sc.deao = None;
        sc.dof = None;
        sc.mc.reflections = false;
        sc.vol_light = None;
        sc.normals_on_zbuf = false;
        sc.stereo_mode = 0;
        sc.slice_2d = 0;
        sc
    }

    /// The scene as the main window would calculate this view ("View to
    /// main"): the starting scene with the navigator's view, at `width` ×
    /// `height` (the detail follows the width).
    pub fn view(&self, width: i32, height: i32) -> Scene {
        let mut m = self.main.clone();
        view_into(&mut m, &self.nav, false, false);
        mod_rot_point(&mut m);
        m.calc_rect = None;
        m.tiling = None;
        let (w, h) = (width.max(16), height.max(16));
        m.scale_image(w as f64 / m.width.max(1) as f64);
        m.width = w;
        m.height = h;
        m
    }

    /// The navigator's scene at a size, the middle in front of the camera.
    fn sized(&self, width: i32, height: i32) -> Scene {
        let mut sc = self.nav.clone();
        mod_rot_point(&mut sc);
        let (w, h) = (width.max(16), height.max(16));
        sc.scale_image(w as f64 / sc.width.max(1) as f64);
        sc.width = w;
        sc.height = h;
        sc
    }
}

/// What "View to main" sends: the navigator's view `n` with its formulas
/// and the values changed there into the main window's parameters `m`,
/// which keep all else (size, detail, shadows, reflections, lighting).
/// (The Navigator window rounds its whole-number formula values after.)
pub fn view_into(m: &mut Scene, n: &Scene, destop_changed: bool, dynfog_changed: bool) {
    m.z_start = n.z_start;
    m.z_end = n.z_end;
    m.mid = n.mid;
    m.zoom = n.zoom;
    m.vgrads = n.vgrads;
    m.fov_y = n.fov_y;
    m.optic = n.optic;
    m.stereo_mode = 0;
    m.iterations = n.iterations;
    m.julia = n.julia;
    if n.julia {
        m.julia_c = n.julia_c;
    }
    m.dfog_on_it = n.dfog_on_it;
    m.rstop = n.rstop;
    if let (Some(a), Some(b)) = (m.decomb.as_mut(), n.decomb.as_ref()) {
        a.smooth = b.smooth;
    }
    if destop_changed {
        m.de_stop = n.de_stop;
    }
    m.color_on_it = n.color_on_it;
    if dynfog_changed {
        m.lighting.dyn_fog = n.lighting.dyn_fog;
    }
    m.formulas = n.formulas.clone();
    m.rot_4d = n.rot_4d;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bulb() -> Scene {
        let mut sc = Scene::preset("Integer Power").unwrap();
        sc.width = 64;
        sc.height = 48;
        sc
    }

    fn distance(n: &Navigator) -> f64 {
        local_de(&n.nav).unwrap()
    }

    #[test]
    fn flying_forward_comes_closer_and_back_goes_away() {
        let mut n = Navigator::new(&bulb());
        let d0 = distance(&n);
        n.fly(40.0);
        let d1 = distance(&n);
        assert!(d1 < d0, "forward: {d0} -> {d1}");
        n.fly(-40.0);
        n.fly(-40.0);
        assert!(distance(&n) > d1);
    }

    #[test]
    fn turning_turns_the_view_only() {
        let mut n = Navigator::new(&bulb());
        let (g0, m0) = (n.nav.vgrads, n.nav.mid);
        n.turn(0.0, 0.0, 0.0);
        assert_eq!(n.nav.vgrads, g0);
        n.turn(10.0, 5.0, 3.0);
        assert_ne!(n.nav.vgrads, g0);
        assert_eq!(n.nav.mid, m0, "the camera stays where it stands");
    }

    #[test]
    fn sliding_moves_across_the_view() {
        let mut n = Navigator::new(&bulb());
        let fwd = unit_rows(&n.nav)[2];
        let m0 = n.nav.mid;
        n.slide(0.2, 0.0);
        let d: Vec<f64> = (0..3).map(|k| n.nav.mid[k] - m0[k]).collect();
        let along: f64 = (0..3).map(|k| d[k] * fwd[k]).sum();
        let all: f64 = d.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!(all > 0.0 && along.abs() < all * 1e-9, "moved {all}, of that forward {along}");
    }

    #[test]
    fn the_view_keeps_the_scene_and_the_preview_is_simpler() {
        let mut sc = bulb();
        sc.shadows = None;
        let mut n = Navigator::new(&sc);
        n.fly(30.0);
        let v = n.view(128, 96);
        assert_eq!((v.width, v.height), (128, 96));
        assert_eq!(v.lighting.dyn_fog, sc.lighting.dyn_fog);
        let mut big = sc.clone();
        big.scale_image(2.0);
        assert_eq!(v.de_stop, big.de_stop, "the detail follows the size");
        assert_ne!(v.mid, sc.mid);
        let p = n.preview(64, 48);
        assert!(p.ao.is_none() && p.dof.is_none() && p.shadows.is_none());
    }
}
