use mb3d::calc::{de_at, CalcParams};
use mb3d::scene::{CameraOptic, Scene};

fn small(name: &str) -> Scene {
    let mut s = Scene::preset(name).unwrap();
    s.width = 96;
    s.height = 72;
    s
}

#[test]
fn deterministic_and_thread_independent() {
    let mut a = small("Integer Power");
    a.threads = 1;
    let mut b = a.clone();
    b.threads = 3;
    let ra = mb3d::render(&a, &|_, _| {}).unwrap();
    let rb = mb3d::render(&b, &|_, _| {}).unwrap();
    assert_eq!(ra.gbuffer, rb.gbuffer);
    assert_eq!(ra.rgb, rb.rgb);
}

#[test]
fn every_preset_hits_the_object() {
    for name in mb3d::Formula::all_names() {
        let r = mb3d::render(&small(name), &|_, _| {}).unwrap();
        let c = r.coverage();
        assert!(c > 0.05 && c < 0.995, "{name}: coverage {c}");
    }
}

#[test]
fn camera_optics() {
    for optic in [CameraOptic::Planar, CameraOptic::Panorama] {
        let mut s = small("Integer Power");
        s.optic = optic;
        if optic == CameraOptic::Panorama {
            s.mid = [0.0, 0.0, 0.0];
            s.z_start = -2.0;
        }
        let r = mb3d::render(&s, &|_, _| {}).unwrap();
        assert!(r.coverage() > 0.01, "{optic:?}");
    }
}

#[test]
fn bulb_distance_estimate_is_sane() {
    let s = small("Integer Power");
    let p = CalcParams::new(&s).unwrap();
    // origin is inside the bulb
    let (_, its) = de_at(&p, [0.0, 0.0, 0.0]);
    assert_eq!(its, s.iterations);
    // (points exactly on the z axis are degenerate in the triplex formula, as in MB3D)
    // a point at distance ~2 from the origin is clearly outside: the DE
    // (converted to world units) must be positive and smaller than the
    // distance to the bulb's bounding sphere surface plus a margin.
    let (d, its) = de_at(&p, [0.3, 0.2, -2.0]);
    assert!(its < s.iterations);
    assert!(d > 0.2 && d < 1.5, "de = {d}");
}

#[test]
fn analytic_box_de_is_sane() {
    // Amazing Box selects the analytic DE (DE option 11): the estimate from
    // outside the box must be of the right magnitude.
    let s = small("Amazing Box");
    let p = CalcParams::new(&s).unwrap();
    assert!(p.is_custom_de);
    let (d, _) = de_at(&p, [0.0, 0.0, -9.0]);
    assert!(d > 0.5 && d < 6.0, "de = {d}");
}

#[test]
fn custom_formulas_render() {
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/formulas")));
    for name in ["MandyCousin", "CosinePow2", "MengerHyper"] {
        let mut s = Scene::preset(name).unwrap();
        s.width = 64;
        s.height = 48;
        s.z_start = -4.0;
        s.zoom = 0.5;
        let r = mb3d::render(&s, &|_, _| {}).unwrap();
        assert!(r.coverage() > 0.02, "{name}: coverage {}", r.coverage());
    }
    assert_eq!(mb3d::iteration::CUSTOM_ERRORS.load(std::sync::atomic::Ordering::Relaxed), 0);
}

#[test]
fn shadows_darken_and_round_trip() {
    let s = small("Integer Power").apply("shadows = all\nshadow_soft_radius = 2").unwrap();
    let h = s.shadows.unwrap();
    assert_eq!(h.lights, 0x3F);
    let t = s.to_text();
    let s2 = Scene::parse(&t).unwrap();
    assert_eq!(s2.shadows, s.shadows);
    let mut a = s.clone();
    a.threads = 1;
    let mut b = s.clone();
    b.threads = 2;
    let ra = mb3d::render(&a, &|_, _| {}).unwrap();
    let rb = mb3d::render(&b, &|_, _| {}).unwrap();
    assert_eq!(ra.rgb, rb.rgb);
    let shadowed = ra.gbuffer.iter().filter(|g| !g.is_background() && g.shadow & 0x400 != 0).count();
    assert!(shadowed > 0, "no pixel in shadow");
    let mut off = s.clone();
    off.shadows = None;
    let ro = mb3d::render(&off, &|_, _| {}).unwrap();
    let sum = |v: &[u8]| v.iter().map(|&x| x as u64).sum::<u64>();
    assert!(sum(&ra.rgb) < sum(&ro.rgb));
}

#[test]
fn positional_and_volumetric_light() {
    let base = small("Integer Power");
    let text = format!(
        "{}\n[light]\ncolor = #FFB060\nposition = -0.9, 0.3, -1.1\nvisible = 3\n",
        base.to_text().split("\n[light]").next().unwrap()
    );
    let s = Scene::parse(&text).unwrap();
    assert!(s.lighting.lights[0].positional);
    let s2 = Scene::parse(&s.to_text()).unwrap();
    assert_eq!(s2.lighting.lights[0].position, s.lighting.lights[0].position);
    assert_eq!(s2.lighting.lights[0].visible, s.lighting.lights[0].visible);
    let r = mb3d::render(&s, &|_, _| {}).unwrap();
    let sum = |v: &[u8]| v.iter().map(|&x| x as u64).sum::<u64>();
    assert!(sum(&r.rgb) > 0);
    let vl = s.clone().apply("vol_light = 1\ndyn_fog = 56").unwrap();
    let rv = mb3d::render(&vl, &|_, _| {}).unwrap();
    assert!(sum(&rv.rgb) > sum(&r.rgb), "volumetric light adds light");
}

#[test]
fn cutting_plane_and_deao() {
    let s = small("Integer Power");
    let cut = s.clone().apply("cut_z = 0").unwrap();
    assert_eq!(Scene::parse(&cut.to_text()).unwrap().cut_pos, cut.cut_pos);
    let r = mb3d::render(&s, &|_, _| {}).unwrap();
    let rc = mb3d::render(&cut, &|_, _| {}).unwrap();
    // the cut face is flat and coloured as interior
    let inside = rc.gbuffer.iter().filter(|g| !g.is_background() && g.si_gradient >= 32768).count();
    assert!(inside > 50, "cut face pixels: {inside}");
    assert!(rc.coverage() <= r.coverage());
    let d = s.clone().apply("ao = deao\ndeao_quality = 1").unwrap();
    assert!(Scene::parse(&d.to_text()).unwrap().deao.is_some());
    let rd = mb3d::render(&d, &|_, _| {}).unwrap();
    let occluded = rd.gbuffer.iter().filter(|g| !g.is_background() && g.amb_shadow > 2000).count();
    assert!(occluded > 0);
}
