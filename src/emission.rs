//! The light of the shining parts of the surface (not in MB3D): where the
//! surface material lets the surface shine (its glow, or the emission of
//! the palette colours it has), the shining parts are light sources and
//! this pass casts their light on what is around them, for the painting.
//! (Monte Carlo does it by its paths.)
//!
//! The idea of light from parts chosen by palette colour is Mandelbulb3D
//! SE's (its emissive dIFS, by Glenn Olander,
//! github.com/Rodrigo-Spacecraft/Mandelbulb3D-SE), which shoots 64 rays
//! over the hemisphere of every pixel and counts those that meet a shining
//! part.  Nothing of its code is used and the way is another one: the
//! shining parts are known (the picture's own pixels), so every pixel
//! aims a few rays at them instead of searching for them, as the direct
//! light sampling of path tracers does, with the choice among the light
//! samples by resampled importance sampling (Talbot 2005; Bitterli et al.
//! 2020, "Spatiotemporal reservoir resampling", without the reuse in
//! space and time):
//!
//! 1. every shining pixel is a light sample: its place, its normal, its
//!    colour times its emission, and the area of surface the pixel shows;
//! 2. a pixel draws a few candidates among them by their power, keeps one
//!    by what it would give that pixel without anything in between (its
//!    distance and the two angles), and sends one ray to it: the light
//!    counts when the ray arrives;
//! 3. the rays run on the graphics card (the reflection pass's march) or
//!    on the CPU;
//! 4. the light is smoothed where the surface is the same (depth and
//!    normal), not across edges.
//!
//! The shining parts the picture does not show (behind another part, in a
//! gap seen from the side, outside the picture) have no light sample: for
//! them every pixel also sends as many rays into the half space above its
//! surface, searching, and takes the light of a shining part a ray meets
//! when no light sample lies there (so no part counts twice).  Their
//! light is noisier than the aimed rays'.
//!
//! Only when the whole picture is calculated at once (not on tiles).
use crate::calc::{CalcParams, Marcher};
use crate::gbuffer::SiLight;
use crate::lighting::{LightVals, PaintCamera};
use crate::math::*;
use crate::scene::Scene;
use std::sync::Arc;

/// The light the shining parts cast on each pixel of `rect`, as the colour
/// of a light of strength 1 falling straight on the surface.
#[derive(Debug)]
pub(crate) struct EmitLight {
    pub rect: [i32; 4],
    pub light: Vec<[f32; 3]>,
}

impl EmitLight {
    pub(crate) fn at(&self, px: i32, py: i32) -> Option<[f32; 3]> {
        let [x0, y0, w, h] = self.rect;
        let (x, y) = (px - x0, py - y0);
        (x >= 0 && y >= 0 && x < w && y < h).then(|| self.light[(y * w + x) as usize])
    }
}

/// What the pass knows of a pixel's surface.
#[derive(Clone, Copy, Default)]
struct Pix {
    pos: Vec3,
    n: [f32; 3],
    /// the DE stop there (steps) and the depth (steps)
    eps: f32,
    zz: f32,
    /// the width of surface the pixel shows (world)
    width: f32,
    ok: bool,
}

/// A light sample: a shining pixel.
#[derive(Clone, Copy)]
struct Em {
    pos: Vec3,
    n: [f32; 3],
    /// its light (the diffuse colour times the emission)
    l: [f32; 3],
    area: f32,
}

/// A ray of a pixel.  Aimed at the light sample `em`: `v` is the light
/// it brings when it arrives.  Searching (`em` = [`SEARCH`]): `v` is its
/// direction; it brings the light of a shining part it meets that the
/// picture does not show (those have no light sample).
#[derive(Clone, Copy)]
struct Ray {
    pix: u32,
    em: u32,
    v: [f32; 3],
}
const SEARCH: u32 = u32::MAX;

/// At most this many light samples (more shining pixels: every n-th, each
/// standing for n).
const MAX_SAMPLES: usize = 1 << 16;
/// The candidates a ray is chosen from.
const CANDIDATES: usize = 8;
/// The rays of a batch on the card.
#[cfg(feature = "gpu")]
const BATCH: usize = 1 << 19;

fn cross(a: &Vec3, b: &Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn luma(c: [f32; 3]) -> f32 {
    0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
}

/// A random number generator per pixel (the same picture every time).
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }
    fn next(&mut self) -> f64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The settings and the picture's values the light depends on, as a
/// number: the last light is kept and used again while it is the same (a
/// repaint with other lights, fog or finishing).
fn signature(sc: &Scene, p: &CalcParams, gbuf: &[SiLight], lv: &LightVals) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    p.rect.hash(&mut h);
    (p.width, p.height).hash(&mut h);
    if let Some(m) = &sc.material {
        m.glow.to_bits().hash(&mut h);
        m.emission.map(f32::to_bits).hash(&mut h);
        m.cast_rays.hash(&mut h);
    }
    for v in sc.vgrads.iter().flatten().chain(sc.mid.iter()) {
        v.to_bits().hash(&mut h);
    }
    lv.palette_signature().hash(&mut h);
    for si in gbuf {
        (si.zpos_fine, si.si_gradient, si.otrap, si.normal).hash(&mut h);
    }
    h.finish()
}

static LAST: std::sync::Mutex<Option<(u64, Arc<EmitLight>)>> = std::sync::Mutex::new(None);

/// The light of the shining parts on the pixels of the picture, None when
/// nothing shines, no rays are asked for (`Material::cast_rays`), or the
/// picture is not calculated whole.
pub(crate) fn light(
    sc: &Scene,
    p: &CalcParams,
    gbuf: &[SiLight],
    lv: &LightVals,
    cam: &PaintCamera,
    threads: usize,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Option<Arc<EmitLight>> {
    let m = sc.material.as_ref().filter(|m| m.emits() && m.cast_rays > 0)?;
    let [x0, y0, w, h] = p.rect;
    if p.slice_2d != 0 || p.inside_rendering || [x0, y0, w, h] != [0, 0, p.width, p.height] || gbuf.len() != (w * h) as usize {
        return None;
    }
    let sig = signature(sc, p, gbuf, lv);
    if let Some((s, l)) = LAST.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        if *s == sig {
            return Some(l.clone());
        }
    }
    let out = Arc::new(calculate(m.cast_rays as usize, p, gbuf, lv, cam, threads, true, progress, cancel)?);
    *LAST.lock().unwrap_or_else(|e| e.into_inner()) = Some((sig, out.clone()));
    Some(out)
}

/// [`light`]'s calculation: `rays` aimed and as many searching rays per
/// pixel, on the card (`card`, when it is ready for them) or the CPU.
#[allow(clippy::too_many_arguments)]
#[cfg_attr(not(feature = "gpu"), allow(unused_variables, unused_mut))]
fn calculate(
    rays: usize,
    p: &CalcParams,
    gbuf: &[SiLight],
    lv: &LightVals,
    cam: &PaintCamera,
    threads: usize,
    card: bool,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Option<EmitLight> {
    let [x0, y0, w, h] = p.rect;
    let (wu, hu) = (w as usize, h as usize);
    let threads = threads.max(1);
    let timing = std::env::var_os("MB3D_EMISSION_TIMING").is_some();
    let t0 = std::time::Instant::now();

    // 1. the surfaces and the light samples
    let mut pix = vec![Pix::default(); wu * hu];
    let mut shine: Vec<Option<[f32; 3]>> = vec![None; wu * hu];
    let rows = hu.div_ceil(threads).max(1);
    std::thread::scope(|s| {
        for (ci, (pc, sc_)) in pix.chunks_mut(rows * wu).zip(shine.chunks_mut(rows * wu)).enumerate() {
            s.spawn(move || {
                let mut m = Marcher::new(p, 1);
                for (k, (px, sh)) in pc.iter_mut().zip(sc_.iter_mut()).enumerate() {
                    let i = ci * rows * wu + k;
                    if k % wu == 0 && cancel() {
                        return;
                    }
                    let (x, y) = ((i % wu) as i32, (i / wu) as i32);
                    let si = &gbuf[i];
                    if !m.surface_point(x, y, si, 4, 0.01) {
                        continue;
                    }
                    let view = normalize(m.vfov);
                    let mut n = normalize(rotate_vector_reverse(&[si.normal[0] as f64, si.normal[1] as f64, si.normal[2] as f64], &p.vgrads));
                    if !n.iter().all(|v| v.is_finite()) {
                        continue;
                    }
                    if dot(&n, &view) > 0.0 {
                        n = scale(n, -1.0);
                    }
                    let width = p.step_width * (1.0 + m.mzz.abs() * p.de_stop_factor as f64);
                    *px = Pix { pos: m.it.c, n: [n[0] as f32, n[1] as f32, n[2] as f32], eps: m.ms_de_stop.max(1e-6), zz: m.mzz as f32, width: width as f32, ok: true };
                    let plv = lv.plv_for_pixel(si, x + x0, y + y0, cam);
                    let idif0 = lv.s_col_zmul * plv.z_pos;
                    let e = lv.emission_at(si, idif0);
                    if e > 0.0 {
                        let c = lv.calc_colors(si, idif0).0;
                        *sh = Some([c[0] * e, c[1] * e, c[2] * e]);
                    }
                }
            });
        }
    });
    if cancel() {
        return None;
    }
    let lit: Vec<usize> = (0..wu * hu).filter(|&i| shine[i].is_some_and(|l| luma(l) > 0.0)).collect();
    let none = || Some(EmitLight { rect: p.rect, light: vec![[0.0; 3]; wu * hu] });
    if lit.is_empty() {
        return none();
    }
    let every = lit.len().div_ceil(MAX_SAMPLES).max(1);
    let ems: Vec<Em> = lit
        .iter()
        .step_by(every)
        .map(|&i| {
            let px = &pix[i];
            // the surface the pixel shows: wider where it turns away
            let (x, y) = ((i % wu) as i32, (i / wu) as i32);
            let (_, dir) = crate::calc::pixel_ray(p, x, y);
            let cos = dot(&[px.n[0] as f64, px.n[1] as f64, px.n[2] as f64], &normalize(dir)).abs().max(0.2);
            Em { pos: px.pos, n: px.n, l: shine[i].unwrap(), area: (px.width as f64 * px.width as f64 / cos) as f32 * every as f32 }
        })
        .collect();
    // (drawn by their power)
    let mut cdf = Vec::with_capacity(ems.len());
    let mut total = 0f64;
    for e in &ems {
        total += (luma(e.l) * e.area) as f64;
        cdf.push(total);
    }
    if !(total > 0.0) {
        return none();
    }
    if timing {
        crate::elog!("  emission: {} light samples of {} shining pixels, surfaces in {:.2} s", ems.len(), lit.len(), t0.elapsed().as_secs_f64());
    }

    // 2. the rays: for each pixel `rays` of them, each to the sample kept
    // of CANDIDATES drawn by power, weighted by what they would give
    let t1 = std::time::Instant::now();
    let geo = |px: &Pix, e: &Em| -> f32 {
        let d = [(e.pos[0] - px.pos[0]) as f32, (e.pos[1] - px.pos[1]) as f32, (e.pos[2] - px.pos[2]) as f32];
        let d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        if d2 <= 0.0 {
            return 0.0;
        }
        let inv = 1.0 / d2.sqrt();
        let cr = (px.n[0] * d[0] + px.n[1] * d[1] + px.n[2] * d[2]) * inv;
        let ce = -(e.n[0] * d[0] + e.n[1] * d[1] + e.n[2] * d[2]) * inv;
        if cr <= 0.0 || ce <= 0.0 {
            return 0.0;
        }
        // (the area in the divisor: a sample nearer than its own size is a
        // patch, not a point)
        cr * ce / (d2 + e.area)
    };
    let (pix_r, ems_r, cdf_r) = (&pix, &ems, &cdf);
    let mut list: Vec<Ray> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..threads)
            .map(|t| {
                s.spawn(move || {
                    let mut out = Vec::new();
                    let (lo, hi) = (t * rows * wu, ((t + 1) * rows * wu).min(wu * hu));
                    for i in lo..hi.max(lo) {
                        if i % wu == 0 && cancel() {
                            return out;
                        }
                        let px = &pix_r[i];
                        if !px.ok {
                            continue;
                        }
                        let mut rng = Rng::new(i as u64);
                        for _ in 0..rays {
                            let (mut sum, mut kept) = (0f32, None);
                            for _ in 0..CANDIDATES {
                                let u = rng.next() * total;
                                let j = cdf_r.partition_point(|c| *c <= u).min(ems_r.len() - 1);
                                let g = geo(px, &ems_r[j]);
                                // (drawn before the test, so every candidate costs the same numbers)
                                let pick = rng.next() as f32;
                                if g > 0.0 {
                                    sum += g;
                                    if pick * sum < g {
                                        kept = Some(j);
                                    }
                                }
                            }
                            let Some(j) = kept else { continue };
                            // the light of the kept sample over the chance it had:
                            // its colour (at the brightness 1), the power of all
                            // samples and the mean of the candidates' geometry
                            let e = &ems_r[j];
                            let k = total as f32 * sum / CANDIDATES as f32 / (std::f32::consts::PI * luma(e.l)) / rays as f32;
                            out.push(Ray { pix: i as u32, em: j as u32, v: [e.l[0] * k, e.l[1] * k, e.l[2] * k] });
                        }
                        // the searching rays: over the half space above the surface,
                        // more of them towards the normal (as the light counts)
                        let n = [px.n[0] as f64, px.n[1] as f64, px.n[2] as f64];
                        let axis = if n[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [0.0, 1.0, 0.0] };
                        let t1 = normalize(cross(&axis, &n));
                        let t2 = cross(&n, &t1);
                        for k in 0..rays {
                            // (stratified over the turn)
                            let (r2, a) = (rng.next(), std::f64::consts::TAU * (k as f64 + rng.next()) / rays as f64);
                            let r = r2.sqrt();
                            let d = add(scale(n, (1.0 - r2).max(0.0).sqrt()), add(scale(t1, r * a.cos()), scale(t2, r * a.sin())));
                            out.push(Ray { pix: i as u32, em: SEARCH, v: [d[0] as f32, d[1] as f32, d[2] as f32] });
                        }
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    if cancel() {
        return None;
    }
    if timing {
        crate::elog!("  emission: {} rays chosen in {:.2} s", list.len(), t1.elapsed().as_secs_f64());
    }

    // 3. the rays that arrive, and what the searching rays meet
    let t2 = std::time::Instant::now();
    // where the light samples lie (all shining pixels): cells of a few pixel widths
    let cell = {
        let mut ws: Vec<f32> = lit.iter().map(|&i| pix[i].width).collect();
        ws.sort_by(|a, b| a.total_cmp(b));
        (ws[ws.len() / 2] * 3.0).max(1e-30) as f64
    };
    let key = |q: &Vec3| [(q[0] / cell).floor() as i64, (q[1] / cell).floor() as i64, (q[2] / cell).floor() as i64];
    let shown: std::collections::HashSet<[i64; 3]> = lit.iter().map(|&i| key(&pix[i].pos)).collect();
    let covered = |q: &Vec3| -> bool {
        let k = key(q);
        (-1..=1).any(|a| (-1..=1).any(|b| (-1..=1).any(|c| shown.contains(&[k[0] + a, k[1] + b, k[2] + c]))))
    };
    // the light of the surface `si` a searching ray met at `q` (depth `zz` in steps)
    // (counted for MB3D_EMISSION_TIMING: hits, shining hits, of those shown)
    let counts = [const { std::sync::atomic::AtomicUsize::new(0) }; 4];
    let met = |si: &SiLight, q: &Vec3, zz: f64| -> [f32; 3] {
        use std::sync::atomic::Ordering::Relaxed;
        let idif0 = lv.s_col_zmul * (zz.abs() * p.step_width + p.zz_stmit_dif) as f32;
        let e = lv.emission_at(si, idif0);
        if timing {
            counts[0].fetch_add(1, Relaxed);
            counts[3].fetch_add(si.si_gradient as usize, Relaxed);
            if e > 0.0 {
                counts[1].fetch_add(1, Relaxed);
                if covered(q) {
                    counts[2].fetch_add(1, Relaxed);
                }
            }
        }
        if e <= 0.0 || covered(q) {
            return [0.0; 3];
        }
        let c = lv.calc_colors(si, idif0).0;
        let k = e / rays as f32;
        [c[0] * k, c[1] * k, c[2] * k]
    };
    // a ray: its start, its direction, its length (a searching ray: none) and
    // how near the end counts as arrived
    let seg = |r: &Ray| -> (Vec3, Vec3, f64, f64) {
        let px = &pix[r.pix as usize];
        let n = [px.n[0] as f64, px.n[1] as f64, px.n[2] as f64];
        let o = add(px.pos, scale(n, 2.0 * px.eps as f64 * p.step_width));
        if r.em == SEARCH {
            return (o, [r.v[0] as f64, r.v[1] as f64, r.v[2] as f64], f64::INFINITY, 0.0);
        }
        let e = &ems[r.em as usize];
        let d = sub(e.pos, o);
        let len = dot(&d, &d).sqrt();
        // the sample lies on the surface: the ray ends in it
        let tol = 4.0 * px.eps as f64 * p.step_width + 2.0 * (e.area as f64).sqrt();
        (o, scale(d, 1.0 / len.max(1e-300)), len, tol)
    };
    let novis = std::env::var_os("MB3D_EMISSION_NOVIS").is_some();
    // what each ray brings
    let mut got = vec![[0f32; 3]; list.len()];
    let mut done = 0usize;
    #[cfg(feature = "gpu")]
    // (on the card when its shader for these rays is compiled: a scene new
    // to the card compiles it meanwhile, for the next time; the CPU's
    // threads take about as long as the card for this pass)
    if card && crate::gpu::enabled() && crate::gpu::unsupported(p).is_none() && std::env::var_os("MB3D_EMISSION_CPU").is_none() && crate::gpu::reflect_compiled(p) {
        while done < list.len() {
            let part = &list[done..(done + BATCH).min(list.len())];
            let tasks: Vec<crate::gpu::ReflectTask> = part
                .iter()
                .map(|r| {
                    let (o, d, _, _) = seg(r);
                    let sw = p.step_width;
                    crate::gpu::ReflectTask { origin: o, dir: [(d[0] * sw) as f32, (d[1] * sw) as f32, (d[2] * sw) as f32], zz: pix[r.pix as usize].zz, pixel: r.pix, ..Default::default() }
                })
                .collect();
            match crate::gpu::reflect_batch(p, &tasks, &crate::gpu::ReflectOpts::default(), None, cancel) {
                Ok((hits, _, unfinished)) if unfinished.is_empty() && hits.len() == tasks.len() => {
                    for (k, (r, hit)) in part.iter().zip(&hits).enumerate() {
                        if r.em == SEARCH {
                            if hit.hit {
                                got[done + k] = met(&hit.si, &hit.pos, hit.zz as f64);
                            }
                            continue;
                        }
                        let (o, _, len, tol) = seg(r);
                        let d = sub(hit.pos, o);
                        if novis || !hit.hit || dot(&d, &d).sqrt() >= len - tol {
                            got[done + k] = r.v;
                        }
                    }
                    done += part.len();
                    progress(done, list.len());
                }
                // (the card gave up: the rest on the CPU)
                Ok(_) => break,
                Err(e) => {
                    if e == "cancelled" {
                        return None;
                    }
                    if timing {
                        crate::elog!("  emission: the card gave up ({e}), the rays on the CPU");
                    }
                    break;
                }
            }
        }
    }
    let on_card = done;
    if done < list.len() {
        let rest = &list[done..];
        let per = rest.len().div_ceil(threads).max(1);
        let count = std::sync::atomic::AtomicUsize::new(done);
        let n_all = list.len();
        std::thread::scope(|s| {
            for (part, out) in rest.chunks(per).zip(got[done..].chunks_mut(per)) {
                let (seg, count, pix, met) = (&seg, &count, &pix, &met);
                s.spawn(move || {
                    let mut m = Marcher::new(p, 1);
                    for (k, (r, v)) in part.iter().zip(out.iter_mut()).enumerate() {
                        if k % 256 == 0 {
                            if cancel() {
                                return;
                            }
                            progress(count.fetch_add(256.min(part.len() - k), std::sync::atomic::Ordering::Relaxed), n_all);
                        }
                        let (o, d, len, tol) = seg(r);
                        let px = &pix[r.pix as usize];
                        let (si, went) = m.march_ray_from(o, scale(d, p.step_width), px.zz as f64);
                        let hit = si.zpos() < 32768;
                        if r.em == SEARCH {
                            if hit {
                                *v = met(&si, &m.it.c, m.mzz);
                            }
                        } else if novis || !hit || went * p.step_width >= len - tol {
                            *v = r.v;
                        }
                    }
                });
            }
        });
        if cancel() {
            return None;
        }
    }
    if timing {
        // the pixel width against the distance of neighbouring pixels on one surface
        let mut r: Vec<f32> = (0..wu * hu - 1)
            .filter(|i| pix[*i].ok && pix[i + 1].ok && (i + 1) % wu != 0)
            .filter(|i| pix[*i].n.iter().zip(pix[i + 1].n).map(|(a, b)| a * b).sum::<f32>() > 0.95)
            .map(|i| (dot(&sub(pix[i + 1].pos, pix[i].pos), &sub(pix[i + 1].pos, pix[i].pos)).sqrt() as f32) / pix[i].width)
            .collect();
        r.sort_by(|a, b| a.total_cmp(b));
        if !r.is_empty() {
            crate::elog!("  emission: neighbour distance / pixel width: median {:.2}, 10 % {:.2}, 90 % {:.2}", r[r.len() / 2], r[r.len() / 10], r[r.len() * 9 / 10]);
        }
        let mean_l = ems.iter().map(|e| luma(e.l) as f64).sum::<f64>() / ems.len() as f64;
        let area: f64 = ems.iter().map(|e| e.area as f64).sum();
        crate::elog!("  emission: samples' mean brightness {:.3}, area in all {:.4e} (step width {:.3e}, a pixel {:.3e})", mean_l, area, p.step_width, pix.iter().find(|p| p.ok).map_or(0.0, |p| p.width));
    }
    let mut light = vec![[0f32; 3]; wu * hu];
    for (r, v) in list.iter().zip(&got) {
        let l = &mut light[r.pix as usize];
        for c in 0..3 {
            l[c] += v[c];
        }
    }
    if timing {
        let aimed = list.iter().filter(|r| r.em != SEARCH).count();
        let n = list.iter().zip(&got).filter(|(r, v)| r.em != SEARCH && luma(**v) > 0.0).count();
        let found = list.iter().zip(&got).filter(|(r, v)| r.em == SEARCH && luma(**v) > 0.0).count();
        let c = counts.each_ref().map(|c| c.load(std::sync::atomic::Ordering::Relaxed));
        crate::elog!("  emission: searching rays: {} hit a surface (mean colour value {}), {} of those shine, {} of those are shown", c[0], c[3] / c[0].max(1), c[1], c[2]);
        crate::elog!("  emission: {n} of {aimed} aimed rays arrive, {found} of {} searching rays meet a hidden shining part ({on_card} rays on the card), {:.2} s", list.len() - aimed, t2.elapsed().as_secs_f64());
    }
    list.clear();
    list.shrink_to_fit();
    if timing {
        let mut v: Vec<f32> = light.iter().zip(&pix).filter(|(_, p)| p.ok).map(|(l, _)| luma(*l)).collect();
        v.sort_by(|a, b| a.total_cmp(b));
        if !v.is_empty() {
            crate::elog!("  emission: light on the surfaces: median {:.4}, 90 % {:.4}, 99 % {:.4}, most {:.3}", v[v.len() / 2], v[v.len() * 9 / 10], v[v.len() * 99 / 100], v[v.len() - 1]);
        }
    }

    // 4. smoothed along the surface
    let t3 = std::time::Instant::now();
    if std::env::var_os("MB3D_EMISSION_RAW").is_none() {
        smooth(&mut light, &pix, wu, hu, threads, cancel);
        if cancel() {
            return None;
        }
    }
    if timing {
        crate::elog!("  emission: smoothed in {:.2} s, all {:.2} s", t3.elapsed().as_secs_f64(), t0.elapsed().as_secs_f64());
    }
    Some(EmitLight { rect: p.rect, light })
}

/// The light smoothed over the pixels around that show the same surface
/// (near in depth for their distance, the normal nearly the same): three
/// rounds of a 5x5 filter with growing holes (the "à trous" wavelet filter
/// of Dammertz et al. 2010), so the noise of the few rays goes and the
/// edges of the light and of the surface stay.
fn smooth(light: &mut Vec<[f32; 3]>, pix: &[Pix], w: usize, h: usize, threads: usize, cancel: &(dyn Fn() -> bool + Sync)) {
    const K: [f32; 3] = [3.0 / 8.0, 1.0 / 4.0, 1.0 / 16.0];
    let rows = h.div_ceil(threads.max(1)).max(1);
    for round in 0..4 {
        let step = 1isize << round;
        let src = &*light;
        let mut dst = vec![[0f32; 3]; w * h];
        std::thread::scope(|s| {
            for (ci, out) in dst.chunks_mut(rows * w).enumerate() {
                s.spawn(move || {
                    for (k, o) in out.iter_mut().enumerate() {
                        let i = ci * rows * w + k;
                        if k % w == 0 && cancel() {
                            return;
                        }
                        let a = &pix[i];
                        if !a.ok {
                            continue;
                        }
                        let (x, y) = ((i % w) as isize, (i / w) as isize);
                        let (mut sum, mut wsum) = ([0f32; 3], 0f32);
                        for dy in -2..=2isize {
                            for dx in -2..=2isize {
                                let (xx, yy) = (x + dx * step, y + dy * step);
                                if xx < 0 || yy < 0 || xx >= w as isize || yy >= h as isize {
                                    continue;
                                }
                                let j = yy as usize * w + xx as usize;
                                let b = &pix[j];
                                if !b.ok {
                                    continue;
                                }
                                let nn = a.n[0] * b.n[0] + a.n[1] * b.n[1] + a.n[2] * b.n[2];
                                if nn < 0.7 {
                                    continue;
                                }
                                // off the plane of the pixel's surface, in pixel widths
                                let d = [(b.pos[0] - a.pos[0]) as f32, (b.pos[1] - a.pos[1]) as f32, (b.pos[2] - a.pos[2]) as f32];
                                let off = (a.n[0] * d[0] + a.n[1] * d[1] + a.n[2] * d[2]).abs() / (a.width * (dx.abs().max(dy.abs()) * step) as f32).max(1e-30);
                                if off > 1.5 {
                                    continue;
                                }
                                let wt = K[dx.unsigned_abs()] * K[dy.unsigned_abs()] * nn.powi(8) * (1.0 - off / 1.5);
                                let l = src[j];
                                sum = [sum[0] + l[0] * wt, sum[1] + l[1] * wt, sum[2] + l[2] * wt];
                                wsum += wt;
                            }
                        }
                        *o = if wsum > 0.0 { [sum[0] / wsum, sum[1] / wsum, sum[2] / wsum] } else { src[i] };
                    }
                });
            }
        });
        if cancel() {
            return;
        }
        *light = dst;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_light_of_a_pixel_is_looked_up_in_its_rect() {
        let l = EmitLight { rect: [10, 20, 2, 2], light: vec![[1.0; 3], [2.0; 3], [3.0; 3], [4.0; 3]] };
        assert_eq!(l.at(11, 21), Some([4.0; 3]));
        assert_eq!(l.at(10, 20), Some([1.0; 3]));
        assert_eq!(l.at(9, 20), None);
        assert_eq!(l.at(12, 20), None);
    }

    #[test]
    fn smoothing_keeps_the_light_and_the_edges() {
        // a flat wall of 8x8 pixels, one bright pixel: its light spreads,
        // the sum stays; a second wall behind it (far off the plane) gets none
        let (w, h) = (16usize, 8usize);
        let mut pix = vec![Pix::default(); w * h];
        for (i, p) in pix.iter_mut().enumerate() {
            let (x, y) = (i % w, i / w);
            let z = if x < 8 { 0.0 } else { 100.0 };
            *p = Pix { pos: [x as f64, y as f64, z], n: [0.0, 0.0, -1.0], eps: 1.0, zz: 0.0, width: 1.0, ok: true };
        }
        let mut light = vec![[0f32; 3]; w * h];
        light[3 * w + 3] = [64.0; 3];
        smooth(&mut light, &pix, w, h, 2, &|| false);
        assert!(light[3 * w + 3][0] < 64.0 && light[3 * w + 4][0] > 0.0, "the light spreads on its wall");
        assert!((0..h).all(|y| (8..w).all(|x| light[y * w + x][0] == 0.0)), "none crosses to the wall behind");
        let sum: f32 = light.iter().map(|l| l[0]).sum();
        assert!(sum > 32.0 && sum < 128.0, "about the same light in all: {sum}");
    }

    /// The default bulb without lights, its first palette colour shining:
    /// the shining parts light the surfaces next to them, the same way
    /// every time, and more rays change the light only a little.
    #[test]
    fn the_shining_parts_light_what_is_around_them() {
        let text = "width = 120\nheight = 90\nlights = none\nmaterial_emission = 40, 0, 0, 0, 0, 0, 0, 0, 0, 0\nmaterial_light_rays = 8\n";
        let sc = Scene::parse(text).unwrap();
        let (p, gbuf) = crate::render::calculate(&sc, &|_, _| {}).unwrap();
        let cam = crate::render::paint_camera(&sc, &p);
        let lv = crate::render::light_vals(&sc, &sc.lighting, &cam);
        let run = |rays: usize| calculate(rays, &p, &gbuf, &lv, &cam, 4, false, &|_, _| {}, &|| false).unwrap();
        let a = run(8);
        assert_eq!(a.rect, p.rect);
        // the shining pixels and the others on the surface
        let shines: Vec<bool> = gbuf
            .iter()
            .enumerate()
            .map(|(i, si)| {
                let plv = lv.plv_for_pixel(si, (i % 120) as i32, (i / 120) as i32, &cam);
                si.zpos() < 32768 && lv.emission_at(si, lv.s_col_zmul * plv.z_pos) > 0.0
            })
            .collect();
        let n_shine = shines.iter().filter(|s| **s).count();
        assert!(n_shine > 50, "the first colour is on the bulb: {n_shine} pixels");
        let lit = a.light.iter().zip(&shines).zip(&gbuf).filter(|((l, s), si)| !**s && si.zpos() < 32768 && luma(**l) > 0.01).count();
        assert!(lit > n_shine / 2, "surfaces that do not shine get light: {lit}");
        // the background gets none
        assert!(a.light.iter().zip(&gbuf).all(|(l, si)| si.zpos() < 32768 || *l == [0.0; 3]));
        // the same every time
        assert_eq!(a.light, run(8).light);
        // four times the rays: about the same light in all
        let sum = |l: &EmitLight| l.light.iter().map(|v| luma(*v) as f64).sum::<f64>();
        let (s8, s32) = (sum(&a), sum(&run(32)));
        assert!((s8 / s32 - 1.0).abs() < 0.15, "8 rays {s8:.1}, 32 rays {s32:.1}");
    }

    #[test]
    fn the_random_numbers_are_the_same_every_time_and_even() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        let mut mean = 0.0;
        for _ in 0..10000 {
            let v = a.next();
            assert_eq!(v, b.next());
            assert!((0.0..1.0).contains(&v));
            mean += v / 10000.0;
        }
        assert!((mean - 0.5).abs() < 0.02, "{mean}");
    }
}
