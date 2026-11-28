//! Subsurface scattering in the painting (not in MB3D): where the surface
//! material lets light into the palette colours ([`Material::subsurface`]),
//! the light of a lamp behind a part comes through it, weakened by the
//! material it crossed, and tinted: the colours of the surface go farther
//! than the others (wax, jade, marble, leaves against the sun).
//!
//! The quick way real-time renderers use, with real rays: for every pixel
//! and lamp a ray goes from just under the surface towards the lamp,
//! inside the material, to where it comes out (the thickness), and from
//! there on towards the lamp (its shadow).  The lamp's light comes out of
//! the surface weakened by the thickness (Beer-Lambert: e^-1 left after
//! every [`Material::depth`] of the picture's width, the surface's own
//! colours going farther), more when the lamp is behind the part than to
//! its side, more when the light goes on towards the eye.  Light that
//! scatters more than once is Monte Carlo's (`mc`, which has the same
//! direct light and a bounce through the material).  The side of the
//! surface turned away from a lamp is lit a little in the shading
//! (`LightVals::subsurface_at`, the light wraps around).
//!
//! The rays run on the graphics card (the reflection pass's march, inside
//! the set for the thickness) or on the CPU, the same march.
//!
//! Mandelbulb3D SE (Glenn Olander, github.com/Rodrigo-Spacecraft/
//! Mandelbulb3D-SE) has subsurface scattering for its transparent dIFS
//! parts; from it comes the depth as a part of the picture's width, so
//! the look stays the same with the zoom.  Nothing of its code is used.
//!
//! Only when the whole picture is calculated at once (not on tiles), not
//! in inside rendering.
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

/// The thickness counted, in depths ([`crate::scene::Material::depth`]):
/// the light left after it is e^-3 (5 %, less for the colours the surface
/// does not have).
pub(crate) const REACH: f64 = 3.0;

/// How far a lamp's shadow is followed beyond a part, in picture widths
/// (at the middle of the scene).
const SHADOW_WIDTHS: f64 = 2.0;

/// How far each colour goes, relative to the depth: the surface's own
/// colours (`dif`, its diffuse colour) farther than the others.
pub(crate) fn tint(dif: [f32; 3]) -> [f32; 3] {
    let cmax = dif[0].max(dif[1]).max(dif[2]).max(1e-6);
    dif.map(|v| 0.25 + 0.75 * (v / cmax).clamp(0.0, 1.0))
}

/// A lamp as the pass sees it: the direction to it (scene space) or its
/// place, and its colour (as the shading adds it, 0..255).
struct Lamp {
    dir: Vec3,
    pos: Option<Vec3>,
    col: [f32; 3],
}

/// A line from a pixel to a lamp.
#[derive(Clone, Copy)]
struct Line {
    pix: u32,
    lamp: u8,
    /// just under the surface, and the unit direction to the lamp
    start: Vec3,
    dir: Vec3,
    /// the pixel's depth (steps)
    zz: f32,
    /// how far the lamp's shadow is followed (scene units)
    far: f64,
    /// the light's share: back side, phase, distance, amount (/255)
    k: f32,
    tint: [f32; 3],
}

/// A ray from `origin` along the unit `dir` at the depth `zz` (steps),
/// inside the set or not, marched as the pixel march steps
/// (`Marcher::march_from`'s steps and its test for the surface; inside the
/// set the DE is inverted) but without what the pixels need at the hit
/// (the binary search, colour, depth values), and only up to `max` (scene
/// units): where it meets the surface.
fn march_line(m: &mut Marcher, origin: Vec3, dir: Vec3, zz: f32, max: f64, step_mul: f64) -> Option<Vec3> {
    let p = m.p;
    let sw = p.step_width;
    let max = max / sw;
    let mut t = 0f64;
    for _ in 0..100_000 {
        m.it.c = [origin[0] + dir[0] * t * sw, origin[1] + dir[1] * t * sw, origin[2] + dir[2] * t * sw];
        m.mzz = zz as f64 + t;
        m.ms_de_stop = (p.de_stop as f64 * (1.0 + m.mzz.abs() * p.de_stop_factor as f64)) as f32;
        let d = m.calc_de();
        let eps = m.ms_de_stop as f64;
        if !(m.it.it_result < p.min_it || (m.it.it_result < m.max_its_result && d >= eps)) {
            return Some(m.it.c);
        }
        t += f64::max(0.11, (d - p.ms_de_sub as f64 * eps) * step_mul);
        if t > max {
            return None;
        }
    }
    None
}

/// A batch of rays from `origin` along the unit `dir` at the depth `zz`,
/// up to the length `max` (scene units), inside the set or not: where each
/// meets the surface (None: nowhere).  On the card when it is ready
/// (`card`; its march goes to the far end, what lies beyond `max` is left
/// out after), else (or for what the card leaves) on the CPU.
#[cfg_attr(not(feature = "gpu"), allow(unused_variables))]
fn rays(p: &CalcParams, rays: &[(Vec3, Vec3, f32, f64)], inside: bool, threads: usize, card: bool, cancel: &(dyn Fn() -> bool + Sync)) -> (Vec<Option<Vec3>>, usize) {
    let sw = p.step_width;
    let mut out: Vec<Option<Vec3>> = vec![None; rays.len()];
    let mut done = 0;
    #[cfg(feature = "gpu")]
    if card && !rays.is_empty() {
        let tasks: Vec<crate::gpu::ReflectTask> = rays
            .iter()
            .enumerate()
            .map(|(i, (o, d, zz, max))| crate::gpu::ReflectTask { origin: *o, dir: [(d[0] * sw) as f32, (d[1] * sw) as f32, (d[2] * sw) as f32], zz: *zz, pixel: i as u32, inside, max_len: (*max / sw) as f32, ..Default::default() })
            .collect();
        match crate::gpu::reflect_batch(p, &tasks, &crate::gpu::ReflectOpts::default(), None, cancel) {
            Ok((hits, n, _)) => {
                for ((o, h), (org, d, _, max)) in out.iter_mut().zip(&hits).zip(rays).take(n) {
                    *o = (h.hit && !h.skip && dot(&sub(h.pos, *org), d) <= *max).then_some(h.pos);
                }
                done = n;
            }
            Err(e) => crate::elog!("  subsurface: the card gave up ({e}), the rays on the CPU"),
        }
    }
    let on_card = done;
    let rest = &rays[done..];
    let per = rest.len().div_ceil(threads.max(1)).max(1);
    std::thread::scope(|s| {
        for (part, o) in rest.chunks(per).zip(out[done..].chunks_mut(per)) {
            s.spawn(move || {
                let mut m = Marcher::new(p, 1);
                m.no_ground = true;
                m.inside = inside;
                m.calc_inside = inside;
                for (k, ((org, d, zz, max), v)) in part.iter().zip(o.iter_mut()).enumerate() {
                    if k % 256 == 0 && cancel() {
                        return;
                    }
                    // (the thickness with the scene's ray step; the shadow,
                    // which only asks whether anything is in the way, by the
                    // whole distance: twice as fast, the same light within
                    // a tenth of a grey level on ApolloBalloons)
                    *v = march_line(&mut m, *org, *d, *zz, *max, if inside { p.z_step_div as f64 } else { (p.z_step_div as f64).max(1.0) });
                }
            });
        }
    });
    (out, on_card)
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
    if !m.scatters() || p.inside_rendering {
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
    // how far the lamps' shadow is followed (SHADOW_WIDTHS)
    let shadow_len = SHADOW_WIDTHS * p.width as f64 * p.step_width * (1.0 + 0.5 * p.zend.abs() * p.de_stop_factor as f64);

    // 1. the surfaces and the lines to the lamps
    let mut pix = vec![Pix::default(); wu * hu];
    let rows = hu.div_ceil(threads).max(1);
    let done = AtomicUsize::new(0);
    let lamps_r = &lamps;
    let mut lines: Vec<Line> = std::thread::scope(|s| {
        let hs: Vec<_> = pix
            .chunks_mut(rows * wu)
            .enumerate()
            .map(|(ci, pc)| {
                let done = &done;
                s.spawn(move || {
                    let mut out = Vec::new();
                    let mut mr = Marcher::new(p, 1);
                    for (k, px) in pc.iter_mut().enumerate() {
                        let i = ci * rows * wu + k;
                        if k % wu == 0 {
                            if cancel() {
                                return out;
                            }
                            progress(done.fetch_add(1, Ordering::Relaxed), hu * 2);
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
                        let width = p.step_width * (1.0 + mr.mzz.abs() * p.de_stop_factor as f64);
                        *px = Pix { pos, n: [n[0] as f32, n[1] as f32, n[2] as f32], eps: mr.ms_de_stop.max(1e-6), zz: mr.mzz as f32, width: width as f32, ok: true };
                        let tint = tint(lv.calc_colors(si, idif0).0);
                        // just under the surface
                        let start = [pos[0] - n[0] * width * 2.0, pos[1] - n[1] * width * 2.0, pos[2] - n[2] * width * 2.0];
                        for (li, l) in lamps_r.iter().enumerate() {
                            let (dir, att, far) = match l.pos {
                                None => (l.dir, 1.0, shadow_len),
                                Some(lp) => {
                                    let v = sub(lp, pos);
                                    let d2 = dot(&v, &v);
                                    (scale(v, 1.0 / d2.sqrt().max(1e-300)), 1.0 / (d2 as f32 + 1e-30), d2.sqrt().min(shadow_len))
                                }
                            };
                            // from behind fully, from the side half, from
                            // the front not at all (that light falls on it)
                            let back = 0.5 - 0.5 * dot(&n, &dir) as f32;
                            if back <= 0.01 {
                                continue;
                            }
                            // more where the light goes on towards the eye
                            let fwd = (dot(&dir, &view) as f32).max(0.0);
                            let phase = 0.5 + 1.5 * fwd * fwd * fwd * fwd;
                            out.push(Line { pix: i as u32, lamp: li as u8, start, dir, zz: mr.mzz as f32, far, k: back * phase * att * amount / 255.0, tint });
                        }
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    });
    if cancel() {
        return None;
    }
    let t1 = t0.elapsed().as_secs_f64();
    #[cfg(feature = "gpu")]
    let card = crate::gpu::enabled() && crate::gpu::unsupported(p).is_none() && std::env::var_os("MB3D_SUBSURFACE_CPU").is_none() && crate::gpu::reflect_compiled(p);
    #[cfg(not(feature = "gpu"))]
    let card = false;

    // 2. the thickness: inside the set to where the line comes out
    // (a line that does not come out within 3 depths lets no light through:
    // its march ends there)
    let first: Vec<(Vec3, Vec3, f32, f64)> = lines.iter().map(|l| (l.start, l.dir, l.zz, reach)).collect();
    let (exits, card1) = rays(p, &first, true, threads, card, cancel);
    if cancel() {
        return None;
    }
    let mut thick = vec![f64::INFINITY; lines.len()];
    for (k, (l, e)) in lines.iter().zip(&exits).enumerate() {
        if let Some(e) = e {
            let t = dot(&sub(*e, l.start), &l.dir);
            if t < reach {
                thick[k] = t.max(0.0);
            }
        }
    }
    let t2 = t0.elapsed().as_secs_f64();

    // 3. the lamps' shadow beyond: on from where the line came out, where
    // the light would be seen (more than about a tenth of a grey level)
    let seen = |k: usize, th: f64| {
        let l = &lines[k];
        let col = lamps[l.lamp as usize].col;
        let most = (0..3).map(|ch| col[ch] * (-(th / (depth * l.tint[ch] as f64)) as f32).exp()).fold(0f32, f32::max);
        most * l.k > 3e-4
    };
    for k in 0..lines.len() {
        if thick[k].is_finite() && !seen(k, thick[k]) {
            thick[k] = f64::INFINITY;
        }
    }
    let keep: Vec<usize> = (0..lines.len()).filter(|&k| thick[k].is_finite()).collect();
    let second: Vec<(Vec3, Vec3, f32, f64)> = keep
        .iter()
        .map(|&k| {
            let l = &lines[k];
            let px = &pix[l.pix as usize];
            let o = add(l.start, scale(l.dir, thick[k] + 2.0 * px.width as f64));
            (o, l.dir, l.zz, l.far)
        })
        .collect();
    let (blocked, card2) = rays(p, &second, false, threads, card, cancel);
    if cancel() {
        return None;
    }
    for (j, &k) in keep.iter().enumerate() {
        if blocked[j].is_some() {
            thick[k] = f64::INFINITY;
        }
    }
    progress(hu * 2, hu * 2);

    // 4. the light
    let mut light = vec![[0f32; 3]; wu * hu];
    for (k, l) in lines.iter_mut().enumerate() {
        if !thick[k].is_finite() {
            continue;
        }
        let col = lamps[l.lamp as usize].col;
        let o = &mut light[l.pix as usize];
        for ch in 0..3 {
            o[ch] += col[ch] * (-(thick[k] / (depth * l.tint[ch] as f64)) as f32).exp() * l.k;
        }
    }
    despeckle(&mut light, &pix, wu, hu);
    crate::emission::smooth(&mut light, &pix, wu, hu, threads, cancel);
    if cancel() {
        return None;
    }
    if timing {
        let lit = thick.iter().filter(|t| t.is_finite()).count();
        crate::elog!(
            "  subsurface: {} lines ({} through, {} on the card), surfaces {:.2} s, thickness {:.2} s, shadows {:.2} s, all {:.2} s",
            lines.len(),
            lit,
            card1 + card2,
            t1,
            t2 - t1,
            t0.elapsed().as_secs_f64() - t2,
            t0.elapsed().as_secs_f64()
        );
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
        let thin = total(&bulb(&format!("{on}\nmaterial_subsurface_depth = 0.05"))).unwrap();
        let deep = total(&bulb(&format!("{on}\nmaterial_subsurface_depth = 0.2"))).unwrap();
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
