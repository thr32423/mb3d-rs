//! A Menger formula in Julia mode after the same scene in Julia off, in one
//! process (the program): the compiled loop kept for Julia off must not be
//! taken for Julia on (reported by Geoffrey N. Barnes). Its own test file:
//! MB3D_JIT is read once per process.

use mb3d::scene::Scene;

#[test]
fn julia_on_after_julia_off_adds_the_constant() {
    // the compiled loop kept whatever its timing
    std::env::set_var("MB3D_JIT", "force");
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/formulas")));
    let mut off = Scene::preset("MengerHyper").unwrap();
    off.width = 64;
    off.height = 48;
    off.z_start = -4.0;
    off.zoom = 0.5;
    off.threads = 1;
    let mut on = off.clone();
    on.julia = true;
    on.julia_c = [0.4, 0.3, 0.2, 0.0];
    let r_off = mb3d::render(&off, &|_, _| {}).unwrap();
    let r_on = mb3d::render(&on, &|_, _| {}).unwrap();
    assert_ne!(r_off.rgb, r_on.rgb, "Julia on after Julia off gives the Julia off picture");
    // and Julia off again: its compiled loop, the same picture
    let r_off2 = mb3d::render(&off, &|_, _| {}).unwrap();
    assert_eq!(r_off.rgb, r_off2.rgb);
}
