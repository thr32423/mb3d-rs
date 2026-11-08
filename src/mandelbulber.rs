//! Mandelbulber settings files (`.fract`) opened as a scene.  The two
//! programs share many formulas (Mandelbulber took a good part of its list
//! from MB3D), so a file traded in a group can be opened here: the camera,
//! the picture size, the iteration count, the Julia mode and the formulas
//! both programs have are carried over, the formulas' values where their
//! meaning is the same, approximately where the scales differ.  Lights,
//! materials, colours and effects stay at MB3D's defaults; everything that
//! was not carried over is named in the notes, so the user knows what to
//! set by hand.
//!
//! The file is text: `# Mandelbulber settings file`, `# version 2.33`,
//! then sections `[main_parameters]`, `[fractal_1]`.. `[fractal_9]`,
//! `[material_N]`, with one `key value;` per line, the decimals written
//! with the writer's locale (a comma in Poland, a dot elsewhere), vectors
//! as space-separated numbers, colours as three 16-bit hexadecimal
//! numbers.  Only the values that differ from Mandelbulber's defaults are
//! written, so a missing key means its default (the defaults are those of
//! Mandelbulber 2.33, src/initparameters.cpp).

use crate::formulas::Formula;
use crate::math::{self, Vec3};
use crate::scene::{CameraOptic, DeCombParams, FormulaEntry, Scene};
use std::collections::HashMap;

/// Whether a file is a Mandelbulber settings file, by its name or its
/// first line.
pub fn is_fract(name: &str, text: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".fract") || text.trim_start().starts_with("# Mandelbulber settings file")
}

/// A section of the file: its keys with their raw values (decimals with a
/// dot).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Section {
    pub keys: HashMap<String, String>,
}

impl Section {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.keys.get(key).map(|s| s.as_str())
    }
    pub fn num(&self, key: &str) -> Option<f64> {
        self.get(key)?.split_whitespace().next()?.parse().ok()
    }
    pub fn vec3(&self, key: &str) -> Option<Vec3> {
        let mut it = self.get(key)?.split_whitespace().map(|t| t.parse::<f64>().ok());
        Some([it.next()??, it.next()??, it.next()??])
    }
    pub fn boolean(&self, key: &str) -> Option<bool> {
        match self.get(key)?.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        }
    }
    /// A colour `rrrr gggg bbbb` of 16-bit hexadecimal channels, as 8 bits.
    pub fn color(&self, key: &str) -> Option<[u8; 3]> {
        let mut it = self.get(key)?.split_whitespace().map(|t| u32::from_str_radix(t, 16).ok().map(|v| (v >> 8).min(255) as u8));
        Some([it.next()??, it.next()??, it.next()??])
    }
}

/// The parsed file: the main section and the fractal sections (index 0 is
/// `[fractal_1]`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub version: String,
    pub main: Section,
    pub fractals: Vec<Section>,
}

/// Reads the sections and keys of a settings file.
pub fn parse(text: &str) -> Result<Settings, String> {
    let mut s = Settings::default();
    let mut cur: Option<String> = None;
    let mut seen_main = false;
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if let Some(rest) = l.strip_prefix('#') {
            let r = rest.trim();
            if let Some(v) = r.strip_prefix("version") {
                s.version = v.trim().to_string();
            }
            continue;
        }
        if l.starts_with('[') && l.ends_with(']') {
            let name = l[1..l.len() - 1].trim().to_ascii_lowercase();
            if name == "main_parameters" {
                seen_main = true;
            }
            cur = Some(name);
            continue;
        }
        let Some(sec) = &cur else { continue };
        let l = l.strip_suffix(';').unwrap_or(l);
        let (key, value) = match l.find(char::is_whitespace) {
            Some(i) => (&l[..i], l[i..].trim()),
            None => (l, ""),
        };
        // the writer's decimal comma
        let value = value.replace(',', ".");
        let target = if sec == "main_parameters" {
            &mut s.main
        } else if let Some(n) = sec.strip_prefix("fractal_").and_then(|n| n.parse::<usize>().ok()).filter(|&n| n >= 1) {
            if s.fractals.len() < n {
                s.fractals.resize(n, Section::default());
            }
            &mut s.fractals[n - 1]
        } else {
            continue;
        };
        target.keys.insert(key.to_string(), value);
    }
    if !seen_main && s.fractals.is_empty() {
        return Err("not a Mandelbulber settings file (no [main_parameters] section)".into());
    }
    Ok(s)
}

/// The scene made of a settings file, with the notes about what was not
/// carried over.
#[derive(Clone, Debug)]
pub struct Import {
    pub scene: Scene,
    pub notes: Vec<String>,
}

/// Mandelbulber's formula numbers (formula/definition/all_fractal_list_enums.hpp)
/// with the formula here that comes closest.  `exact` says the two are the
/// same formula (the values carry over), else the shape is only similar.
struct Target {
    name: &'static str,
    mandelbulber: &'static str,
    exact: bool,
}

fn target(id: i32) -> Option<Target> {
    let t = |name, mandelbulber, exact| Some(Target { name, mandelbulber, exact });
    match id {
        1 => t("Integer Power", "fast mandelbulb", true),
        2 => t("Integer Power", "mandelbulb", true),
        3 => t("Integer Power", "mandelbulb power 2", true),
        5 => t("Quaternion", "quaternion", true),
        7 => t("Menger3", "menger sponge", true),
        8 => t("Amazing Box", "mandelbox", true),
        9 => t("Amazing Box", "mandelbox fast", true),
        10 => t("MengerIFS", "kaleidoscopic IFS", false),
        11 => t("Real Power", "xenodreambuie", false),
        13 => t("Integer Power", "mandelbulb 2", false),
        14 => t("Integer Power", "mandelbulb 3", false),
        15 => t("Integer Power", "mandelbulb 4", false),
        16 => t("Folding Int Pow", "box fold bulb pow2", true),
        19 => t("Aexion1", "aexion", true),
        20 => t("Benesi1Pow2", "benesi", false),
        21 => t("BristorBrot", "bristorbrot", true),
        51 => t("MsltoeSym2", "msltoe sym2 mod", true),
        60 => t("ABoxModKali", "abox mod kali", true),
        61 => t("Menger3", "menger mod1", false),
        64 => t("ABoxMod1", "abox mod1", true),
        65 | 118 => t("Real Power", "mandelbulb multi", false),
        66 | 67 => t("BenesiPine1", "benesi pine tree", false),
        68 => t("MsltoeSym3", "msltoe sym3 mod", true),
        69 => t("MsltoeSym4", "msltoe sym4 mod", true),
        71 | 72 | 139 => t("Riemann", "riemann sphere msltoe", false),
        73 => t("Amazing Surf", "amazing surf", true),
        74 => t("Kalisets1", "kalisets1", true),
        75 => t("ABoxMod2", "abox mod2", true),
        76 => t("ABoxVS_icen1", "abox vs icen1", true),
        77 => t("ASurfMod1", "amazing surf mod1", true),
        79 => t("Quaternion", "quaternion 4D", false),
        83 => t("IQ-bulb", "iq bulb", true),
        101 | 103 | 104 | 117 => t("PseudoKleinian", "pseudo kleinian", false),
        102 | 145 => t("BristorBrot", "bristorbrot variant", false),
        110 => t("Sierpinski3", "sierpinski 3D", true),
        113 => t("MengerSmt", "menger smooth", true),
        125 | 126 => t("Folding Int Pow", "box fold bulb pow2 variant", false),
        _ => None,
    }
}

/// Mandelbulber's name of a formula number that has no counterpart here,
/// for the notes.
fn mandelbulber_name(id: i32) -> String {
    let n = match id {
        4 => "hypercomplex",
        6 => "minus fast trig",
        12 => "hybrid",
        17 => "mandelbox smooth",
        18 => "mandelbox vary scale 4D",
        22..=25 => "invert",
        26 => "spherical fold",
        27 => "pow XYZ",
        28..=30 => "scale",
        31..=33 => "offset",
        34..=36 => "angle multiply",
        37 => "generalized fold box",
        39 | 40 => "ides",
        41 => "buffalo",
        48 => "quick dudley",
        49 => "lkmitch",
        50 => "makin 3D2",
        56 => "platonic solid",
        59 => "fold box mod1",
        62 => "quaternion 3D",
        63 => "aexion octopus mod",
        70 => "eiffie msltoe",
        78 => "mandelbulb vary power",
        80 | 81 => "msltoe sym3 mod2/3",
        82 => "msltoe donut",
        84 => "quick dudley mod",
        85 => "riemann bulb msltoe mod2",
        86 | 96 => "msltoe toroidal",
        87 => "abox mod kali eiffie",
        88 | 89 => "collatz",
        90 => "menger pwr2 poly",
        91 => "mandelbox menger",
        92 | 93 => "mandelbulb kali",
        94 => "mandelbulb bermarte",
        95 => "menger middle mod",
        97 => "amazing surf multi",
        98 => "menger cross KIFS",
        99 | 107 => "menger prism shape",
        100 => "benesi mag transforms",
        105 => "benesi pwr2 mandelbulb",
        106 => "menger cross mod1",
        108 => "mixpinski 4D",
        109 => "sierpinski 4D",
        111 | 112 => "menger 4D",
        114 => "menger smooth mod1",
        115 => "menger octo",
        116 => "abox 4D",
        119 | 123 | 127 => "abox mod 11/12/13",
        120 => "mandelbulb quat",
        121 => "mandelbulb juliabulb",
        122 | 143 => "jos kleinian",
        124 => "quaternion cubic 4D",
        128 => "box fold quat",
        129 => "coastalbrot",
        130 => "modulus menger sponge",
        131 => "modulus mandelbulb",
        132 | 147 => "amazing surf mod2/3",
        133 => "mandelbox variable",
        134 => "box fold 4D bulb pow2",
        135 | 136 => "abox mod kali V2/V3",
        137 => "mandelbulb abs power2",
        138 => "mandelbulb pow2 V2",
        140 => "abox surf box",
        141 => "box fold bulb menger",
        142 => "vicsek",
        144 => "mandelbulb atan2 power2",
        146 => "mandeltorus",
        148 => "mandelbulb power 1234",
        149 => "menger poly fold",
        150 => "IFS XY",
        1000..=1999 => "a transform",
        _ => "",
    };
    if n.is_empty() {
        format!("formula number {id}")
    } else {
        format!("'{n}' ({id})")
    }
}

/// Sets an option by name, noting an option the formula does not have.
fn set(f: &mut Formula, key: &str, value: f64, notes: &mut Vec<String>) {
    if f.set_option(key, value).is_err() {
        notes.push(format!("{}: the value '{key}' was not carried over (the formula has no such option)", f.name()));
    }
}

/// The formula of slot `i` (0-based) from the file: the formula and the
/// values the two programs share.
fn convert(id: i32, main: &Section, fr: Option<&Section>, notes: &mut Vec<String>) -> Option<Formula> {
    let t = target(id)?;
    let empty = Section::default();
    let fr = fr.unwrap_or(&empty);
    let mut f = match crate::formulas::lookup(t.name) {
        Ok(f) => f,
        Err(_) => {
            notes.push(format!("Mandelbulber's {} needs the formula '{}' , which is not installed; the slot was left empty", mandelbulber_name(id), t.name));
            return None;
        }
    };
    let val = |key: &str, default: f64| fr.num(key).or_else(|| main.num(key)).unwrap_or(default);
    match id {
        // the bulbs: the power (an integer 2..8 as the integer bulb)
        1 | 2 | 11 | 13 | 14 | 15 | 65 | 118 => {
            let p = val("power", 9.0);
            if (p - p.round()).abs() < 1e-9 && (2.0..=8.0).contains(&p) {
                set(&mut f, "power", p, notes);
            } else {
                f = crate::formulas::lookup("Real Power").ok()?;
                set(&mut f, "power", p, notes);
            }
            if fr.num("alpha_angle_offset").is_some_and(|v| v != 0.0) || fr.num("beta_angle_offset").is_some_and(|v| v != 0.0) {
                notes.push(format!("{}: the angle offsets were not carried over", f.name()));
            }
        }
        3 => set(&mut f, "power", 2.0, notes),
        7 | 61 | 113 => set(&mut f, "scale", val("transf_scale_3", 3.0), notes),
        8 | 9 => {
            set(&mut f, "scale", val("mandelbox_scale", 2.0), notes);
            set(&mut f, "fold", val("mandelbox_folding_limit", 1.0), notes);
            set(&mut f, "min r", val("mandelbox_folding_min_radius", 0.5), notes);
            let fixed = val("mandelbox_folding_fixed_radius", 1.0);
            if (fixed - 1.0).abs() > 1e-9 {
                notes.push(format!("{}: the fixed radius {fixed} was not carried over (it is 1 here)", f.name()));
            }
        }
        10 => set(&mut f, "scale", val("IFS_scale", 2.0), notes),
        16 | 125 | 126 => set(&mut f, "r fold", val("boxfold_bulbpow2_folding_factor", 2.0), notes),
        19 => set(&mut f, "c add", val("cadd", -1.3), notes),
        60 => set(&mut f, "scale", val("transf_scale_15", 1.5), notes),
        64 | 77 => {
            set(&mut f, "scale", val("mandelbox_scale", 2.0), notes);
            set(&mut f, "fold", val("mandelbox_folding_value", 2.0), notes);
        }
        73 | 75 => set(&mut f, "scale", val("mandelbox_scale", 2.0), notes),
        76 => {
            set(&mut f, "scale", val("mandelbox_scale", 2.0), notes);
            set(&mut f, "min r", val("mandelbox_folding_min_radius", 0.5), notes);
        }
        74 => set(&mut f, "scale", val("transf_scale", 1.0), notes),
        110 => set(&mut f, "scale", val("transf_scaleA_2", 2.0), notes),
        _ => {}
    }
    if !t.exact {
        notes.push(format!("Mandelbulber's '{}' ({id}) became '{}', a similar formula, not the same one; its values are the defaults", t.mandelbulber, f.name()));
    }
    Some(f)
}

/// The camera of the file as MB3D's: the eye, the view direction and the
/// up vector become `vgrads`, the pivot `mid`, the start plane `z_start`
/// and the pixel spacing `zoom`.
fn camera(s: &Settings, sc: &mut Scene, notes: &mut Vec<String>) {
    let m = &s.main;
    let eye = m.vec3("camera").unwrap_or([3.0, -6.0, 2.0]);
    let tgt = m.vec3("target").unwrap_or([0.0; 3]);
    let mut dir = math::sub(tgt, eye);
    let dist = math::len(&dir);
    if dist < 1e-12 {
        // the target on the eye: the direction from the rotation (yaw
        // around z, pitch), looking along +y when both are 0
        let rot = m.vec3("camera_rotation").unwrap_or([0.0; 3]);
        let (a, b) = (rot[0].to_radians(), rot[1].to_radians());
        dir = [-a.sin() * b.cos(), a.cos() * b.cos(), b.sin()];
    }
    let dir = math::normalize(dir);
    let top = m.vec3("camera_top").unwrap_or([0.0, 0.0, 1.0]);
    let mut up = math::sub(top, math::scale(dir, math::dot(&top, &dir)));
    if math::len(&up) < 1e-9 {
        up = if dir[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [0.0, 1.0, 0.0] };
        up = math::sub(up, math::scale(dir, math::dot(&up, &dir)));
    }
    let up = math::normalize(up);
    let right = math::normalize(cross(dir, up));
    // the rows: the picture's x to the right, its y down, z into the picture
    sc.vgrads = [right, [-up[0], -up[1], -up[2]], dir];

    let w = sc.width.max(1) as f64;
    let h = sc.height.max(1) as f64;
    let fov_h = m.num("fov").unwrap_or(53.13).clamp(0.1, 179.0).to_radians();
    let optic = m.num("perspective_type").unwrap_or(0.0) as i32;
    // Mandelbulber's field of view is the horizontal angle (the tangent
    // of its half spans the picture's width); MB3D's is the vertical one
    sc.fov_y = (2.0 * ((fov_h * 0.5).tan() * h / w).atan()).to_degrees();
    // the pivot: the target when it is a real distance away, else two
    // units in front of the eye
    let pivot = if dist > 0.05 { dist } else { 2.0 };
    // the start plane just in front of the eye (what lies closer is not
    // drawn, as in MB3D)
    let near = 0.01 * dist.max(1e-3);
    sc.mid = math::add(eye, math::scale(dir, pivot));
    sc.z_start = sc.mid[2] - (pivot - near);
    sc.z_end = sc.mid[2] + 30.0 * pivot;
    // the pixel spacing on the start plane: MB3D's rays start there,
    // spread by one pixel angle each
    let pixel_angle = sc.fov_y.to_radians() / h;
    let sw = near * pixel_angle.tan();
    sc.zoom = 2.1345 / (sw * w);
    sc.optic = match optic {
        2 => {
            // the panorama's rays start at the eye
            sc.mid = eye;
            sc.z_start = eye[2];
            sc.zoom = 1.0;
            notes.push("the equirectangular camera became a spherical panorama".into());
            CameraOptic::Panorama
        }
        1 | 3 => {
            notes.push("the fish eye camera became MB3D's equidistant fish eye; the angle may differ".into());
            CameraOptic::Fisheye
        }
        _ => CameraOptic::Common,
    };
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Opens the text of a settings file as a scene.
pub fn import(text: &str) -> Result<Import, String> {
    let s = parse(text)?;
    let m = &s.main;
    let mut notes = Vec::new();
    let mut sc = Scene::default();
    sc.width = m.num("image_width").unwrap_or(800.0).clamp(8.0, 65535.0) as i32;
    sc.height = m.num("image_height").unwrap_or(600.0).clamp(8.0, 65535.0) as i32;
    sc.iterations = m.num("N").unwrap_or(250.0).clamp(1.0, 65535.0) as i32;
    camera(&s, &mut sc, &mut notes);

    // the detail: Mandelbulber's detail level is the inverse of MB3D's
    // DE stop (both relative to the pixel size)
    if let Some(d) = m.num("detail_level").filter(|d| *d > 0.0) {
        sc.de_stop = (1.0 / d).clamp(0.01, 100.0);
    }
    if let Some(f) = m.num("DE_factor").filter(|f| *f > 0.0 && (*f - 1.0).abs() > 1e-9) {
        sc.z_step_div = (0.5 * f).clamp(0.01, 10.0);
    }
    // the Julia constant: the file's, or the first formula's own
    let julia_in = |sec: &Section| sec.boolean("julia_mode").unwrap_or(false).then(|| sec.vec3("julia_c").unwrap_or([0.0; 3]));
    if let Some(c) = julia_in(m).or_else(|| s.fractals.first().and_then(julia_in)) {
        sc.julia = true;
        sc.julia_c = [c[0], c[1], c[2], 0.0];
    }
    // the background colours become the ambient light's top and bottom,
    // which MB3D shows as the background
    if let Some(c) = m.color("background_color_1") {
        sc.lighting.amb_top = c;
    }
    if let Some(c) = m.color("background_color_2") {
        sc.lighting.amb_bottom = c;
    }

    // the formulas: the first alone, or the hybrid's slots in turn (as
    // MB3D's), or the boolean objects as a DE combination
    let hybrid = m.boolean("hybrid_fractal_enable").unwrap_or(false);
    let boolean = m.boolean("boolean_operators").unwrap_or(false);
    let mut slots: Vec<(Formula, i32)> = Vec::new();
    let mut dropped = 0;
    for i in 1..=9usize {
        if i > 1 && !hybrid && !boolean {
            break;
        }
        let id = m.num(&format!("formula_{i}")).map(|v| v as i32).unwrap_or(if i == 1 { 2 } else { 0 });
        if id <= 0 || !m.boolean(&format!("fractal_enable_{i}")).unwrap_or(true) {
            continue;
        }
        let its = m.num(&format!("formula_iterations_{i}")).unwrap_or(1.0).clamp(1.0, 65536.0) as i32;
        let fr = s.fractals.get(i - 1);
        match convert(id, m, fr, &mut notes) {
            Some(f) => {
                if slots.len() >= 6 {
                    notes.push(format!("formula {i} ({}) was dropped: MB3D has six slots", f.name()));
                    continue;
                }
                slots.push((f, its));
            }
            None => {
                if target(id).is_none() {
                    notes.push(format!("Mandelbulber's {} has no counterpart here and was left out", mandelbulber_name(id)));
                }
                dropped += 1;
            }
        }
        if let Some(fr) = fr {
            if fr.vec3("fractal_constant_factor").is_some_and(|v| v != [1.0; 3]) || fr.vec3("fractal_rotation").is_some_and(|v| v != [0.0; 3]) {
                notes.push(format!("formula {i}: the constant factor and rotation were not carried over"));
            }
        }
    }
    if let Some(v) = m.vec3("fractal_constant_factor").filter(|v| *v != [1.0; 3]) {
        notes.push(format!("the constant factor {} {} {} was not carried over", v[0], v[1], v[2]));
    }
    if m.vec3("fractal_rotation").is_some_and(|v| v != [0.0; 3]) {
        notes.push("the fractal rotation was not carried over (turn the camera instead)".into());
    }
    if slots.is_empty() {
        return Err(format!(
            "none of the file's formulas has a counterpart here ({dropped} left out); the scene would be empty"
        ));
    }
    sc.formulas = slots.iter().map(|(f, its)| FormulaEntry { formula: f.clone(), iterations: *its }).collect();
    if boolean && slots.len() >= 2 {
        let kind = match m.get("boolean_operator_1").map(|v| v.trim().to_ascii_uppercase()).as_deref() {
            Some("AND") => 2,
            Some("SUB") => 3,
            _ => 1,
        };
        sc.decomb = Some(DeCombParams { kind, end1: 0, start2: 1, end2: 1, ..DeCombParams::new(sc.iterations) });
        if slots.len() > 2 {
            notes.push("the boolean mode combines two objects here; the formulas after the second were left out".into());
            sc.formulas.truncate(2);
        } else {
            notes.push("the boolean objects became a DE combination of two parts".into());
        }
    }
    let n = sc.formulas.len();
    notes.insert(
        0,
        format!(
            "Mandelbulber {} settings: {} formula{} carried over{}; the lights, materials, colours, fog and effects are MB3D's defaults",
            if s.version.is_empty() { "2".to_string() } else { s.version.clone() },
            n,
            if n == 1 { "" } else { "s" },
            if dropped > 0 { format!(", {dropped} left out") } else { String::new() }
        ),
    );
    Ok(Import { scene: sc, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_commas() {
        let s = parse("# Mandelbulber settings file\n# version 2.33\n# only modified parameters\n[main_parameters]\ncamera -0,2 -3,7 -0,6;\nfov 28,07;\nformula_1 8;\nfog_color_1 718c 718c 718c;\n[fractal_1]\nmandelbox_scale -1,5;\n[fractal_3]\nIFS_rotation_enabled true;\n").unwrap();
        assert_eq!(s.version, "2.33");
        assert_eq!(s.main.vec3("camera"), Some([-0.2, -3.7, -0.6]));
        assert_eq!(s.main.num("fov"), Some(28.07));
        assert_eq!(s.main.color("fog_color_1"), Some([0x71, 0x71, 0x71]));
        assert_eq!(s.fractals.len(), 3);
        assert_eq!(s.fractals[0].num("mandelbox_scale"), Some(-1.5));
        assert_eq!(s.fractals[2].boolean("IFS_rotation_enabled"), Some(true));
        assert!(parse("just text").is_err());
    }

    #[test]
    fn is_fract_by_name_or_header() {
        assert!(is_fract("a.FRACT", ""));
        assert!(is_fract("a.txt", "# Mandelbulber settings file\n"));
        assert!(!is_fract("a.txt", "Mandelbulb3Dv1.99"));
    }
}
