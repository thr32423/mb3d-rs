//! Animation: interpolated frames render like their keyframes, and the
//! light values are blended between keyframes.

use mb3d::anim::{interpolate_linear, Animation, Interpolation, Keyframe};
use mb3d::scene::Scene;

fn small() -> Scene {
    let mut s = Scene::preset("Integer Power").unwrap();
    s.width = 80;
    s.height = 60;
    s.threads = 2;
    s
}

fn mean(rgb: &[u8]) -> [f64; 3] {
    let n = (rgb.len() / 3) as f64;
    let mut m = [0.0; 3];
    for p in rgb.chunks(3) {
        for c in 0..3 {
            m[c] += p[c] as f64 / n;
        }
    }
    m
}

fn max_diff(a: &[u8], b: &[u8]) -> u8 {
    a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0)
}

#[test]
fn identical_keyframes_render_like_the_scene() {
    let s = small();
    let plain = mb3d::render(&s, &|_, _| {}).unwrap();
    let f = interpolate_linear(&s, &s, 0.5);
    assert!(f.light_blend.is_some());
    let r = mb3d::render(&f, &|_, _| {}).unwrap();
    assert_eq!(plain.gbuffer, r.gbuffer);
    assert!(max_diff(&plain.rgb, &r.rgb) <= 1, "{}", max_diff(&plain.rgb, &r.rgb));
}

#[test]
fn first_and_last_frame_are_the_keyframes() {
    let a = small();
    let mut b = small();
    b.zoom *= 1.5;
    b.lighting.lights[0].color = [40, 90, 255];
    let anim = Animation {
        keyframes: vec![Keyframe::new(a.clone(), 4), Keyframe::new(b.clone(), 4)],
        width: 80,
        height: 60,
        interpolation: Interpolation::Linear,
        ..Default::default()
    };
    let r0 = mb3d::render(&anim.frame_scene(0).unwrap(), &|_, _| {}).unwrap();
    let ra = mb3d::render(&a, &|_, _| {}).unwrap();
    assert_eq!(r0.gbuffer, ra.gbuffer);
    assert!(max_diff(&r0.rgb, &ra.rgb) <= 1);
    let r4 = mb3d::render(&anim.frame_scene(4).unwrap(), &|_, _| {}).unwrap();
    let rb = mb3d::render(&b, &|_, _| {}).unwrap();
    assert_eq!(r4.rgb, rb.rgb);
}

#[test]
fn light_colour_and_switching_fade() {
    let mut a = small();
    let mut b = small();
    a.ao = None;
    b.ao = None;
    for l in a.lighting.lights.iter_mut().chain(b.lighting.lights.iter_mut()) {
        l.on = false;
    }
    a.lighting.lights[0].on = true;
    a.lighting.lights[0].color = [255, 0, 0];
    b.lighting.lights[0].on = true;
    b.lighting.lights[0].color = [0, 0, 255];
    // a second light only in the second keyframe
    b.lighting.lights[1] = b.lighting.lights[0].clone();
    b.lighting.lights[1].color = [0, 255, 0];
    b.lighting.lights[1].x_angle = 0.8;
    let m = |s: &Scene| mean(&mb3d::render(s, &|_, _| {}).unwrap().rgb);
    let (ma, mb) = (m(&a), m(&b));
    let mm = m(&interpolate_linear(&a, &b, 0.5));
    for c in 0..3 {
        let lo = ma[c].min(mb[c]);
        let hi = ma[c].max(mb[c]);
        assert!(mm[c] > lo + 0.2 * (hi - lo) && mm[c] < hi - 0.2 * (hi - lo), "channel {c}: {ma:?} {mm:?} {mb:?}");
    }
}

#[test]
fn light_direction_turns_on_the_sphere() {
    let mut a = small();
    a.ao = None;
    a.lighting.lights[0].x_angle = -1.0;
    // on the equator the great circle midpoint is the middle angle
    a.lighting.lights[0].y_angle = 0.0;
    let mut b = a.clone();
    b.lighting.lights[0].x_angle = 1.0;
    let mut c = a.clone();
    c.lighting.lights[0].x_angle = 0.0;
    let half = mb3d::render(&interpolate_linear(&a, &b, 0.5), &|_, _| {}).unwrap();
    let mid = mb3d::render(&c, &|_, _| {}).unwrap();
    // the slerped direction is the middle angle (single precision differences)
    assert!(max_diff(&half.rgb, &mid.rgb) <= 3, "{}", max_diff(&half.rgb, &mid.rgb));
}
