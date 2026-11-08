//! Sharing a picture with a group (the Share window, not in MB3D): the
//! picture as a JPEG of a size the site keeps sharp, the parameters as a
//! text file (Facebook turns pasted parameters into links and has banned
//! members for "spam", so the Mandelbulb Maniacs group asks for them as a
//! .txt file), and the text of the post, written into a folder of their
//! own.  The words of the post are the program's (translated); this module
//! does the files.

use crate::scene::{Scene, DECOMB_NAMES};
use std::path::{Path, PathBuf};

/// The long side Facebook keeps sharp (it shrinks larger pictures).
pub const FACEBOOK_LONG_SIDE: usize = 2048;
/// The width Facebook takes for a 360 photo at most.
pub const FACEBOOK_PANO_WIDTH: usize = 6000;

/// The formulas of a scene for a post: the used slots' names in order,
/// once each; a DE combination as both parts with its kind
/// (`Integer Power, Amazing Box + SphereTree (DE combination: min)`).
pub fn formula_summary(sc: &Scene) -> String {
    let names = |r: std::ops::Range<usize>| {
        let mut v: Vec<String> = Vec::new();
        for (i, f) in sc.formulas.iter().enumerate() {
            if r.contains(&i) && f.iterations != 0 {
                let n = f.formula.name();
                if !v.contains(&n) {
                    v.push(n);
                }
            }
        }
        v.join(", ")
    };
    match sc.decomb {
        Some(d) => {
            let a = names(0..d.end1 + 1);
            let b = names(d.start2..d.end2 + 1);
            let kind = DECOMB_NAMES[(d.kind.clamp(1, 6) - 1) as usize].replace('_', " ");
            format!("{a} + {b} (DE combination: {kind})")
        }
        None => names(0..sc.formulas.len()),
    }
}

/// A file name for `title`: letters, digits, spaces, `-` and `_` kept,
/// other characters left out, at most 80 characters; "fractal" when
/// nothing is left.
pub fn file_name_of(title: &str) -> String {
    let t: String = title.chars().filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_')).collect();
    let t: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let t: String = t.chars().take(80).collect();
    if t.is_empty() {
        "fractal".into()
    } else {
        t
    }
}

/// The picture with its long side at most `long` (averaging the source
/// pixels each new pixel covers); unchanged when it is that small.
pub fn shrink_to(rgb: &[u8], w: usize, h: usize, long: usize) -> (Vec<u8>, usize, usize) {
    let side = w.max(h);
    if long == 0 || side <= long || w == 0 || h == 0 {
        return (rgb.to_vec(), w, h);
    }
    let f = long as f64 / side as f64;
    let (nw, nh) = (((w as f64 * f).round() as usize).max(1), ((h as f64 * f).round() as usize).max(1));
    let (sx, sy) = (w as f64 / nw as f64, h as f64 / nh as f64);
    let mut out = vec![0u8; nw * nh * 3];
    for y in 0..nh {
        let (y0, y1) = (y as f64 * sy, (y + 1) as f64 * sy);
        for x in 0..nw {
            let (x0, x1) = (x as f64 * sx, (x + 1) as f64 * sx);
            let mut acc = [0f64; 3];
            let mut wsum = 0.0;
            let mut yy = y0.floor() as usize;
            while (yy as f64) < y1 && yy < h {
                let wy = (y1.min(yy as f64 + 1.0) - y0.max(yy as f64)).max(0.0);
                let mut xx = x0.floor() as usize;
                while (xx as f64) < x1 && xx < w {
                    let wx = (x1.min(xx as f64 + 1.0) - x0.max(xx as f64)).max(0.0);
                    let k = (yy * w + xx) * 3;
                    let wt = wx * wy;
                    for c in 0..3 {
                        acc[c] += rgb[k + c] as f64 * wt;
                    }
                    wsum += wt;
                    xx += 1;
                }
                yy += 1;
            }
            let k = (y * nw + x) * 3;
            for c in 0..3 {
                out[k + c] = (acc[c] / wsum.max(1e-9)).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    (out, nw, nh)
}

/// The long side a picture of `sc` gets for Facebook: 2048, a 360 photo
/// up to 6000 wide (its own size when smaller).
pub fn facebook_long_side(sc: &Scene, w: usize, h: usize) -> usize {
    if crate::vr::is_pano(sc, w, h) {
        FACEBOOK_PANO_WIDTH
    } else {
        FACEBOOK_LONG_SIDE
    }
}

/// What to share.
pub struct Share<'a> {
    /// the scene the picture shows
    pub scene: &'a Scene,
    /// the file names' stem
    pub name: String,
    /// the picture (8 bit RGB), and the long side to shrink it to (0: as it is)
    pub picture: Option<(&'a [u8], usize, usize, usize)>,
    /// the parameter file's text (the lines above the parameters and the
    /// MB3D text parameters)
    pub params: Option<String>,
    /// the post's text
    pub post: String,
}

/// The files written by [`write`].
#[derive(Debug, Default, PartialEq)]
pub struct Written {
    pub folder: PathBuf,
    pub picture: Option<PathBuf>,
    pub params: Option<PathBuf>,
    pub post: PathBuf,
}

/// A folder of its own for `name` in `parent`: `name`, or `name 2`, ...
/// when that exists already.
pub fn new_folder(parent: &Path, name: &str) -> PathBuf {
    let mut p = parent.join(name);
    let mut k = 2;
    while p.exists() {
        p = parent.join(format!("{name} {k}"));
        k += 1;
    }
    p
}

/// Writes the picture (JPEG, quality 92, with the tags of a 360 photo for
/// a panorama), the parameter file and the post's text into a new folder
/// of `parent`.
pub fn write(parent: &Path, s: &Share) -> Result<Written, String> {
    let folder = new_folder(parent, &s.name);
    std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let put = |file: &str, data: &[u8]| -> Result<PathBuf, String> {
        let p = folder.join(file);
        std::fs::write(&p, data).map_err(|e| format!("{}: {e}", p.display()))?;
        Ok(p)
    };
    let mut out = Written { folder: folder.clone(), ..Default::default() };
    if let Some((rgb, w, h, long)) = s.picture {
        let (px, pw, ph) = shrink_to(rgb, w, h, long);
        // (a shrunk 360 picture stays 2:1)
        out.picture = Some(put(&format!("{}.jpg", s.name), &crate::vr::encode_jpeg_for(s.scene, pw, ph, &px, 92))?);
    }
    if let Some(t) = &s.params {
        out.params = Some(put(&format!("{}.txt", s.name), t.replace('\n', "\r\n").as_bytes())?);
    }
    out.post = put("post.txt", s.post.replace('\n', "\r\n").as_bytes())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{DeCombParams, FormulaEntry};

    #[test]
    fn formulas_of_a_scene() {
        let mut sc = Scene::default();
        let f = |n: &str, its| FormulaEntry { formula: crate::formulas::lookup(n).unwrap(), iterations: its };
        sc.formulas = vec![f("Integer Power", 4), f("Amazing Box", 2), f("Integer Power", 1), f("Bulbox", 0)];
        assert_eq!(formula_summary(&sc), "Integer Power, Amazing Box");
        let mut d = DeCombParams::new(30);
        d.kind = 5;
        d.end1 = 0;
        d.start2 = 1;
        d.end2 = 3;
        sc.decomb = Some(d);
        assert_eq!(formula_summary(&sc), "Integer Power + Amazing Box, Integer Power (DE combination: smooth)");
    }

    #[test]
    fn file_names() {
        assert_eq!(file_name_of("  Spaceport: a tweak / of \"Pipe Organ\"  "), "Spaceport a tweak of Pipe Organ");
        assert_eq!(file_name_of("???"), "fractal");
        assert_eq!(file_name_of("Ünïcode ok"), "Ünïcode ok");
    }

    #[test]
    fn shrinking_keeps_the_mean_and_the_shape() {
        let (w, h) = (300, 150);
        let rgb: Vec<u8> = (0..w * h).flat_map(|i| if (i % w) < w / 2 { [200, 100, 0] } else { [0, 100, 200] }).collect();
        let (p, nw, nh) = shrink_to(&rgb, w, h, 64);
        assert_eq!((nw, nh), (64, 32));
        let mean = |v: &[u8], c: usize| v.chunks(3).map(|p| p[c] as f64).sum::<f64>() / (v.len() / 3) as f64;
        assert!((mean(&p, 0) - mean(&rgb, 0)).abs() < 1.0);
        assert_eq!(p[0], 200);
        assert_eq!(p[(nw - 1) * 3 + 2], 200);
        // small pictures stay
        assert_eq!(shrink_to(&rgb, w, h, 400).1, 300);
    }

    #[test]
    fn a_share_folder_holds_the_three_files() {
        let parent = std::env::temp_dir().join(format!("mb3d-share-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        let mut sc = Scene::default();
        sc.optic = crate::scene::CameraOptic::Panorama;
        let rgb = vec![120u8; 400 * 200 * 3];
        let s = Share { scene: &sc, name: "Hall".into(), picture: Some((&rgb, 400, 200, 100)), params: Some("by me\n\nMandelbulb3Dv18{x}".into()), post: "Hall\nFormulas: X".into() };
        let a = write(&parent, &s).unwrap();
        assert_eq!(a.folder, parent.join("Hall"));
        let img = crate::image::decode(&std::fs::read(a.picture.as_ref().unwrap()).unwrap()).unwrap();
        assert_eq!((img.width, img.height), (100, 50));
        // a 360 photo: its tags
        let j = std::fs::read(a.picture.unwrap()).unwrap();
        assert!(String::from_utf8_lossy(&j).contains("equirectangular"));
        assert_eq!(std::fs::read_to_string(a.params.unwrap()).unwrap(), "by me\r\n\r\nMandelbulb3Dv18{x}");
        // a second share of that name: a folder of its own
        let b = write(&parent, &s).unwrap();
        assert_eq!(b.folder, parent.join("Hall 2"));
    }
}
