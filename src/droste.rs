//! The Droste effect (not in MB3D): the picture repeated in itself, as
//! Escher's "Print Gallery" and the Droste cocoa tin.  The ring between two
//! radii around a centre is the only part of the picture kept; in log-polar
//! coordinates (the logarithm of the distance, the angle) that ring is a
//! rectangle, and repeating the rectangle along the distance axis fills the
//! plane with ever smaller copies towards the centre and ever larger ones
//! outwards.  Escher's twist rotates the rectangle before it is repeated so
//! that one shrink step lines up with one turn of the angle: the copies
//! then join along a spiral and no seam is left as a circle.  The phase
//! zooms through one shrink step, and the picture at phase 1 is the one at
//! phase 0, so an animation of the phase from 0 to 1 is an endless zoom.
//!
//! The mathematics is Bart de Smit's and Hendrik Lenstra's ("The
//! mathematical structure of Escher's Print Gallery", 2003), who completed
//! Escher's unfinished middle; Jos Leys, Josh Sommers (the MathMap script)
//! and Tom Beddard (the Pixel Bender filter) made it a picture filter and
//! gave it the controls used here.  Runs in the finishing pass over the
//! whole picture, after the other finishing (`render::paint_with`).

use std::f64::consts::PI;

/// The Droste effect's settings (`Scene::droste`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Droste {
    /// the centre as parts of the picture's width and height (0.5, 0.5 the
    /// middle)
    pub center: [f32; 2],
    /// the inner radius of the ring kept, as a part of the picture's
    /// shorter side
    pub inner: f32,
    /// the outer radius of the ring kept, as a part of the shorter side
    pub outer: f32,
    /// the turns of the spiral per shrink step: 0 concentric copies, 1
    /// Escher's spiral, 2 a steeper one, negative the other way round
    pub twist: f32,
    /// the zoom through shrink steps: 1 looks as 0 (an animation from 0 to
    /// 1 is an endless zoom)
    pub phase: f32,
    /// 0..1: the part of a step over which neighbouring copies crossfade
    /// (0: hard edges where the ring's inner edge meets its outer one)
    pub blend: f32,
}

impl Default for Droste {
    fn default() -> Self {
        Droste { center: [0.5, 0.5], inner: 0.12, outer: 0.45, twist: 1.0, phase: 0.0, blend: 0.35 }
    }
}

impl Droste {
    /// The settings within their ranges (the outer radius above the inner).
    pub fn clamped(mut self) -> Droste {
        self.center = [self.center[0].clamp(-1.0, 2.0), self.center[1].clamp(-1.0, 2.0)];
        self.inner = self.inner.clamp(0.001, 2.0);
        self.outer = self.outer.clamp(0.002, 4.0).max(self.inner * 1.05);
        self.twist = self.twist.clamp(-8.0, 8.0);
        self.phase = self.phase.clamp(-1000.0, 1000.0);
        self.blend = self.blend.clamp(0.0, 1.0);
        self
    }

    /// The scene keys of these settings (`Scene::extra_text`).
    pub fn to_text(&self) -> String {
        format!(
            "droste = {}, {}, {}, {}\ndroste_twist = {}\ndroste_phase = {}\ndroste_blend = {}\n",
            self.center[0], self.center[1], self.inner, self.outer, self.twist, self.phase, self.blend
        )
    }
}

#[inline]
fn smoothstep(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The source picture read between its pixels.
struct Source<'a> {
    img: &'a [f32],
    w: usize,
    h: usize,
}

impl Source<'_> {
    /// The colour at (x, y) in pixel units (pixel centres at .5), None
    /// outside the picture.
    #[inline]
    fn at(&self, x: f64, y: f64) -> Option<[f32; 3]> {
        let (x, y) = (x - 0.5, y - 0.5);
        if x < -0.5 || y < -0.5 || x > self.w as f64 - 0.5 || y > self.h as f64 - 0.5 {
            return None;
        }
        let x = x.clamp(0.0, (self.w - 1) as f64);
        let y = y.clamp(0.0, (self.h - 1) as f64);
        let (x0, y0) = (x as usize, y as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (fx, fy) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);
        let p = |xx: usize, yy: usize| {
            let o = (yy * self.w + xx) * 3;
            [self.img[o], self.img[o + 1], self.img[o + 2]]
        };
        let (a, b, c, d) = (p(x0, y0), p(x1, y0), p(x0, y1), p(x1, y1));
        let mut out = [0f32; 3];
        for k in 0..3 {
            let top = a[k] + (b[k] - a[k]) * fx;
            let bottom = c[k] + (d[k] - c[k]) * fx;
            out[k] = top + (bottom - top) * fy;
        }
        Some(out)
    }
}

/// The map of one picture: everything that does not depend on the pixel.
struct Map {
    cx: f64,
    cy: f64,
    r1: f64,
    /// the shrink step: ln(outer / inner)
    lr: f64,
    /// 1 / alpha, alpha = 1 + twist * lr / (2 pi i) (Escher's rotation of
    /// the strip; 1 for concentric copies)
    inv_alpha: (f64, f64),
    /// |1 / alpha|
    inv_alpha_abs: f64,
    phase: f64,
    blend: f64,
}

impl Map {
    fn new(d: &Droste, w: usize, h: usize) -> Map {
        let d = d.clamped();
        let size = w.min(h) as f64;
        let r1 = d.inner as f64 * size;
        let r2 = d.outer as f64 * size;
        let lr = (r2 / r1).ln();
        // alpha = 1 - i * twist * lr / (2 pi)
        let (ar, ai) = (1.0, -(d.twist as f64) * lr / (2.0 * PI));
        let n = ar * ar + ai * ai;
        let inv_alpha = (ar / n, -ai / n);
        Map {
            cx: d.center[0] as f64 * w as f64,
            cy: d.center[1] as f64 * h as f64,
            r1,
            lr,
            inv_alpha,
            inv_alpha_abs: (inv_alpha.0 * inv_alpha.0 + inv_alpha.1 * inv_alpha.1).sqrt(),
            phase: d.phase as f64,
            blend: d.blend as f64,
        }
    }

    /// The log-polar point of the output point (x, y) in the source's
    /// strip, before the wrap: (log of the distance over the inner radius,
    /// the angle), and the magnification of the source there (source
    /// pixels per output pixel at the ring's inner edge).
    #[inline]
    fn log_polar(&self, x: f64, y: f64) -> (f64, f64, f64) {
        let (dx, dy) = (x - self.cx, y - self.cy);
        let r = (dx * dx + dy * dy).sqrt().max(1e-9);
        let (lre, lim) = ((r / self.r1).ln(), dy.atan2(dx));
        // w / alpha
        let re = lre * self.inv_alpha.0 - lim * self.inv_alpha.1;
        let im = lre * self.inv_alpha.1 + lim * self.inv_alpha.0;
        (re - self.phase * self.lr, im, self.r1 / r * self.inv_alpha_abs)
    }

    /// The source point of the strip point (re, im).
    #[inline]
    fn source(&self, re: f64, im: f64) -> (f64, f64) {
        let r = self.r1 * re.exp();
        let (s, c) = im.sin_cos();
        (self.cx + r * c, self.cy + r * s)
    }
}

/// Applies the effect to `img` (RGB in floating point, `w` x `h`, the whole
/// picture) in place; `threads` share the rows.  Each pixel is read at
/// four points; the copies towards the centre fade into the ring's mean
/// colour where a pixel of them would hold more than a few source pixels.
pub fn apply(img: &mut [f32], w: usize, h: usize, d: &Droste, threads: usize) {
    if w == 0 || h == 0 || img.len() < w * h * 3 {
        return;
    }
    let src_px = img[..w * h * 3].to_vec();
    let src = Source { img: &src_px, w, h };
    let map = Map::new(d, w, h);
    let mean = ring_mean(&src, &map);
    let threads = threads.clamp(1, h);
    let rows = h.div_ceil(threads);
    std::thread::scope(|s| {
        for (k, part) in img[..w * h * 3].chunks_mut(rows * w * 3).enumerate() {
            let (src, map) = (&src, &map);
            s.spawn(move || {
                for (i, px) in part.chunks_exact_mut(3).enumerate() {
                    let (x, y) = ((i % w) as f64, (k * rows + i / w) as f64);
                    let mut acc = [0f32; 3];
                    for (sx, sy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                        let c = sample(src, map, x + sx, y + sy, mean);
                        for (a, v) in acc.iter_mut().zip(c) {
                            *a += v * 0.25;
                        }
                    }
                    px.copy_from_slice(&acc);
                }
            });
        }
    });
}

/// The colour of the output point (x, y): the ring's point one or more
/// shrink steps away, neighbouring copies crossfaded, and the copies that
/// shrink below the pixels faded into the ring's mean colour.
#[inline]
fn sample(src: &Source, map: &Map, x: f64, y: f64, mean: [f32; 3]) -> [f32; 3] {
    let (re, im, mag_inner) = map.log_polar(x, y);
    let wrapped = re.rem_euclid(map.lr);
    let read = |re: f64| -> [f32; 3] {
        let (sx, sy) = map.source(re, im);
        // a source point outside the picture: the copy one step inward
        src.at(sx, sy).or_else(|| {
            let (sx, sy) = map.source(re - map.lr, im);
            src.at(sx, sy)
        }).unwrap_or(mean)
    };
    let mut c = read(wrapped);
    if map.blend > 0.0 {
        let m = smoothstep(wrapped / (map.blend * map.lr)) as f32;
        if m < 1.0 {
            let o = read(wrapped + map.lr);
            for k in 0..3 {
                c[k] = c[k] * m + o[k] * (1.0 - m);
            }
        }
    }
    // the magnification at this source point: source pixels per output pixel
    let mag = mag_inner * wrapped.exp();
    if mag > 3.0 {
        let f = smoothstep((mag - 3.0) / 9.0) as f32;
        for k in 0..3 {
            c[k] = c[k] * (1.0 - f) + mean[k] * f;
        }
    }
    c
}

/// The mean colour of the ring kept (what the smallest copies fade into).
fn ring_mean(src: &Source, map: &Map) -> [f32; 3] {
    let mut acc = [0f64; 3];
    let mut n = 0.0;
    for step in 0..48 {
        let re = (step as f64 + 0.5) / 48.0 * map.lr;
        for a in 0..64 {
            let im = (a as f64 + 0.5) / 64.0 * 2.0 * PI;
            let (sx, sy) = map.source(re, im);
            if let Some(c) = src.at(sx, sy) {
                for k in 0..3 {
                    acc[k] += c[k] as f64;
                }
                n += 1.0;
            }
        }
    }
    if n == 0.0 {
        return [0.0; 3];
    }
    acc.map(|v| (v / n) as f32)
}

/// [`apply`] on a whole 8 bit RGB picture (Monte Carlo's).
pub fn apply_rgb8(rgb: &mut [u8], w: usize, h: usize, d: Option<&Droste>) {
    let Some(d) = d else { return };
    let mut v: Vec<f32> = rgb.iter().map(|&c| c as f32 / 255.0).collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    apply(&mut v, w, h, d, threads);
    for (o, x) in rgb.iter_mut().zip(v) {
        *o = (x * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture whose colour is a smooth function of the place, so that
    /// a resampled copy of it can be checked.
    fn picture(w: usize, h: usize) -> Vec<f32> {
        let mut v = vec![0f32; w * h * 3];
        for y in 0..h {
            for x in 0..w {
                let o = (y * w + x) * 3;
                v[o] = x as f32 / w as f32;
                v[o + 1] = y as f32 / h as f32;
                v[o + 2] = 0.5;
            }
        }
        v
    }

    #[test]
    fn the_ring_itself_stays_without_twist_and_blend() {
        let (w, h) = (160, 120);
        let img = picture(w, h);
        let d = Droste { twist: 0.0, blend: 0.0, inner: 0.2, outer: 0.45, ..Default::default() };
        let mut out = img.clone();
        apply(&mut out, w, h, &d, 3);
        // a pixel in the ring (at 0.3 of the shorter side from the middle)
        let (x, y) = (80 + 36, 60);
        let o = (y * w + x) * 3;
        for k in 0..3 {
            assert!((out[o + k] - img[o + k]).abs() < 0.02, "{k}: {} vs {}", out[o + k], img[o + k]);
        }
        // a pixel inside the inner radius changed into a copy of the ring:
        // the colour of the point one shrink step outward in the same
        // direction (20 px * 2.25 = 45 px; the inner radius is 24 px)
        let (x, y) = (80 + 20, 60);
        let o = (y * w + x) * 3;
        let src_x = 80.0 + 45.0;
        assert!((out[o] - src_x as f32 / w as f32).abs() < 0.03, "{}", out[o]);
        assert!((out[o + 1] - 0.5).abs() < 0.03, "{}", out[o + 1]);
    }

    #[test]
    fn phase_one_is_phase_zero_and_the_twist_moves_the_copies() {
        let (w, h) = (96, 96);
        let img = picture(w, h);
        let d = Droste::default();
        let mut a = img.clone();
        apply(&mut a, w, h, &d, 2);
        let mut b = img.clone();
        apply(&mut b, w, h, &Droste { phase: 1.0, ..d }, 2);
        for (x, y) in a.iter().zip(&b) {
            assert!((x - y).abs() < 1e-5);
        }
        let mut c = img.clone();
        apply(&mut c, w, h, &Droste { phase: 0.5, ..d }, 2);
        assert!(a.iter().zip(&c).any(|(x, y)| (x - y).abs() > 0.05));
        let mut t = img.clone();
        apply(&mut t, w, h, &Droste { twist: 0.0, ..d }, 2);
        assert!(a.iter().zip(&t).any(|(x, y)| (x - y).abs() > 0.05));
        // a different thread count gives the same picture
        let mut e = img.clone();
        apply(&mut e, w, h, &d, 7);
        assert_eq!(a, e);
    }

    #[test]
    fn settings_are_clamped_and_written() {
        let d = Droste { inner: 0.5, outer: 0.3, phase: 1.25, twist: 20.0, ..Default::default() }.clamped();
        assert!(d.outer > d.inner);
        assert_eq!(d.phase, 1.25);
        assert_eq!(d.twist, 8.0);
        let t = Droste::default().to_text();
        assert!(t.starts_with("droste = 0.5, 0.5, 0.12, 0.45\n"));
        assert!(t.contains("droste_twist = 1\n") && t.contains("droste_phase = 0\n") && t.contains("droste_blend = 0.35\n"));
        let mut rgb = vec![128u8; 32 * 32 * 3];
        apply_rgb8(&mut rgb, 32, 32, None);
        assert!(rgb.iter().all(|&v| v == 128));
        apply_rgb8(&mut rgb, 32, 32, Some(&Droste::default()));
        // a flat picture stays flat
        assert!(rgb.iter().all(|&v| (v as i32 - 128).abs() <= 1));
    }
}
