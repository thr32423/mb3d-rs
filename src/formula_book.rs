//! The formula book: what the program can say about each formula in
//! words, for the Formula book window ([`crate::app::formula_book_form`])
//! and the command line.  The Mandelbulb Maniacs group asks for
//! "plain-language documentation of each formula and parameter"; what
//! there is to work from: the formula's kind (its DE option, which decides
//! how it combines), the author's description in the `.m3f` file (most
//! files have one), the names of its values (which follow conventions:
//! Scale, Fold, Min R, ...), and the example parameter files that use it.
//! The texts are labels, so they come in every language of the program.

use crate::formulas::Formula;
use crate::scene::Scene;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The kind of a formula, from its DE option (the lists of MB3D's
/// Formulas window sort by the same numbers).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    /// numerical 3D distance estimate
    Escape3d,
    /// analytic 3D distance estimate (DE option 2, 11)
    Escape3dAnalytic,
    /// 4D, numerical (DE option 4)
    Escape4d,
    /// 4D, analytic (5, 6)
    Escape4dAnalytic,
    /// changes the point only (-1, -2)
    Transformation,
    /// dIFS shape (20)
    DifsShape,
    /// dIFS transformation (21, 22)
    DifsTransformation,
}

pub const CATEGORIES: [Category; 7] = [
    Category::Escape3d,
    Category::Escape3dAnalytic,
    Category::Escape4d,
    Category::Escape4dAnalytic,
    Category::Transformation,
    Category::DifsShape,
    Category::DifsTransformation,
];

impl Category {
    pub fn of(de_option: i32) -> Category {
        match de_option {
            2 | 11 => Category::Escape3dAnalytic,
            4 => Category::Escape4d,
            5 | 6 => Category::Escape4dAnalytic,
            -1 | -2 => Category::Transformation,
            20 => Category::DifsShape,
            21 | 22 => Category::DifsTransformation,
            _ => Category::Escape3d,
        }
    }

    /// The label of the category's name (the filter list).
    pub fn label(self) -> &'static str {
        match self {
            Category::Escape3d => "@MB:N3DEscapeTimeFormulas",
            Category::Escape3dAnalytic => "@MB:N3DFormulasWithAnalyticDEFast",
            Category::Escape4d => "@MB:N4DFormulas",
            Category::Escape4dAnalytic => "@MB:N4DFormulasWithAnalyticDE",
            Category::Transformation => "@MB:TransformationsOnlyInCombination",
            Category::DifsShape => "@MB:DIFSShapes",
            Category::DifsTransformation => "@MB:DIFSTransformations",
        }
    }

    /// The label of the sentence that says what the kind means.
    pub fn explanation(self) -> &'static str {
        match self {
            Category::Escape3d => "@MB:A3DEscapeTimeFormulaWith",
            Category::Escape3dAnalytic => "@MB:A3DEscapeTimeFormulaWith2",
            Category::Escape4d => "@MB:A4DFormulaTheFourthCoordinate",
            Category::Escape4dAnalytic => "@MB:A4DFormulaWithAnAnalytic",
            Category::Transformation => "@MB:ATransformationItChangesThePoint",
            Category::DifsShape => "@MB:ADIFSShapeADistanceEstimated",
            Category::DifsTransformation => "@MB:ADIFSTransformationItBendsRepeats",
        }
    }

    pub fn is_4d(self) -> bool {
        matches!(self, Category::Escape4d | Category::Escape4dAnalytic)
    }
}

/// A formula's page of the book.
#[derive(Clone, Debug)]
pub struct Topic {
    pub name: String,
    /// one of the program's own formulas (else a `.m3f` file)
    pub builtin: bool,
    pub file: Option<PathBuf>,
    pub category: Category,
    /// the author's text from the formula file
    pub description: String,
    /// the values with their defaults
    pub options: Vec<(String, f64)>,
}

/// Every formula the Formulas window lists: the built-in ones and the
/// formula files, by name.
pub fn topics() -> Vec<Topic> {
    let mut v: Vec<Topic> = Formula::all_names()
        .iter()
        .filter_map(|n| Formula::default_for(n))
        .map(|f| Topic {
            name: f.name(),
            builtin: true,
            file: None,
            category: Category::of(f.de_option()),
            description: crate::app::formula_form::description(&f),
            options: f.options(),
        })
        .collect();
    let builtin: Vec<String> = v.iter().map(|t| t.name.to_ascii_lowercase()).collect();
    let custom = crate::formulas::list_custom_with_de();
    let files: std::collections::HashSet<String> = custom.iter().map(|(n, _)| n.to_ascii_lowercase()).collect();
    for (name, de) in custom {
        if builtin.contains(&name.to_ascii_lowercase()) || crate::app::formula_form::excluded_formula(&name, &files) {
            continue;
        }
        // (the file is read for its description and values; a broken one
        // is listed with its kind alone)
        let (description, options) = match crate::formulas::load_custom(&name) {
            Ok(m) => (m.description.trim().to_string(), m.options.iter().map(|o| (o.name.clone(), o.default)).collect()),
            Err(_) => (String::new(), Vec::new()),
        };
        v.push(Topic { name: name.clone(), builtin: false, file: crate::formulas::custom_path(&name), category: Category::of(de), description, options });
    }
    v.sort_by_cached_key(|t| t.name.to_lowercase());
    v
}

/// The label that explains a value by its name, for the names that follow
/// the usual conventions (Scale, Fold, Min R, ...); None for a name the
/// book cannot say anything about.
pub fn option_explanation(name: &str) -> Option<&'static str> {
    let n = name.trim().to_ascii_lowercase().replace(['_', '-'], " ");
    let has = |w: &str| n.contains(w);
    // the order matters: "Colour scale" is about colour, "CScale X" is
    // the fold's offset, "Scale vary" is the vary
    let id = if has("col") {
        "@MB:ChangesTheColouringOnlyNotThe"
    } else if has("vary") {
        "@MB:HowTheValueChangesWithThe"
    } else if has("cscale") || has("c scale") {
        "@MB:TheFoldSOffsetOnOne"
    } else if has("min r") || has("minr") || has("min radius") {
        "@MB:TheRadiusOfTheSphericalFold"
    } else if has("smooth") {
        "@MB:RoundsTheEdgesSmallValuesKeep"
    } else if has("thick") {
        "@MB:TheWallThicknessOfAHollow"
    } else if has("level") || has("iteration") || n == "its" {
        "@MB:HowManyCopiesWithinCopiesThe"
    } else if has("inv") {
        "@MB:ASphereInversionOfTheSpace"
    } else if has("julia") {
        "@MB:TheJuliaConstantOfThisFormula"
    } else if has("pow") {
        "@MB:TheExponentOfTheBulb8"
    } else if has("de ") || n.starts_with("de") || has(" de") || has("destop") {
        "@MB:FineTuningOfTheDistanceEstimate"
    } else if has("angle") || has("rot") {
        "@MB:ARotationAppliedEachIterationIn"
    } else if has("fold") {
        "@MB:TheBoxFoldSLimitWhat"
    } else if has("scale") {
        "@MB:StretchesTheSpaceEachIterationAround"
    } else if has("mode") || has("shape") || has("type") || has("switch") || has("swap") || has(" or ") {
        "@MB:ChoosesAVariantOfTheFormula"
    } else if n.starts_with("w ") || has(" w ") || n == "w" || has("w add") {
        "@MB:TheFourthCoordinateThisStartsIt"
    } else if has("mul") {
        "@MB:ScalesOneCoordinateEachIteration1"
    } else if has("size") || has("radius") {
        "@MB:TheSizeOfTheBaseShape"
    } else if has("off") || has("add") || has("shift") {
        "@MB:AConstantAddedEachIterationIt"
    } else {
        return None;
    };
    Some(id)
}

/// The labels of the hints for a formula with `examples` example files.
pub fn hints(t: &Topic, examples: usize) -> Vec<&'static str> {
    let mut v = Vec::new();
    match t.category {
        Category::Escape3dAnalytic | Category::Escape3d if t.options.len() <= 4 && examples > 0 => v.push("@MB:GoodFirstFormulaFewValuesFast"),
        Category::Escape4d => v.push("@MB:PatienceA4DFormulaWithNumerical"),
        Category::Transformation | Category::DifsTransformation => v.push("@MB:ATransformationShowsNothingAlonePut"),
        _ => {}
    }
    if t.category.is_4d() {
        v.push("@MB:WatchThe4DRotationFormulasWindow");
    }
    if examples > 0 {
        v.push("@MB:StartFromAnExampleOpenOne");
    }
    if t.options.len() >= 8 {
        v.push("@MB:ManyValuesChangeOneAtA");
    }
    v
}

/// The example files of a folder by the formulas they use: the lower-case
/// formula name to the files (sorted by name).  A file that does not open
/// is left out.
pub fn example_index(folders: &[PathBuf]) -> HashMap<String, Vec<PathBuf>> {
    let mut map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    for folder in folders {
        let Ok(rd) = std::fs::read_dir(folder) else { continue };
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && matches!(p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref(), Some("m3p" | "m3i" | "txt")))
            .collect();
        files.sort();
        for p in files {
            let canon = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
            if !seen.insert(canon) {
                continue;
            }
            let Ok((sc, _)) = crate::animfile::load_scene(&p) else { continue };
            let mut names: Vec<String> = sc.formulas.iter().filter(|f| f.iterations > 0).map(|f| f.formula.name().to_ascii_lowercase()).collect();
            names.sort();
            names.dedup();
            for n in names {
                map.entry(n).or_default().push(p.clone());
            }
        }
    }
    map
}

/// The scene of a formula alone at its default values, for its picture
/// when no example file uses it.
pub fn starter_scene(name: &str) -> Result<Scene, String> {
    let f = crate::formulas::lookup(name)?;
    let cat = Category::of(f.de_option());
    let mut s = match cat {
        // a transformation shows nothing alone: it works on a shape, the
        // bulb (or a dIFS box) in the slot after it
        Category::Transformation | Category::DifsTransformation => {
            let shape = if cat == Category::Transformation { "Integer Power" } else { "boxIFS" };
            let mut s = Scene::preset(shape)?;
            let shape = s.formulas[0].clone();
            s.formulas = vec![crate::scene::FormulaEntry { formula: f, iterations: 1 }, shape];
            s
        }
        _ => Scene::preset(name)?,
    };
    s.width = 320;
    s.height = 240;
    Ok(s)
}

/// The file of the starter scene in the cache folder (written when
/// missing), so the preview queue can calculate it like any file.
pub fn starter_file(name: &str) -> Result<PathBuf, String> {
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let p = crate::appdirs::cache_folder("formula-book").join(format!("{safe}.txt"));
    let text = starter_scene(name)?.to_text();
    if std::fs::read_to_string(&p).ok().as_deref() != Some(&text) {
        std::fs::write(&p, &text).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok(p)
}

/// The page of a formula as plain text (the window's memo, the command
/// line): kind, description, values, hints, examples.
pub fn page_text(t: &Topic, examples: &[PathBuf]) -> String {
    use crate::vcl::lang::{tr, trf};
    let mut lines: Vec<String> = Vec::new();
    lines.push(match &t.file {
        Some(p) if !t.builtin => trf("@MB:FormulaFile0", &[&p.display().to_string()]),
        _ => tr("@MB:BuiltInFormula"),
    });
    lines.push(trf("@MB:Kind0", &[&tr(t.category.label())]));
    lines.push(tr(t.category.explanation()));
    lines.push(String::new());
    if t.description.is_empty() {
        lines.push(tr("@MB:NoDescriptionFromTheAuthor"));
    } else {
        lines.push(t.description.replace("\r\n", "\n"));
    }
    if !t.options.is_empty() {
        lines.push(String::new());
        lines.push(tr("@MB:Values"));
        for (k, v) in &t.options {
            match option_explanation(k) {
                Some(id) => lines.push(format!("  {k} = {v}: {}", tr(id))),
                None => lines.push(format!("  {k} = {v}")),
            }
        }
    }
    let h = hints(t, examples.len());
    if !h.is_empty() {
        lines.push(String::new());
        lines.push(tr("@MB:Hints"));
        for id in h {
            lines.push(format!("  - {}", tr(id)));
        }
    }
    lines.push(String::new());
    if examples.is_empty() {
        lines.push(tr("@MB:NoExampleFileUsesItTry"));
    } else {
        let names: Vec<String> = examples.iter().map(|p| file_title(p)).collect();
        lines.push(trf("@MB:ExampleFilesUsingIt0", &[&names.join(", ")]));
    }
    lines.join("\n")
}

/// A file's name without its extension.
pub fn file_title(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_follow_the_de_option() {
        assert_eq!(Category::of(0), Category::Escape3d);
        assert_eq!(Category::of(11), Category::Escape3dAnalytic);
        assert_eq!(Category::of(4), Category::Escape4d);
        assert_eq!(Category::of(6), Category::Escape4dAnalytic);
        assert_eq!(Category::of(-1), Category::Transformation);
        assert_eq!(Category::of(20), Category::DifsShape);
        assert_eq!(Category::of(22), Category::DifsTransformation);
    }

    #[test]
    fn value_names_are_explained_by_convention() {
        assert_eq!(option_explanation("Scale"), Some("@MB:StretchesTheSpaceEachIterationAround"));
        assert_eq!(option_explanation("CScale X"), Some("@MB:TheFoldSOffsetOnOne"));
        assert_eq!(option_explanation("Scale vary"), Some("@MB:HowTheValueChangesWithThe"));
        assert_eq!(option_explanation("Colour scale"), Some("@MB:ChangesTheColouringOnlyNotThe"));
        assert_eq!(option_explanation("Min R"), Some("@MB:TheRadiusOfTheSphericalFold"));
        assert_eq!(option_explanation("Box fold"), Some("@MB:TheBoxFoldSLimitWhat"));
        assert_eq!(option_explanation("Integer power"), Some("@MB:TheExponentOfTheBulb8"));
        assert_eq!(option_explanation("Z multiplier"), Some("@MB:ScalesOneCoordinateEachIteration1"));
        assert_eq!(option_explanation("W add"), Some("@MB:TheFourthCoordinateThisStartsIt"));
        assert_eq!(option_explanation("DE offset"), Some("@MB:FineTuningOfTheDistanceEstimate"));
        assert_eq!(option_explanation("Sphere or Cylinder"), Some("@MB:ChoosesAVariantOfTheFormula"));
        assert_eq!(option_explanation("Fix"), None);
    }

    #[test]
    fn built_in_topics_and_their_hints() {
        let v = topics();
        let abox = v.iter().find(|t| t.name == "Amazing Box").expect("Amazing Box");
        assert!(abox.builtin && abox.category == Category::Escape3dAnalytic);
        assert_eq!(abox.options.len(), 3);
        assert!(hints(abox, 2).contains(&"@MB:GoodFirstFormulaFewValuesFast"));
        assert!(!hints(abox, 0).contains(&"@MB:GoodFirstFormulaFewValuesFast"));
        let quat = v.iter().find(|t| t.name == "Quaternion").unwrap();
        assert!(hints(quat, 0).contains(&"@MB:PatienceA4DFormulaWithNumerical"));
        assert!(hints(quat, 0).contains(&"@MB:WatchThe4DRotationFormulasWindow"));
        let text = page_text(abox, &[]);
        assert!(text.contains("Scale = 2") && text.contains("Min R = 0.5"));
    }

    #[test]
    fn starter_scene_shows_the_formula() {
        let s = starter_scene("Amazing Box").unwrap();
        assert_eq!(s.formulas[0].formula.name(), "Amazing Box");
        assert!(starter_scene("no such formula").is_err());
        // a transformation gets a shape behind it
        crate::formulas::add_formula_dir("M3Formulas-rs".into());
        if crate::formulas::lookup("_AddC").is_ok() {
            let s = starter_scene("_AddC").unwrap();
            assert_eq!(s.formulas.len(), 2);
            assert_eq!(s.formulas[1].formula.name(), "Integer Power");
        }
    }
}
