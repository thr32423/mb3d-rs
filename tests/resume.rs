//! A render stopped halfway and continued from its checkpoint gives the
//! picture of an uninterrupted render, byte for byte (the rows done before
//! the stop are kept, the rest calculated as if nothing had happened).

use mb3d::checkpoint::Policy;
use mb3d::render::{calculate_with, no_stage, open_checkpoint};
use mb3d::scene::Scene;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

// (MB3D_RESUME_DIR is process wide: one test at a time)
static ONE: Mutex<()> = Mutex::new(());

fn scene() -> Scene {
    let mut s = Scene::preset("Integer Power").unwrap();
    s.width = 96;
    s.height = 72;
    s.threads = 3;
    s
}

/// `sc` rendered straight through, with all post passes.
fn straight(sc: &Scene) -> Vec<mb3d::gbuffer::SiLight> {
    calculate_with(sc, &|_, _| {}, &no_stage, &|| false, 2, None, None).unwrap().1
}

/// `sc` stopped when `stop_at(stage, rows done, rows in all)` says so, the
/// checkpoint kept, then continued to the end.
fn stopped_and_continued(sc: &Scene, stop_at: &(dyn Fn(&str, usize, usize) -> bool + Sync), in_march: bool) -> Vec<mb3d::gbuffer::SiLight> {
    let stage_name = Mutex::new(String::new());
    let done = AtomicUsize::new(0);
    let total = AtomicUsize::new(0);
    let want_stop = AtomicBool::new(false);
    let progress = |d: usize, t: usize| {
        done.store(d, Ordering::Relaxed);
        total.store(t, Ordering::Relaxed);
        let s = stage_name.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if stop_at(&s, d, t) {
            want_stop.store(true, Ordering::Relaxed);
        }
    };
    let stage = |name: &str| {
        *stage_name.lock().unwrap_or_else(|e| e.into_inner()) = name.to_string();
        no_stage(name);
    };
    let ck = open_checkpoint(sc, "resume test", Policy::Fresh).expect("a checkpoint");
    let r = calculate_with(sc, &progress, &stage, &|| want_stop.load(Ordering::Relaxed), 2, None, Some(&ck));
    assert_eq!(r.err().as_deref(), Some("cancelled"), "the render should have been stopped");
    ck.keep();
    let ck = open_checkpoint(sc, "resume test", Policy::Continue).expect("the checkpoint again");
    let resumed = ck.resumed();
    let g = calculate_with(sc, &|_, _| {}, &no_stage, &|| false, 2, None, Some(&ck)).unwrap().1;
    ck.finish();
    assert!(resumed > 0, "the checkpoint held no rows");
    // (stopped in a post pass, the checkpoint holds every row of the march)
    assert!(resumed < sc.height as usize || !in_march, "the checkpoint held every row: the stop came too late");
    g
}

fn with_dir(f: impl FnOnce()) {
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let d = std::env::temp_dir().join(format!("mb3d-resume-test-{}", std::process::id()));
    std::env::set_var("MB3D_RESUME_DIR", &d);
    std::env::set_var("MB3D_RESUME_KEEP_SECS", "0");
    #[cfg(feature = "gpu")]
    mb3d::gpu::set_enabled(false);
    f();
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn stopped_in_the_march_and_continued_equals_straight() {
    with_dir(|| {
        let sc = scene();
        let a = straight(&sc);
        let b = stopped_and_continued(&sc, &|stage, done, total| stage.is_empty() && total > 0 && done * 3 >= total, true);
        assert_eq!(a, b);
    });
}

#[test]
fn stopped_in_a_post_pass_and_continued_equals_straight() {
    with_dir(|| {
        let sc = scene();
        let a = straight(&sc);
        let b = stopped_and_continued(&sc, &|stage, done, total| !stage.is_empty() && total > 0 && done * 2 >= total, false);
        assert_eq!(a, b);
    });
}

#[test]
fn stopped_with_other_thread_counts_equals_straight() {
    with_dir(|| {
        let mut sc = scene();
        sc.threads = 1;
        let a = straight(&sc);
        sc.threads = 5;
        let b = stopped_and_continued(&sc, &|stage, done, total| stage.is_empty() && total > 0 && done * 5 >= total * 2, true);
        assert_eq!(a, b);
    });
}
