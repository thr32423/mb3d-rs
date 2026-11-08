//! Texture export (not in MB3D): the maps a fractal render gives a 3D
//! program or a material editor, from the picture calculated last, each
//! as its own PNG next to the others: the colour as rendered, the height
//! (the depth of the surface, 16 bit, the nearest point white), the normal
//! map (the surface normals as colours, the usual OpenGL convention: red
//! right, green up, blue towards the viewer) and the ambient occlusion
//! (16 bit grey, open surface white).  The set can be made tileable by
//! mirroring it (the picture and its mirror images side by side, which
//! joins seamlessly at every edge).  The group's recipe for textures (a
//! field of view near 1 degree, so the picture is nearly orthographic) is
//! the camera preset.

use crate::gbuffer::SiLight;
use crate::scene::{CameraOptic, Scene};
use std::path::{Path, PathBuf};

/// Which maps to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Maps {
    pub color: bool,
    pub height: bool,
    pub normal: bool,
    pub ao: bool,
    /// mirror the picture to twice its size, seamless at every edge
    pub tileable: bool,
}

impl Default for Maps {
    fn default() -> Self {
        Maps { color: true, height: true, normal: true, ao: true, tileable: false }
    }
}

/// The camera of the group's texture recipe: a planar optic with a field
/// of view of one degree (nearly orthographic: no perspective in the
/// height map), the view straight on.  Everything else stays.
pub fn camera_preset(sc: &mut Scene) {
    sc.optic = CameraOptic::Planar;
    sc.fov_y = 1.0;
}

/// A picture and its mirror images, twice as wide and high.
pub fn mirror<T: Copy>(px: &[T], w: usize, h: usize, channels: usize) -> Vec<T> {
    let (w2, h2) = (w * 2, h * 2);
    let mut out = Vec::with_capacity(w2 * h2 * channels);
    for y in 0..h2 {
        let sy = if y < h { y } else { h2 - 1 - y };
        for x in 0..w2 {
            let sx = if x < w { x } else { w2 - 1 - x };
            out.extend_from_slice(&px[(sy * w + sx) * channels..(sy * w + sx + 1) * channels]);
        }
    }
    out
}

/// A normal map and its mirror images: in the mirrored copies the normals
/// are mirrored too (red inverted where the picture is flipped left to
/// right, green where it is flipped top to bottom), so the light falls
/// right on every copy.
pub fn mirror_normals(px: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut m = mirror(px, w, h, 3);
    let (w2, h2) = (w * 2, h * 2);
    for y in 0..h2 {
        for x in 0..w2 {
            let i = (y * w2 + x) * 3;
            if x >= w {
                m[i] = 255 - m[i];
            }
            if y >= h {
                m[i + 1] = 255 - m[i + 1];
            }
        }
    }
    m
}

/// The height map: the depth of the surface over the picture, 16 bit, the
/// nearest surface point 65535, the farthest 1, the background 0.
pub fn height_map(g: &[SiLight]) -> Vec<u16> {
    let depths = g.iter().filter(|s| !s.is_background()).map(|s| s.zpos());
    let (near, far) = depths.fold((u32::MAX, 0u32), |(a, b), z| (a.min(z), b.max(z)));
    let range = (far.max(near) - near).max(1) as f64;
    g.iter()
        .map(|s| if s.is_background() { 0 } else { 1 + ((far - s.zpos()) as f64 / range * 65534.0).round() as u16 })
        .collect()
}

/// The normal map: the surface normals of the G-buffer (view space, x
/// right, y down, z into the picture) as colours in the OpenGL
/// convention (x right, y up, z towards the viewer, 128 = 0); the
/// background faces the viewer.
pub fn normal_map(g: &[SiLight]) -> Vec<u8> {
    let mut out = Vec::with_capacity(g.len() * 3);
    for s in g {
        if s.is_background() {
            out.extend_from_slice(&[128, 128, 255]);
            continue;
        }
        let n = [s.normal[0] as f64, -(s.normal[1] as f64), -(s.normal[2] as f64)];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1.0);
        for v in n {
            out.push(((v / len) * 127.5 + 127.5).round().clamp(0.0, 255.0) as u8);
        }
    }
    out
}

/// The ambient occlusion map: the G-buffer's ambient shadow, 16 bit, open
/// surface white, the background white too.
pub fn ao_map(g: &[SiLight]) -> Vec<u16> {
    g.iter().map(|s| if s.is_background() { 65535 } else { 65535 - s.amb_shadow.min(65535) }).collect()
}

/// Writes the chosen maps of a calculated picture as `<folder>/<name>_<map>.png`
/// and returns the files written.
pub fn write(g: &[SiLight], rgb: &[u8], w: usize, h: usize, folder: &Path, name: &str, maps: Maps) -> Result<Vec<PathBuf>, String> {
    if g.len() != w * h || rgb.len() != w * h * 3 {
        return Err("the picture and its G-buffer do not fit together".into());
    }
    std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let name = if name.trim().is_empty() { "texture" } else { name.trim() };
    let (ow, oh) = if maps.tileable { (w * 2, h * 2) } else { (w, h) };
    let mut out = Vec::new();
    let mut save = |suffix: &str, png: Vec<u8>| -> Result<(), String> {
        let p = folder.join(format!("{name}_{suffix}.png"));
        std::fs::write(&p, png).map_err(|e| format!("{}: {e}", p.display()))?;
        out.push(p);
        Ok(())
    };
    let gray16 = |v: Vec<u16>| if maps.tileable { mirror(&v, w, h, 1) } else { v };
    let rgb8 = |v: Vec<u8>| if maps.tileable { mirror(&v, w, h, 3) } else { v };
    if maps.color {
        save("color", crate::png::encode_rgb(ow, oh, &rgb8(rgb.to_vec())))?;
    }
    if maps.height {
        save("height", crate::png::encode_gray16(ow, oh, &gray16(height_map(g))))?;
    }
    if maps.normal {
        let n = normal_map(g);
        save("normal", crate::png::encode_rgb(ow, oh, &if maps.tileable { mirror_normals(&n, w, h) } else { n }))?;
    }
    if maps.ao {
        save("ao", crate::png::encode_gray16(ow, oh, &gray16(ao_map(g))))?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface(z: u32, normal: [i16; 3], amb: u16) -> SiLight {
        SiLight { zpos_fine: z << 8, normal, amb_shadow: amb, ..Default::default() }
    }

    #[test]
    fn maps_from_a_small_buffer() {
        let bg = SiLight::default();
        assert!(bg.is_background());
        let g = [surface(1000, [0, 0, -32767], 0), surface(3000, [32767, 0, 0], 65535), bg, surface(2000, [0, -32767, 0], 32768)];
        let h = height_map(&g);
        assert_eq!(h, [65535, 1, 0, 32768]);
        let n = normal_map(&g);
        assert_eq!(&n[0..3], &[128, 128, 255]);
        assert_eq!(&n[3..6], &[255, 128, 128]);
        assert_eq!(&n[6..9], &[128, 128, 255]);
        assert_eq!(&n[9..12], &[128, 255, 128]);
        assert_eq!(ao_map(&g), [65535, 0, 65535, 32767]);
    }

    #[test]
    fn mirroring_is_seamless() {
        let px: Vec<u8> = vec![1, 2, 3, 4];
        let m = mirror(&px, 2, 2, 1);
        assert_eq!(m, vec![1, 2, 2, 1, 3, 4, 4, 3, 3, 4, 4, 3, 1, 2, 2, 1]);
        // a normal pointing right (red 255) points left in the mirrored half
        let n = mirror_normals(&[255, 128, 128], 1, 1);
        assert_eq!(n, vec![255, 128, 128, 0, 128, 128, 255, 127, 128, 0, 127, 128]);
    }

    #[test]
    fn height_agrees_with_the_depth_map_of_the_command_line() {
        let mut s = Scene::preset("Integer Power").unwrap();
        s.width = 64;
        s.height = 48;
        let r = crate::render::render(&s, &|_, _| {}).unwrap();
        let h = height_map(&r.gbuffer);
        let (d, _, _) = crate::frames::depth_of(&r.gbuffer, 64, 48, 1);
        // both are brighter the nearer the surface: the same order on
        // every pair of surface pixels
        let obj: Vec<usize> = (0..h.len()).filter(|&i| !r.gbuffer[i].is_background()).collect();
        assert!(obj.len() > 100);
        for w in obj.windows(2) {
            let (a, b) = (w[0], w[1]);
            assert_eq!(h[a].cmp(&h[b]), d[a].cmp(&d[b]), "pixels {a} {b}");
        }
        // the nearest pixel is white, the farthest nearly black
        assert_eq!(*h.iter().max().unwrap(), 65535);
        assert_eq!(obj.iter().map(|&i| h[i]).min().unwrap(), 1);
    }

    #[test]
    fn writes_the_files() {
        let dir = std::env::temp_dir().join(format!("mb3d-texture-test-{}", std::process::id()));
        let g = vec![surface(1000, [0, 0, -32767], 0); 4];
        let rgb = vec![200u8; 12];
        let files = write(&g, &rgb, 2, 2, &dir, "t", Maps { tileable: true, ..Maps::default() }).unwrap();
        assert_eq!(files.len(), 4);
        assert!(files.iter().all(|f| f.exists()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut sc = Scene::default();
        camera_preset(&mut sc);
        assert_eq!((sc.optic, sc.fov_y), (CameraOptic::Planar, 1.0));
    }
}
