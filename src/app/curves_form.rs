//! The animation's curves (an idea from Mandelbulber, not in MB3D): a
//! value of the scene (the zoom, the position, the rotation, ...) over all
//! frames of the animation as it is interpolated, the keyframes as squares
//! on it; a square dragged up or down, or a value typed for the chosen
//! keyframe, changes that keyframe.

use super::util::{fts, ptf};
use super::Mb3d;
use crate::curves::{self, PARAMS};
use crate::vcl::bitmap::Bitmap;
use crate::vcl::form::MouseButton;
use crate::vcl::lang::tr;
use crate::vcl::{Ev, Event, Ui};

pub const F: &str = "CurvesFrm";

#[derive(Default)]
pub struct State {
    /// the value shown (`PARAMS`)
    param: usize,
    /// the chosen keyframe
    sel: Option<usize>,
    /// a square is being dragged
    drag: bool,
    /// the value of every frame, the keyframes' (frame, value), the range
    /// drawn
    values: Vec<f64>,
    keys: Vec<(usize, f64)>,
    range: (f64, f64),
}

/// Opens the window (the Animation window's Curves button).
pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    let items: Vec<String> = PARAMS.iter().map(|p| tr(p.label).trim_end_matches([':', ' ', '：']).to_string()).collect();
    ui.set_items(F, "ParamCB", items);
    ui.set_item_index(F, "ParamCB", app.curves.param as i32);
    ui.show(F);
    update(app, ui);
}

/// The curve of the chosen value anew (after a change of the keyframes).
pub fn update(app: &mut Mb3d, ui: &mut Ui) {
    if !ui.showing(F) {
        return;
    }
    let a = &app.anim.anim;
    let p = &PARAMS[app.curves.param.min(PARAMS.len() - 1)];
    let (values, keys) = curves::curve(a, p);
    let all = values.iter().chain(keys.iter().map(|k| &k.1)).filter(|v| v.is_finite());
    let (mut lo, mut hi) = all.fold((f64::MAX, f64::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    if lo > hi {
        (lo, hi) = (0.0, 1.0);
    }
    let pad = ((hi - lo) * 0.08).max(1e-9 * hi.abs().max(1.0));
    let st = &mut app.curves;
    st.values = values;
    st.keys = keys;
    st.range = (lo - pad, hi + pad);
    if st.sel.is_some_and(|s| s >= st.keys.len()) {
        st.sel = None;
    }
    let short = |v: f64| format!("{v:.4}").trim_end_matches('0').trim_end_matches('.').to_string();
    ui.set_caption(F, "MaxLbl", &short(st.range.1));
    ui.set_caption(F, "MinLbl", &short(st.range.0));
    show_key(app, ui);
    draw(app, ui);
}

fn show_key(app: &Mb3d, ui: &mut Ui) {
    let st = &app.curves;
    match st.sel.and_then(|k| st.keys.get(k).map(|v| (k, v.1))) {
        Some((k, v)) => {
            ui.set_caption(F, "KeyLbl", &crate::vcl::lang::trf("@MB:Keyframe2", &[&(k + 1)]));
            ui.set_text(F, "KeyValEdit", &if v.is_finite() { fts(v) } else { String::new() });
        }
        None => {
            ui.set_caption(F, "KeyLbl", "");
            ui.set_text(F, "KeyValEdit", "");
        }
    }
}

/// The picture's size and the place of frame `f` / value `v` in it.
fn geometry(app: &Mb3d, ui: &Ui) -> (i32, i32, impl Fn(f64) -> f64, impl Fn(f64) -> f64, impl Fn(f64) -> f64) {
    let c = ui.c(F, "Image1");
    let (w, h) = (c.width.max(40), c.height.max(40));
    let n = app.curves.values.len().max(2) as f64;
    let (lo, hi) = app.curves.range;
    let m = 8.0;
    let fx = move |f: f64| m + f / (n - 1.0) * (w as f64 - 2.0 * m);
    let vy = move |v: f64| m + (hi - v) / (hi - lo).max(1e-300) * (h as f64 - 2.0 * m);
    let yv = move |y: f64| hi - (y - m) / (h as f64 - 2.0 * m) * (hi - lo);
    (w, h, fx, vy, yv)
}

fn draw(app: &Mb3d, ui: &mut Ui) {
    let (w, h, fx, vy, _) = geometry(app, ui);
    let mut b = Bitmap::new(w as usize, h as usize, 0xFF20_2226);
    let st = &app.curves;
    let put = |b: &mut Bitmap, x: i32, y: i32, c: u32| {
        if (0..w).contains(&x) && (0..h).contains(&y) {
            b.px[y as usize * w as usize + x as usize] = c;
        }
    };
    // the keyframes' lines
    for &(f, _) in &st.keys {
        let x = fx(f as f64).round() as i32;
        for y in (0..h).step_by(3) {
            put(&mut b, x, y, 0xFF50_5560);
        }
    }
    // the zero line, when in range
    if (st.range.0..=st.range.1).contains(&0.0) {
        let y = vy(0.0).round() as i32;
        for x in (0..w).step_by(2) {
            put(&mut b, x, y, 0xFF44_4850);
        }
    }
    // the curve
    let mut last: Option<(f64, f64)> = None;
    for (f, v) in st.values.iter().enumerate() {
        if !v.is_finite() {
            last = None;
            continue;
        }
        let p = (fx(f as f64), vy(*v));
        if let Some(q) = last {
            let steps = ((p.0 - q.0).abs().max((p.1 - q.1).abs()).ceil() as i32).max(1);
            for i in 0..=steps {
                let t = i as f64 / steps as f64;
                let (x, y) = (q.0 + (p.0 - q.0) * t, q.1 + (p.1 - q.1) * t);
                put(&mut b, x.round() as i32, y.round() as i32, 0xFF39_C5BB);
                put(&mut b, x.round() as i32, y.round() as i32 + 1, 0xFF2A_9890);
            }
        } else {
            put(&mut b, p.0.round() as i32, p.1.round() as i32, 0xFF39_C5BB);
        }
        last = Some(p);
    }
    // the keyframes' squares, the chosen one filled
    for (k, &(f, v)) in st.keys.iter().enumerate() {
        if !v.is_finite() {
            continue;
        }
        let (x, y) = (fx(f as f64).round() as i32, vy(v).round() as i32);
        let chosen = st.sel == Some(k);
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                let edge = dx.abs() == 4 || dy.abs() == 4;
                if edge || chosen {
                    put(&mut b, x + dx, y + dy, if chosen { 0xFFE8_559A } else { 0xFFF0_F0F0 });
                }
            }
        }
    }
    ui.set_picture(F, "Image1", Some(b));
}

/// The keyframe whose square is at (x, y), if any.
fn key_at(app: &Mb3d, ui: &Ui, x: i32, y: i32) -> Option<usize> {
    let (_, _, fx, vy, _) = geometry(app, ui);
    app.curves
        .keys
        .iter()
        .enumerate()
        .filter(|(_, (_, v))| v.is_finite())
        .map(|(k, &(f, v))| (k, (fx(f as f64) - x as f64).abs().max((vy(v) - y as f64).abs())))
        .filter(|(_, d)| *d <= 7.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(k, _)| k)
}

/// Sets the chosen value of keyframe `k` to `v`.
fn set_key(app: &mut Mb3d, ui: &mut Ui, k: usize, v: f64) {
    let p = &PARAMS[app.curves.param.min(PARAMS.len() - 1)];
    curves::set_key(&mut app.anim.anim, p, k, v);
    update(app, ui);
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => update(app, ui),
        "ParamCBChange" => {
            app.curves.param = ui.item_index(F, "ParamCB").clamp(0, PARAMS.len() as i32 - 1) as usize;
            update(app, ui);
        }
        "Image1MouseDown" => {
            if let Ev::MouseDown { x, y, button: MouseButton::Left, .. } = e.ev {
                app.curves.sel = key_at(app, ui, x, y);
                app.curves.drag = app.curves.sel.is_some();
                show_key(app, ui);
                draw(app, ui);
            }
        }
        "Image1MouseMove" => {
            if let (Ev::MouseMove { y, .. }, true, Some(k)) = (&e.ev, app.curves.drag, app.curves.sel) {
                let (.., yv) = geometry(app, ui);
                let v = yv(*y as f64);
                // (the range stays while dragging, so the square follows the mouse)
                let range = app.curves.range;
                set_key(app, ui, k, v);
                let (lo, hi) = app.curves.range;
                app.curves.range = (range.0.min(lo), range.1.max(hi));
                draw(app, ui);
            }
        }
        "Image1MouseUp" => {
            if app.curves.drag {
                app.curves.drag = false;
                if let Some(k) = app.curves.sel {
                    super::anim_form::keyframe_changed(app, ui, k);
                }
                update(app, ui);
            }
        }
        "SetBtnClick" => {
            if let Some(k) = app.curves.sel {
                let t = ui.text(F, "KeyValEdit");
                if !t.trim().is_empty() {
                    set_key(app, ui, k, ptf(&t));
                    super::anim_form::keyframe_changed(app, ui, k);
                }
            }
        }
        _ => {}
    }
}
