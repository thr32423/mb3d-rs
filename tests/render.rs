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

#[test]
fn interpolation_hybrid_blends_two_formulas() {
    // weights 1 : 0 give the first formula alone, for the gradient DE
    // (Integer Power) and the analytic one (Amazing Box)
    for (a, b) in [("Integer Power", "Real Power"), ("Amazing Box", "Integer Power")] {
        let single = small(a);
        let mut ip = single.clone();
        ip.formulas.truncate(1);
        ip.formulas[0].iterations = 1;
        ip.formulas.push(mb3d::scene::FormulaEntry { formula: mb3d::formulas::lookup(b).unwrap(), iterations: 1 });
        ip.interpolation = Some([2.0, 0.0]);
        let p1 = CalcParams::new(&single).unwrap();
        let p2 = CalcParams::new(&ip).unwrap();
        assert_eq!(p1.is_custom_de, p2.is_custom_de, "{a}");
        for pos in [[0.3, 0.2, -2.0], [0.7, -0.4, 0.5], [0.0, 1.5, -6.0]] {
            let (d1, i1) = de_at(&p1, pos);
            let (d2, i2) = de_at(&p2, pos);
            assert_eq!(i1, i2, "{a} {pos:?}");
            assert!((d1 - d2).abs() <= 1e-9 * d1.abs().max(1e-3), "{a} {pos:?}: {d1} {d2}");
        }
        // a real blend renders and differs
        ip.interpolation = Some([0.5, 0.5]);
        let p3 = CalcParams::new(&ip).unwrap();
        let (d3, _) = de_at(&p3, [0.7, -0.4, 0.5]);
        assert!(d3.is_finite());
        let r = mb3d::render(&ip, &|_, _| {}).unwrap();
        assert!(r.coverage() > 0.01, "{a}");
        // .m3s and .m3p round trip
        let back = Scene::parse(&ip.to_text()).unwrap();
        assert_eq!(back.interpolation, Some([0.5, 0.5]));
        let m3p = mb3d::m3p::write(&ip);
        let l = mb3d::m3p::parse(&m3p).unwrap();
        assert_eq!(l.scene.interpolation, Some([0.5, 0.5]));
        assert_eq!(l.scene.formulas[1].formula.name(), b);
    }
}

#[test]
fn slices_2d_and_color_on_iteration() {
    let mut s = small("Integer Power");
    s.iterations = 20;
    let mut imgs = Vec::new();
    for plane in ["start", "mid", "end"] {
        let t = s.clone().apply(&format!("slice_2d = {plane}")).unwrap();
        let r = mb3d::render(&t, &|_, _| {}).unwrap();
        // a 2D calculation fills every pixel with a flat normal
        assert!(r.gbuffer.iter().all(|g| g.normal[2] == -32768 && !g.is_background()));
        imgs.push(r.gbuffer.iter().map(|g| g.si_gradient).collect::<Vec<_>>());
        assert_eq!(Scene::parse(&t.to_text()).unwrap().slice_2d, t.slice_2d);
        let back = mb3d::m3p::parse(&mb3d::m3p::write(&t)).unwrap().scene;
        assert_eq!(back.slice_2d, t.slice_2d);
    }
    assert_ne!(imgs[0], imgs[1]);
    // the middle plane cuts the bulb: some pixels reach the iteration limit
    assert!(imgs[1].iter().any(|&g| g >= 32768) && imgs[1].iter().any(|&g| g < 32768));

    let c = s.clone().apply("color_on_iteration = 2\ncolor_option = 0").unwrap();
    assert_eq!(c.color_on_it, 3);
    assert_eq!(Scene::parse(&c.to_text()).unwrap().color_on_it, 3);
    assert_eq!(mb3d::m3p::parse(&mb3d::m3p::write(&c)).unwrap().scene.color_on_it, 3);
    let a = mb3d::render(&s, &|_, _| {}).unwrap();
    let b = mb3d::render(&c, &|_, _| {}).unwrap();
    let ot = |r: &mb3d::render::RenderResult| r.gbuffer.iter().filter(|g| !g.is_background()).map(|g| g.otrap as u64).sum::<u64>();
    assert_ne!(ot(&a), ot(&b), "colour on iteration 2 changes the orbit trap colouring");
}

#[test]
fn analytic_4d_de_uses_the_4d_loop() {
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/formulas")));
    let mut s = Scene::preset("ABoxMod4d").unwrap();
    s.width = 64;
    s.height = 48;
    let p = CalcParams::new(&s).unwrap();
    assert!(p.is_custom_de && p.mode == mb3d::iteration::HybridMode::Alt4D);
    // CalcDEanalytic with doHybrid4DDEPas: sqrt(Rout) / |Deriv1| (the
    // derivative is not in w, which is the 4th coordinate)
    let pos = [0.31, 0.27, -4.0];
    let (d, _) = de_at(&p, pos);
    let mut it = p.new_iteration();
    it.c = pos;
    let raw = it.hybrid_4d_de(&p.slots);
    assert!((raw - it.rout.sqrt() / it.deriv1.abs()).abs() < 1e-12 && it.deriv1 > 1.0);
    assert!((d - raw * p.d_de_scale as f64 * p.step_width).abs() < 1e-9 * d.abs(), "de = {d}, raw {raw}");
    let r = mb3d::render(&s, &|_, _| {}).unwrap();
    assert!(r.coverage() > 0.02, "coverage {}", r.coverage());
}

