//! Mandelbulber settings files opened as scenes: the camera, the formulas
//! and the notes about what was not carried over.

use mb3d::mandelbulber::import;
use mb3d::math::{dot, len, normalize, sub};

fn text(name: &str) -> String {
    std::fs::read_to_string(format!("tests/data/{name}")).unwrap()
}

fn option(f: &mb3d::formulas::Formula, key: &str) -> f64 {
    let k = key.to_ascii_lowercase();
    f.options().iter().find(|(n, _)| n.to_ascii_lowercase().contains(&k)).map(|(_, v)| *v).unwrap_or_else(|| panic!("no option {key} in {:?}", f.options()))
}

#[test]
fn mandelbox_file_becomes_an_amazing_box_scene() {
    mb3d::formulas::add_formula_dir("M3Formulas-rs".into());
    let im = import(&text("mandelbox.fract")).unwrap();
    let sc = &im.scene;
    assert_eq!((sc.width, sc.height, sc.iterations), (400, 300, 30));
    assert_eq!(sc.formulas.len(), 1);
    let f = &sc.formulas[0].formula;
    assert_eq!(f.name(), "Amazing Box");
    assert_eq!(option(f, "Scale"), -1.5);
    assert_eq!(option(f, "Fold"), 1.1);
    assert_eq!(option(f, "Min R"), 0.6);
    // detail level 2 is a DE stop of 0.5, DE factor 0.8 a ray step of 0.4
    assert!((sc.de_stop - 0.5).abs() < 1e-9 && (sc.z_step_div - 0.4).abs() < 1e-9);
    assert_eq!(sc.lighting.amb_top, [0x20, 0x40, 0x80]);
    assert_eq!(sc.lighting.amb_bottom, [255, 255, 255]);
    assert!(im.notes[0].contains("1 formula carried over"), "{:?}", im.notes);

    // the camera: the eye where Mandelbulber's camera was, looking at the
    // target, the top vector up
    let eye = mb3d::splat::eye(sc).unwrap();
    let want = [3.0, -6.0, 2.0];
    let off = len(&sub(eye, want));
    assert!(off < 0.02, "eye {eye:?} is {off} from {want:?}");
    let (_, dir) = mb3d::splat::start_plane(sc).unwrap();
    let to_target = normalize(sub([0.0; 3], want));
    assert!(dot(&dir, &to_target) > 0.9999, "view direction {dir:?} vs {to_target:?}");
    let up = [-sc.vgrads[1][0], -sc.vgrads[1][1], -sc.vgrads[1][2]];
    assert!(dot(&up, &[-0.1277753, 0.2555506, 0.958314]) > 0.9999, "up {up:?}");
    // the vertical field of view from the horizontal 90 degrees at 4:3
    assert!((sc.fov_y - 2.0f64 * (45.0f64.to_radians().tan() * 0.75).atan().to_degrees()).abs() < 1e-6);

    // the box is in the picture
    let mut small = sc.clone();
    small.width = 96;
    small.height = 72;
    let r = mb3d::render(&small, &|_, _| {}).unwrap();
    let c = r.coverage();
    assert!(c > 0.05 && c < 0.95, "coverage {c}");
}

#[test]
fn hybrid_slots_julia_and_notes() {
    mb3d::formulas::add_formula_dir("M3Formulas-rs".into());
    let im = import(&text("hybrid_julia.fract")).unwrap();
    let sc = &im.scene;
    let names: Vec<(String, i32)> = sc.formulas.iter().map(|f| (f.formula.name(), f.iterations)).collect();
    // the transform 1004 has no counterpart
    assert_eq!(names[0], ("Amazing Box".to_string(), 2), "{names:?}");
    assert_eq!(names[1], ("Integer Power".to_string(), 1), "{names:?}");
    assert_eq!(option(&sc.formulas[1].formula, "Power"), 2.0);
    assert!(sc.julia && sc.julia_c == [-1.0, 0.0, 0.0, 0.0]);
    assert!(im.notes.iter().any(|n| n.contains("a transform") && n.contains("left out")), "{:?}", im.notes);
    // Menger3 is a formula file of the MB3D bundle, not of this repository:
    // where it is installed it gets the scale of the file, elsewhere the
    // notes say that it is missing and the slot is left out
    if mb3d::formulas::lookup("Menger3").is_ok() {
        assert_eq!(names.len(), 3, "{names:?}");
        assert_eq!(names[2].0, "Menger3");
        assert_eq!(option(&sc.formulas[2].formula, "Scale"), 2.5);
        assert!(im.notes[0].contains("3 formulas carried over, 1 left out"), "{:?}", im.notes);
    } else {
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(im.notes.iter().any(|n| n.contains("Menger3") && n.contains("not installed")), "{:?}", im.notes);
        assert!(im.notes[0].contains("2 formulas carried over, 2 left out"), "{:?}", im.notes);
    }
}

#[test]
fn opens_through_the_scene_loaders() {
    let (sc, notes) = mb3d::animfile::scene_from_bytes("a.fract", text("mandelbox.fract").into_bytes()).unwrap();
    assert_eq!(sc.formulas[0].formula.name(), "Amazing Box");
    assert!(!notes.is_empty());
    // by the header alone, whatever the name
    let (sc2, _) = mb3d::animfile::scene_from_bytes("pasted.txt", text("mandelbox.fract").into_bytes()).unwrap();
    assert_eq!(sc2.zoom, sc.zoom);
}

#[test]
fn a_file_of_unknown_formulas_is_refused() {
    let e = import("# Mandelbulber settings file\n[main_parameters]\nformula_1 4;\n").unwrap_err();
    assert!(e.contains("none of the file's formulas"), "{e}");
}
