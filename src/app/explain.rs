//! Plain-language explanations of the errors new users meet most (asked
//! for in the Mandelbulb Maniacs group, where "formula option is not
//! valid" and "mixing of dIFS will not work" puzzle beginners): a
//! parameter file that names a formula not installed, and the formula
//! combinations a calculation cannot start with.  Each explanation says
//! what the words mean and what to do; where the program can do it (a
//! DE combination for dIFS and other formulas in a clean order), it
//! offers to.

use crate::scene::{DeCombParams, Scene};
use crate::vcl::lang::{tr, trf};

/// What the program can set up for the user.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fix {
    /// a DE combination of slots ..=`end1` and `start2`..=`end2` (0-based)
    DeComb { end1: usize, start2: usize, end2: usize },
}

/// An error in words, with the fix the program can apply.
#[derive(Clone, Debug, PartialEq)]
pub struct Explained {
    pub caption: String,
    pub text: String,
    pub fix: Option<Fix>,
}

/// The name of the formula of a "formula 'X' not found" error.
pub fn missing_formula_name(err: &str) -> Option<String> {
    let rest = err.split("formula '").nth(1)?;
    let (name, after) = rest.split_once('\'')?;
    after.trim_start().starts_with("not found").then(|| name.to_string())
}

/// Letters and digits only, lower case (to compare formula names).
fn plain(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The names in `installed` that look like `name` (the same letters and
/// digits, one containing the other, or a few letters apart), the
/// closest first, at most `max`.
pub fn similar_names(name: &str, installed: &[String], max: usize) -> Vec<String> {
    let want = plain(name);
    if want.is_empty() {
        return Vec::new();
    }
    let limit = (want.len() / 4).clamp(1, 3);
    let mut v: Vec<(usize, &String)> = installed
        .iter()
        .filter_map(|n| {
            let p = plain(n);
            if p.is_empty() {
                return None;
            }
            let d = if p == want {
                0
            } else if p.len() >= 4 && want.len() >= 4 && (p.contains(&want) || want.contains(&p)) {
                1 + p.len().abs_diff(want.len()).min(9)
            } else {
                let d = distance(&p, &want);
                if d > limit {
                    return None;
                }
                d + 1
            };
            Some((d, n))
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.to_ascii_lowercase().cmp(&b.1.to_ascii_lowercase())));
    v.into_iter().take(max).map(|(_, n)| n.clone()).collect()
}

/// A "formula 'X' not found" error in words: what the file needs, where
/// formulas go, and installed ones of a similar name.
pub fn missing_formula(err: &str) -> Option<Explained> {
    let name = missing_formula_name(err)?;
    let mut dirs: Vec<String> = Vec::new();
    for d in crate::formulas::formula_dir_list().into_iter().filter(|d| d.is_dir()) {
        let line = format!("    {}", std::fs::canonicalize(&d).unwrap_or(d).display());
        if !dirs.contains(&line) {
            dirs.push(line);
        }
    }
    let mut text = trf("@MB:TheseParametersUseTheFormulaWhich", &[&name, &name, &dirs.join("\n")]);
    let mut installed = crate::formulas::list_custom();
    installed.extend(crate::formulas::Formula::all_names().iter().map(|s| s.to_string()));
    let similar = similar_names(&name, &installed, 5);
    if !similar.is_empty() {
        text.push_str("\n\n");
        text.push_str(&trf("@MB:InstalledFormulasWithASimilarName", &[&similar.join(", ")]));
    }
    Some(Explained { caption: tr("@MB:FormulaNotInstalled"), text, fix: None })
}

/// The slots that take part in the calculation: (slot, name, DE option).
fn used_slots(sc: &Scene, range: std::ops::RangeInclusive<usize>) -> Vec<(usize, String, i32)> {
    sc.formulas
        .iter()
        .enumerate()
        .filter(|(i, f)| range.contains(i) && f.iterations != 0)
        .map(|(i, f)| (i, f.formula.name(), f.formula.de_option()))
        .collect()
}

fn names(v: &[(usize, String, i32)], difs: bool) -> String {
    let mut n: Vec<String> = Vec::new();
    for (_, name, de) in v {
        if (*de >= 20) == difs && !n.contains(name) {
            n.push(name.clone());
        }
    }
    n.join(", ")
}

/// The slots as the user counts them: "3", or "1 to 2".
fn slot_range(a: usize, b: usize) -> String {
    if a == b {
        trf("@MB:Slot", &[&(a + 1)])
    } else {
        trf("@MB:SlotsTo", &[&(a + 1), &(b + 1)])
    }
}

/// The DE combination that separates dIFS and other formulas when the
/// used slots hold one kind first and the other after it.
pub fn decomb_fix(sc: &Scene) -> Option<Fix> {
    if sc.decomb.is_some() || sc.interpolation.is_some() {
        return None;
    }
    let used = used_slots(sc, 0..=sc.formulas.len().saturating_sub(1));
    let kinds: Vec<bool> = used.iter().map(|(_, _, de)| *de >= 20).collect();
    let switch = kinds.windows(2).filter(|w| w[0] != w[1]).count();
    if switch != 1 {
        return None;
    }
    let k = kinds.windows(2).position(|w| w[0] != w[1])? + 1;
    let (end1, start2, end2) = (used[k - 1].0, used[k].0, used.last()?.0);
    // the second part must hold a dIFS shape when it is the dIFS one (and
    // the first part as well): a part of dIFS transforms only draws nothing
    let part_ok = |r: &[(usize, String, i32)]| !r.iter().any(|x| x.2 >= 20) || r.iter().any(|x| x.2 == 20);
    if !part_ok(&used[..k]) || !part_ok(&used[k..]) {
        return None;
    }
    Some(Fix::DeComb { end1, start2, end2 })
}

/// Applies a fix to the scene.
pub fn apply_fix(sc: &mut Scene, fix: Fix) {
    match fix {
        Fix::DeComb { end1, start2, end2 } => {
            let its = sc.formulas.iter().map(|f| f.iterations).max().unwrap_or(1).max(1);
            let mut d = DeCombParams::new(sc.iterations.max(its));
            d.kind = 1;
            d.end1 = end1;
            d.start2 = start2;
            d.end2 = end2;
            d.repeat2 = start2;
            sc.decomb = Some(d);
        }
    }
}

/// A calculation's error in words, for the scene it came from.
pub fn calc_error(sc: &Scene, err: &str) -> Option<Explained> {
    let caption = tr("@MB:WhyTheCalculationCannotStart");
    let last = sc.formulas.len().saturating_sub(1);
    if err.contains("mixing of dIFS with other formulas in the second part") {
        let d = sc.decomb?;
        let used = used_slots(sc, d.start2..=d.end2.min(last));
        let text = trf("@MB:ThePartOfTheDECombination", &[&tr("@MB:Second"), &names(&used, true), &names(&used, false)]);
        return Some(Explained { caption, text, fix: None });
    }
    if err.contains("mixing of dIFS with other formulas") {
        let range = match sc.decomb {
            Some(d) => 0..=d.end1.min(last),
            None => 0..=last,
        };
        let used = used_slots(sc, range);
        if sc.decomb.is_some() {
            let text = trf("@MB:ThePartOfTheDECombination", &[&tr("@MB:First"), &names(&used, true), &names(&used, false)]);
            return Some(Explained { caption, text, fix: None });
        }
        let mut text = trf("@MB:ThisSceneMixesTwoKindsOf", &[&names(&used, true), &names(&used, false)]);
        let fix = decomb_fix(sc);
        text.push_str("\n\n");
        match fix {
            Some(Fix::DeComb { end1, start2, end2 }) => {
                let first = used.first().map(|u| u.0).unwrap_or(0);
                text.push_str(&trf("@MB:SetItUpNowTheFormulas", &[&slot_range(first, end1), &slot_range(start2, end2)]));
            }
            None => text.push_str(&tr("@MB:HereTheDIFSFormulasAndThe")),
        }
        return Some(Explained { caption, text, fix });
    }
    if err.contains("no dIFS shape formula selected") {
        let used = used_slots(sc, 0..=last);
        let text = trf("@MB:TheDIFSFormulasHereOnlyTransform", &[&names(&used, true)]);
        return Some(Explained { caption, text, fix: None });
    }
    if err.contains("dIFS formulas do not work in an interpolation hybrid") {
        return Some(Explained { caption, text: tr("@MB:AnInterpolationHybridTheInterpolateTab"), fix: None });
    }
    if err.contains("formula option is not valid") || err.contains("no formula with iterations > 0") {
        let used = used_slots(sc, 0..=last);
        let list = used.iter().map(|u| u.1.clone()).collect::<Vec<_>>().join(", ");
        let text = trf("@MB:NoneOfTheFormulaSlotsHolds", &[&if list.is_empty() { "-".to_string() } else { list }]);
        return Some(Explained { caption, text, fix: None });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::FormulaEntry;

    #[test]
    fn the_name_of_a_missing_formula() {
        let e = "formula 'MyFold3' not found (searched a, b); use --formulas DIR or MB3D_FORMULAS";
        assert_eq!(missing_formula_name(e).as_deref(), Some("MyFold3"));
        assert_eq!(missing_formula_name("formula 'x' is fine"), None);
        assert_eq!(missing_formula_name("other error"), None);
    }

    #[test]
    fn similar_formula_names() {
        let installed: Vec<String> = ["_SphereFold2", "SphereFold", "AmazingBox", "Amazing Box", "Integer Power", "boxIFS", "HeightMapIFS", "HeightMap2IFS"].iter().map(|s| s.to_string()).collect();
        let s = similar_names("SphereFold2", &installed, 5);
        assert_eq!(s.first().map(|s| s.as_str()), Some("_SphereFold2"));
        assert!(s.contains(&"SphereFold".to_string()));
        assert_eq!(similar_names("heightmapifs", &installed, 1), vec!["HeightMapIFS".to_string()]);
        assert!(similar_names("Xyzzy", &installed, 5).is_empty());
    }

    fn scene(names: &[(&str, i32)]) -> Scene {
        crate::formulas::add_formula_dir(std::path::PathBuf::from("M3Formulas-rs"));
        let mut sc = Scene::default();
        sc.formulas = names.iter().map(|(n, its)| FormulaEntry { formula: crate::formulas::lookup(n).unwrap(), iterations: *its }).collect();
        sc
    }

    /// dIFS after other formulas: the error explains, the fix makes a DE
    /// combination that calculates.
    #[test]
    fn difs_and_others_become_a_de_combination() {
        let mut sc = scene(&[("Integer Power", 8), ("Amazing Box", 2), ("SphereTree", 1)]);
        let err = crate::calc::CalcParams::new(&sc).err().unwrap();
        let ex = calc_error(&sc, &err).unwrap();
        assert!(ex.text.contains("SphereTree") && ex.text.contains("Integer Power, Amazing Box"), "{}", ex.text);
        let fix = ex.fix.unwrap();
        assert_eq!(fix, Fix::DeComb { end1: 1, start2: 2, end2: 2 });
        apply_fix(&mut sc, fix);
        crate::calc::CalcParams::new(&sc).unwrap();
        // the dIFS one first works too
        let mut sc = scene(&[("SphereTree", 1), ("Integer Power", 8)]);
        let err = crate::calc::CalcParams::new(&sc).err().unwrap();
        let fix = calc_error(&sc, &err).unwrap().fix.unwrap();
        assert_eq!(fix, Fix::DeComb { end1: 0, start2: 1, end2: 1 });
        apply_fix(&mut sc, fix);
        crate::calc::CalcParams::new(&sc).unwrap();
    }

    /// Kinds taking turns: explained, no fix offered.
    #[test]
    fn difs_taking_turns_with_others_has_no_fix() {
        let sc = scene(&[("Integer Power", 8), ("SphereTree", 1), ("Amazing Box", 2)]);
        let err = crate::calc::CalcParams::new(&sc).err().unwrap();
        let ex = calc_error(&sc, &err).unwrap();
        assert!(ex.fix.is_none());
        assert!(ex.text.contains(&tr("@MB:HereTheDIFSFormulasAndThe")));
    }

    #[test]
    fn no_fractal_in_any_slot_is_explained() {
        let mut sc = scene(&[("Integer Power", 0)]);
        sc.formulas[0].iterations = 0;
        let err = crate::calc::CalcParams::new(&sc).err().unwrap();
        assert!(calc_error(&sc, &err).is_some(), "{err}");
        assert!(calc_error(&sc, "formula option is not valid").unwrap().text.contains("Integer Power"));
        assert!(calc_error(&sc, "something else").is_none());
    }
}
