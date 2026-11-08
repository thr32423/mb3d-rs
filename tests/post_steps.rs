//! The step settings of the passes after the main one, kept with the
//! parameters and used by the shadow pass.

use mb3d::scene::{PostSteps, Scene};

#[test]
fn kept_in_text_and_mb3d_parameters() {
    let mut s = Scene::preset("Amazing Box").unwrap();
    s.z_step_div = 0.1;
    s.post_steps = Some(PostSteps { raystep: 0.5, de_stop: 1.5 });
    let t = s.to_text();
    assert!(t.contains("post_raystep = 0.5") && t.contains("post_de_stop = 1.5"), "{t}");
    let back = Scene::parse(&t).unwrap();
    assert_eq!(back.post_steps, s.post_steps);
    // through MB3D's parameters (the mb3d-rs block)
    let raw = mb3d::m3p::write(&s);
    let m = mb3d::m3p::parse(&raw).unwrap();
    assert_eq!(m.scene.post_steps, s.post_steps);
    // the post scene has the post values, the main pass keeps its own
    let ps = s.post_scene().unwrap();
    assert_eq!((ps.z_step_div, ps.de_stop), (0.5, 1.5));
    assert!(ps.post_steps.is_none());
    s.post_steps = Some(PostSteps::of(&s));
    assert!(s.post_scene().is_none());
}

#[test]
fn the_shadow_pass_renders_with_post_steps() {
    let mut s = Scene::preset("Amazing Box").unwrap();
    s.width = 64;
    s.height = 48;
    s.shadows = Some(Default::default());
    let plain = mb3d::render(&s, &|_, _| {}).unwrap();
    // the same main pass, the shadows with a coarser step: the surface is
    // the same, the picture still has a shadow pass
    s.post_steps = Some(PostSteps { raystep: s.z_step_div * 4.0, de_stop: s.de_stop });
    let post = mb3d::render(&s, &|_, _| {}).unwrap();
    let same_surface = plain.gbuffer.iter().zip(&post.gbuffer).all(|(a, b)| a.zpos_fine == b.zpos_fine);
    assert!(same_surface);
    assert_eq!(post.coverage(), plain.coverage());
}
