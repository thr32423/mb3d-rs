//! The noise doctor (not in MB3D): black specks and holes in a render come
//! from the march (a ray step too long for the formula, a step limiter too
//! loose), and the group's folk wisdom about which setting cures them
//! contradicts itself from post to post.  The doctor renders a crop of the
//! picture with the main pass alone under a few step settings, counts the
//! specks in each and says which setting had the fewest and what it cost,
//! so the user picks with numbers instead of guesses.

use crate::scene::Scene;

/// One setting tried and what it gave.
#[derive(Clone, Debug, PartialEq)]
pub struct Trial {
    /// what was changed, in words ("as set", "ray step 0.25", ...)
    pub what: String,
    pub raystep: f64,
    pub limiter: f64,
    /// specks and holes per thousand pixels of the crop
    pub specks_per_k: f64,
    pub seconds: f64,
}

/// Specks in a picture: pixels far darker or brighter than every one of
/// their eight neighbours (a dot on a surface, however detailed the
/// surface: a textured surface varies between neighbours too, a speck
/// stands out from all of them), and holes: a background pixel among
/// object pixels or the other way round, where `object` says which pixels
/// hit the surface.  Returns the count.
pub fn specks(rgb: &[u8], object: &[bool], w: usize, h: usize) -> usize {
    if w < 3 || h < 3 {
        return 0;
    }
    let luma = |i: usize| (rgb[i * 3] as i32 * 299 + rgb[i * 3 + 1] as i32 * 587 + rgb[i * 3 + 2] as i32 * 114) / 1000;
    let mut n = 0;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = y * w + x;
            let mut around = [0i32; 8];
            let mut obj_around = 0;
            let mut k = 0;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let j = ((y as i32 + dy) as usize) * w + (x as i32 + dx) as usize;
                    around[k] = luma(j);
                    obj_around += object[j] as i32;
                    k += 1;
                }
            }
            // a hole: alone of its kind among its neighbours
            if (object[i] && obj_around <= 1) || (!object[i] && obj_around >= 7) {
                n += 1;
                continue;
            }
            if !object[i] {
                continue;
            }
            let (lo, hi) = around.iter().fold((255, 0), |(lo, hi), &v| (lo.min(v), hi.max(v)));
            let l = luma(i);
            if l < lo - 48 || l > hi + 48 {
                n += 1;
            }
        }
    }
    n
}

/// The crop of the picture the doctor looks at: the middle, up to 256 x
/// 192 pixels.
pub fn crop(sc: &Scene) -> [i32; 4] {
    let w = sc.width.clamp(1, 256);
    let h = sc.height.clamp(1, 192);
    [(sc.width - w) / 2, (sc.height - h) / 2, w, h]
}

/// The scene reduced to its main pass over the crop: no shadows, no
/// occlusion, no reflections, no depth of field, no volumetric light.
fn main_pass_only(sc: &Scene) -> Scene {
    let mut s = sc.clone();
    s.calc_rect = Some(crop(sc));
    s.shadows = None;
    s.ao = None;
    s.deao = None;
    s.dof = None;
    s.vol_light = None;
    s.mc.reflections = false;
    s.clouds = None;
    s
}

/// The settings the doctor tries: the scene's own, a finer ray step, and
/// a tighter step limiter (or a finer step still when the limiter is
/// tight already).
pub fn settings(sc: &Scene) -> Vec<(String, f64, f64)> {
    let (rs, lim) = (sc.z_step_div, sc.raystep_limiter);
    let mut v = vec![("as set".to_string(), rs, lim), (format!("ray step {}", trim(rs * 0.5)), rs * 0.5, lim)];
    if lim > 0.3 {
        v.push((format!("step limiter {}", trim(0.1f64.max(lim * 0.25))), rs, 0.1f64.max(lim * 0.25)));
    } else {
        v.push((format!("ray step {}", trim(rs * 0.25)), rs * 0.25, lim));
    }
    v
}

fn trim(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Renders the crop with each setting (on the processor, so the trials
/// compare) and counts the specks.
pub fn diagnose(sc: &Scene, cancel: &(dyn Fn() -> bool + Sync)) -> Result<Vec<Trial>, String> {
    let base = main_pass_only(sc);
    let mut out = Vec::new();
    for (what, raystep, limiter) in settings(sc) {
        if cancel() {
            return Err("cancelled".into());
        }
        let mut s = base.clone();
        s.z_step_div = raystep;
        s.raystep_limiter = limiter;
        let t = std::time::Instant::now();
        let (p, g) = crate::render::calculate_raw_cancellable(&s, &|_, _| {}, cancel)?;
        let secs = t.elapsed().as_secs_f64();
        let rgb = crate::render::paint(&s, &p, &g);
        let (w, h) = (p.rect[2] as usize, p.rect[3] as usize);
        let object: Vec<bool> = g.iter().map(|p| !p.is_background()).collect();
        let n = specks(&rgb, &object, w, h);
        out.push(Trial { what, raystep, limiter, specks_per_k: n as f64 * 1000.0 / (w * h).max(1) as f64, seconds: secs });
    }
    Ok(out)
}

/// The trial to recommend: the fewest specks, among equals (within a
/// twentieth of a speck per thousand pixels) the fastest; the scene's own
/// setting unless another is better.
pub fn best(trials: &[Trial]) -> usize {
    let mut b = 0;
    for (i, t) in trials.iter().enumerate().skip(1) {
        let cur = &trials[b];
        if t.specks_per_k + 0.05 < cur.specks_per_k || ((t.specks_per_k - cur.specks_per_k).abs() <= 0.06 && t.seconds < cur.seconds * 0.9 && i != 0) {
            b = i;
        }
    }
    b
}

/// The report in words.
pub fn report(trials: &[Trial]) -> String {
    let b = best(trials);
    let mut lines: Vec<String> = trials
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{}{}: {:.1} specks per 1000 pixels, {:.2} s", if i == b { "> " } else { "  " }, t.what, t.specks_per_k, t.seconds))
        .collect();
    lines.push(if b == 0 {
        "The settings as they are give the fewest specks; keep them.".to_string()
    } else {
        format!("Recommended: {} ({:.1} instead of {:.1} specks per 1000 pixels).", trials[b].what, trials[b].specks_per_k, trials[0].specks_per_k)
    });
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_specks_and_holes() {
        let (w, h) = (8usize, 8usize);
        let mut rgb = vec![128u8; w * h * 3];
        let mut obj = vec![true; w * h];
        assert_eq!(specks(&rgb, &obj, w, h), 0);
        // a black dot on the surface
        rgb[(3 * w + 3) * 3..(3 * w + 3) * 3 + 3].copy_from_slice(&[0, 0, 0]);
        assert_eq!(specks(&rgb, &obj, w, h), 1);
        // a hole: one background pixel among object pixels
        obj[5 * w + 5] = false;
        assert_eq!(specks(&rgb, &obj, w, h), 2);
        // a textured surface (neighbours that vary) is no speck
        let mut tex = vec![128u8; w * h * 3];
        for (i, px) in tex.chunks_mut(3).enumerate() {
            let v = if (i % w + i / w) % 2 == 0 { 60 } else { 200 };
            px.copy_from_slice(&[v, v, v]);
        }
        assert_eq!(specks(&tex, &vec![true; w * h], w, h), 0);
        // an edge between object and background is no speck
        let mut obj2 = vec![true; w * h];
        for y in 0..h {
            for x in 4..w {
                obj2[y * w + x] = false;
            }
        }
        let flat = vec![128u8; w * h * 3];
        assert_eq!(specks(&flat, &obj2, w, h), 0);
    }

    #[test]
    fn best_prefers_fewer_specks_then_speed() {
        let t = |s: f64, secs: f64| Trial { what: String::new(), raystep: 0.5, limiter: 1.0, specks_per_k: s, seconds: secs };
        assert_eq!(best(&[t(2.0, 1.0), t(0.5, 2.0), t(0.6, 1.5)]), 1);
        assert_eq!(best(&[t(0.5, 1.0), t(0.5, 2.0)]), 0);
        assert_eq!(best(&[t(0.5, 1.0), t(0.55, 0.5)]), 1);
        assert!(report(&[t(0.5, 1.0), t(0.5, 2.0)]).contains("keep them"));
    }

    #[test]
    fn settings_try_finer_steps() {
        let mut s = Scene::default();
        s.z_step_div = 0.5;
        s.raystep_limiter = 1.0;
        let v = settings(&s);
        assert_eq!(v.len(), 3);
        assert_eq!(v[1].1, 0.25);
        assert_eq!(v[2].2, 0.25);
        assert_eq!(crop(&s), [(s.width - 256) / 2, (s.height - 192) / 2, 256, 192]);
    }
}
