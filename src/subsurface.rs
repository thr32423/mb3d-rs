//! Subsurface scattering in the painting (not in MB3D): where the surface
//! material lets light into the palette colours ([`Material::subsurface`]),
//! the light of a lamp behind a part comes through it, weakened by the
//! material it crossed, and tinted: the colours of the surface go farther
//! than the others (wax, jade, marble, leaves against the sun).
//!
//! Step 1, the quick way real-time renderers use: for every pixel and
//! lamp, the material between the pixel's surface and the lamp is
//! measured along one line towards the lamp (points at even distances,
//! inside or outside the fractal), and the lamp's light comes out of the
//! surface weakened by it (Beer-Lambert: half of it lost every
//! [`Material::depth`] of the picture's width, scaled by the colour), more
//! when the lamp is behind the part than to its side, more when the light
//! goes on towards the eye.  What the light meets beyond the part (another
//! part in front of the lamp) does not count, nor light that scatters
//! more than once: Monte Carlo does not have it yet.  The side of the
//! surface turned away from a lamp is lit a little in the shading
//! (`LightVals::subsurface_at`, the light wraps around).
//!
//! Mandelbulb3D SE (Glenn Olander, github.com/Rodrigo-Spacecraft/
//! Mandelbulb3D-SE) has subsurface scattering for its transparent dIFS
//! parts; from it comes the depth as a part of the picture's width, so
//! the look stays the same with the zoom.  Nothing of its code is used.
//!
//! Only when the whole picture is calculated at once (not on tiles).
//!
//! [`Material::subsurface`]: crate::scene::Material::subsurface
//! [`Material::depth`]: crate::scene::Material::depth
use crate::calc::{CalcParams, Marcher};
use crate::emission::{EmitLight, Pix};
use crate::gbuffer::SiLight;
use crate::lighting::{LightVals, PaintCamera};
use crate::math::*;
use crate::scene::Scene;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Points measured along the line to a lamp.
const POINTS: usize = 24;
/// The line's length, in depths ([`crate::scene::Material::depth`]): the
/// light left after it is e^-3 (5 %, less for the colours the surface
/// does not have).
const REACH: f64 = 3.0;

/// A lamp as the pass sees it: the direction to it (scene space) or its
/// place, and its colour (as the shading adds it, 0..255).
struct Lamp {
    dir: Vec3,
    pos: Option<Vec3>,
    col: [f32; 3],
}

/// The light that comes through the material to each pixel of the
/// picture, as the colour of a light of strength 1 falling straight on
/// the surface (as [`EmitLight`]).
#[allow(clippy::too_many_arguments)]
pub(crate) fn light(
    sc: &Scene,
    p: &CalcParams,
    gbuf: &[SiLight],
    lv: &LightVals,
    cam: &PaintCamera,
    threads: usize,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Option<std::sync::Arc<EmitLight>> {
    let m = sc.material?;
    if !m.scatters() {
        return None;
    }
    let timing = std::env::var_os("MB3D_SUBSURFACE_TIMING").is_some();
    let t0 = std::time::Instant::now();
    // the lamps, as the hard shadows find them (render::shadow_job)
    let rot = normalise_matrix_to(1.0, &p.vgrads);
    let mid = sc.stereo_mid();
    let lamps: Vec<Lamp> = lv
        .lights
        .iter()
        .filter(|l| l.on && l.col.iter().any(|c| *c > 0.0))
        .map(|l| {
            let ln = [l.ln[0] as f64, l.ln[1] as f64, l.ln[2] as f64];
            if l.positional {
                Lamp { dir: [0.0; 3], pos: Some([ln[0] + mid[0], ln[1] + mid[1], ln[2] + mid[2]]), col: l.col }
            } else {
                Lamp { dir: normalize(rotate_vector_reverse(&normalize(ln), &rot)), pos: None, col: l.col }
            }
        })
        .collect();
    if lamps.is_empty() {
        return None;
    }
    let [x0, y0, w, h] = p.rect;
    let (wu, hu) = (w as usize, h as usize);
    let threads = threads.max(1);
    // the depth in scene units: a part of the picture's width
    let depth = m.depth() as f64 * p.width as f64 * p.step_width;
    let reach = REACH * depth;
    let step = reach / POINTS as f64;

    let mut light = vec![[0f32; 3]; wu * hu];
    let mut pix = vec![Pix::default(); wu * hu];
    let rows = hu.div_ceil(threads).max(1);
    let done = AtomicUsize::new(0);
    let lamps = &lamps;
    std::thread::scope(|s| {
        for (ci, (lc, pc)) in light.chunks_mut(rows * wu).zip(pix.chunks_mut(rows * wu)).enumerate() {
            let done = &done;
            s.spawn(move || {
                let mut mr = Marcher::new(p, 1);
                // (the ground plane is not material)
                mr.no_ground = true;
                for (k, (out, px)) in lc.iter_mut().zip(pc.iter_mut()).enumerate() {
                    let i = ci * rows * wu + k;
                    if k % wu == 0 {
                        if cancel() {
                            return;
                        }
                        progress(done.fetch_add(1, Ordering::Relaxed), hu);
                    }
                    let (x, y) = ((i % wu) as i32, (i / wu) as i32);
                    let si = &gbuf[i];
                    let plv = lv.plv_for_pixel(si, x + x0, y + y0, cam);
                    let idif0 = lv.s_col_zmul * plv.z_pos;
                    let amount = lv.subsurface_at(si, idif0);
                    if amount <= 0.0 || !mr.surface_point(x, y, si, 4, 0.01) {
                        continue;
                    }
                    let view = normalize(mr.vfov);
                    let mut n = normalize(rotate_vector_reverse(&[si.normal[0] as f64, si.normal[1] as f64, si.normal[2] as f64], &p.vgrads));
                    if !n.iter().all(|v| v.is_finite()) {
                        continue;
                    }
                    if dot(&n, &view) > 0.0 {
                        n = scale(n, -1.0);
                    }
                    let pos = mr.it.c;
                    let eps = mr.ms_de_stop.max(1e-6);
                    let width = p.step_width * (1.0 + mr.mzz.abs() * p.de_stop_factor as f64);
                    *px = Pix { pos, n: [n[0] as f32, n[1] as f32, n[2] as f32], eps, zz: mr.mzz as f32, width: width as f32, ok: true };
                    // the tint: the surface's own colours go farther
                    let c = lv.calc_colors(si, idif0).0;
                    let cmax = c[0].max(c[1]).max(c[2]).max(1e-6);
                    let tint = c.map(|v| 0.25 + 0.75 * (v / cmax).clamp(0.0, 1.0));
                    // where the line starts: just under the surface
                    let start = [pos[0] - n[0] * width * 2.0, pos[1] - n[1] * width * 2.0, pos[2] - n[2] * width * 2.0];
                    // (the points' offset, the same for a pixel and its lamps)
                    let jitter = (((i as u32).wrapping_mul(2654435761) >> 8) as f64) / (1u32 << 24) as f64;
                    let mut sum = [0f32; 3];
                    for l in lamps {
                        let (dir, att) = match l.pos {
                            None => (l.dir, 1.0),
                            Some(lp) => {
                                let v = sub(lp, pos);
                                let d2 = dot(&v, &v);
                                (scale(v, 1.0 / d2.sqrt().max(1e-300)), 1.0 / (d2 as f32 + 1e-30))
                            }
                        };
                        let ndl = dot(&n, &dir) as f32;
                        // from behind fully, from the side half, from the
                        // front not at all (that light falls on it)
                        let back = 0.5 - 0.5 * ndl;
                        if back <= 0.01 {
                            continue;
                        }
                        // more where the light goes on towards the eye
                        let fwd = (dot(&dir, &view) as f32).max(0.0);
                        let phase = 0.5 + 1.5 * fwd * fwd * fwd * fwd;
                        // the material on the line to the lamp
                        let mut thick = 0f64;
                        let mut t = step * jitter;
                        while t < reach {
                            mr.it.c = [start[0] + dir[0] * t, start[1] + dir[1] * t, start[2] + dir[2] * t];
                            mr.ms_de_stop = eps;
                            let d = mr.calc_de();
                            if mr.it.it_result >= mr.max_its_result || d <= eps as f64 {
                                thick += step;
                                t += step;
                            } else {
                                // no material nearer than the distance
                                let free = d * p.step_width;
                                if free >= reach - t {
                                    break;
                                }
                                t += step.max((free / step).floor() * step);
                            }
                        }
                        let k = back * phase * att * amount / 255.0;
                        for ch in 0..3 {
                            let tr = (-(thick / (depth * tint[ch] as f64)) as f32).exp();
                            sum[ch] += l.col[ch] * tr * k;
                        }
                    }
                    *out = sum;
                }
            });
        }
    });
    if cancel() {
        return None;
    }
    let t1 = t0.elapsed().as_secs_f64();
    despeckle(&mut light, &pix, wu, hu);
    crate::emission::smooth(&mut light, &pix, wu, hu, threads, cancel);
    if cancel() {
        return None;
    }
    if timing {
        let n = pix.iter().filter(|p| p.ok).count();
        let most = light.iter().map(|c| c[0].max(c[1]).max(c[2])).fold(0f32, f32::max);
        let mean = light.iter().map(|c| c[0] + c[1] + c[2]).sum::<f32>() / (3 * n.max(1)) as f32;
        crate::elog!("  subsurface: {} pixels, {} lamps, {:.2} s, smoothed in {:.2} s; light mean {:.4}, most {:.4}", n, lamps.len(), t1, t0.elapsed().as_secs_f64() - t1, mean, most);
    }
    Some(std::sync::Arc::new(EmitLight { rect: p.rect, light }))
}

/// Lone bright pixels (a part thinner than a pixel, far away or at an
/// edge, lets all the light through where its neighbours let none): a
/// pixel's light at most 1.5 times its second brightest neighbour's, so a
/// lone pixel is dimmed but a thin line (two bright neighbours) is kept.
fn despeckle(light: &mut [[f32; 3]], pix: &[Pix], w: usize, h: usize) {
    let lum = |c: [f32; 3]| 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
    let src: Vec<f32> = light.iter().map(|c| lum(*c)).collect();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !pix[i].ok || src[i] <= 0.0 {
                continue;
            }
            let (mut a, mut b) = (0f32, 0f32);
            for dy in -1..=1isize {
                for dx in -1..=1isize {
                    let (xx, yy) = (x as isize + dx, y as isize + dy);
                    if (dx == 0 && dy == 0) || xx < 0 || yy < 0 || xx >= w as isize || yy >= h as isize {
                        continue;
                    }
                    let v = src[yy as usize * w + xx as usize];
                    if v > a {
                        b = a;
                        a = v;
                    } else if v > b {
                        b = v;
                    }
                }
            }
            let most = 1.5 * b;
            if src[i] > most {
                let k = most / src[i];
                light[i] = light[i].map(|v| v * k);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bulb(extra: &str) -> Scene {
        // the default bulb, one lamp behind it
        let text = format!("width = 120\nheight = 90\n{extra}\n[light]\nslot = 1\ncolor = #FFFFFF\nx_angle = 0\ny_angle = 170\namplitude = 1\n");
        Scene::parse(&text).unwrap()
    }

    fn total(sc: &Scene) -> Option<f32> {
        let (p, gbuf) = crate::render::calculate(sc, &|_, _| {}).unwrap();
        let cam = crate::render::paint_camera(sc, &p);
        let lv = crate::render::light_vals(sc, &sc.lighting, &cam);
        let l = light(sc, &p, &gbuf, &lv, &cam, 4, &|_, _| {}, &|| false)?;
        assert_eq!(l.rect, p.rect);
        // the background gets none
        assert!(l.light.iter().zip(&gbuf).all(|(c, si)| si.zpos() < 32768 || *c == [0.0; 3]));
        Some(l.light.iter().map(|c| c[0] + c[1] + c[2]).sum())
    }

    /// Lit from behind, light comes through the bulb, more when it goes
    /// deeper into the material; without subsurface there is no pass.
    #[test]
    fn light_comes_through_the_material() {
        let on = "material_subsurface = 1, 1, 1, 1, 1, 1, 1, 1, 1, 1";
        let thin = total(&bulb(&format!("{on}\nmaterial_subsurface_depth = 0.01"))).unwrap();
        let deep = total(&bulb(&format!("{on}\nmaterial_subsurface_depth = 0.1"))).unwrap();
        assert!(thin > 0.0, "some light comes through: {thin}");
        assert!(deep > thin * 1.2, "more where it goes deeper: {thin} {deep}");
        assert!(total(&bulb("material_glow = 0")).is_none());
    }

    #[test]
    fn the_settings_are_kept_in_the_scene_text() {
        let sc = bulb("material_subsurface = 0, 0.5, 0, 0, 0, 0, 0, 0, 0, 1\nmaterial_subsurface_depth = 0.07");
        let back = Scene::parse(&sc.to_text()).unwrap();
        let m = back.material.unwrap();
        assert_eq!(m.subsurface[1], 0.5);
        assert_eq!(m.subsurface[9], 1.0);
        assert_eq!(m.subsurface_depth, 0.07);
        assert!((m.depth() - 0.07).abs() < 1e-7);
        assert_eq!(crate::scene::Material::default().depth(), crate::scene::Material::SUBSURFACE_DEPTH);
    }
}
