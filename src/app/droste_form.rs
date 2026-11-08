//! The Droste effect window (`DrosteFrm`, this program's own, opened from
//! the Post processing window's Finishing section): the picture repeated
//! in itself (`crate::droste`, `Scene::droste`), with a live preview of the
//! calculated picture at a reduced size, the centre set by a click or a
//! drag on it, the radii by the mouse wheel; *Paint now* paints the effect
//! at full size (a repaint of the finishing).

use super::util::{fts_single, ptf};
use super::{Mb3d, Ui};
use crate::droste::Droste;
use crate::vcl::bitmap::Bitmap;
use crate::vcl::lang::tr;
use crate::vcl::{Ev, Event, MouseButton};

const F: &str = "DrosteFrm";
/// the preview's size (the picture control)
const PREVIEW: (usize, usize) = (500, 375);

#[derive(Default)]
pub struct State {
    /// the calculated picture (before the effect) reduced to the preview:
    /// RGB in floating point, its width and height
    source: Option<(Vec<f32>, usize, usize)>,
    /// the engine's picture version the source came from
    ver: u64,
    /// the window writes the scene's values into its controls: their
    /// change events must not write them back meanwhile
    filling: bool,
    /// the window is shown
    open: bool,
    /// the left button is down on the preview (dragging the centre)
    dragging: bool,
}

pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    ui.show(F);
    show_values(app, ui);
}

/// The scene's settings into the window (also when parameters are loaded).
pub fn show_values(app: &mut Mb3d, ui: &mut Ui) {
    if !ui.has_form(F) {
        return;
    }
    app.droste.filling = true;
    let d = app.scene.droste.unwrap_or_default();
    ui.set_checked(F, "OnCheck", app.scene.droste.is_some());
    for (e, v) in [
        ("CxEdit", d.center[0]),
        ("CyEdit", d.center[1]),
        ("InnerEdit", d.inner),
        ("OuterEdit", d.outer),
        ("TwistEdit", d.twist),
        ("BlendEdit", d.blend),
        ("PhaseEdit", d.phase),
    ] {
        ui.set_text(F, e, &fts_single(v as f64));
    }
    ui.set_position(F, "PhaseBar", (d.phase.rem_euclid(1.0) * 100.0).round() as i64);
    enable(ui);
    app.droste.filling = false;
    if app.droste.open {
        refresh(app, ui);
    }
}

/// The window's settings into the scene.
fn read_values(app: &mut Mb3d, ui: &Ui) {
    if !ui.has_form(F) || app.droste.filling {
        return;
    }
    if ui.checked(F, "OnCheck") {
        let num = |e: &str| ptf(&ui.text(F, e)) as f32;
        let base = app.scene.droste.unwrap_or_default();
        let or = |v: f32, b: f32| if v.is_finite() { v } else { b };
        app.scene.droste = Some(
            Droste {
                center: [or(num("CxEdit"), base.center[0]), or(num("CyEdit"), base.center[1])],
                inner: or(num("InnerEdit"), base.inner),
                outer: or(num("OuterEdit"), base.outer),
                twist: or(num("TwistEdit"), base.twist),
                blend: or(num("BlendEdit"), base.blend),
                phase: or(num("PhaseEdit"), base.phase),
            }
            .clamped(),
        );
    } else {
        app.scene.droste = None;
    }
}

fn enable(ui: &mut Ui) {
    let on = ui.checked(F, "OnCheck");
    for c in [
        "CxLabel", "CxEdit", "CyLabel", "CyEdit", "InnerLabel", "InnerEdit", "OuterLabel", "OuterEdit", "TwistLabel", "TwistEdit", "BlendLabel", "BlendEdit", "PhaseLabel", "PhaseEdit",
        "PhaseBar", "ResetBtn",
    ] {
        ui.set_enabled(F, c, on);
    }
}

/// The calculated picture reduced to the preview's size (a box filter).
fn reduce(rgb: &[u8], w: usize, h: usize) -> Option<(Vec<f32>, usize, usize)> {
    if w == 0 || h == 0 || rgb.len() < w * h * 3 {
        return None;
    }
    let scale = (PREVIEW.0 as f64 / w as f64).min(PREVIEW.1 as f64 / h as f64).min(1.0);
    let (pw, ph) = (((w as f64 * scale).round() as usize).max(1), ((h as f64 * scale).round() as usize).max(1));
    let mut out = vec![0f32; pw * ph * 3];
    for py in 0..ph {
        let y0 = py * h / ph;
        let y1 = ((py + 1) * h / ph).max(y0 + 1).min(h);
        for px in 0..pw {
            let x0 = px * w / pw;
            let x1 = ((px + 1) * w / pw).max(x0 + 1).min(w);
            let mut acc = [0f32; 3];
            for y in y0..y1 {
                for x in x0..x1 {
                    let o = (y * w + x) * 3;
                    for k in 0..3 {
                        acc[k] += rgb[o + k] as f32;
                    }
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as f32 * 255.0;
            let o = (py * pw + px) * 3;
            for k in 0..3 {
                out[o + k] = acc[k] / n;
            }
        }
    }
    Some((out, pw, ph))
}

/// Takes the engine's current picture as the preview's source when it
/// changed; true when there is one.
fn take_source(app: &mut Mb3d) -> bool {
    let (ver, pic) = {
        let o = app.eng.0.out.lock().unwrap();
        (o.ver, o.plain.clone().or_else(|| o.rgb.clone()).map(|p| (p, o.w, o.h)))
    };
    if ver != app.droste.ver || app.droste.source.is_none() {
        app.droste.ver = ver;
        app.droste.source = pic.and_then(|(p, w, h)| reduce(&p, w, h));
    }
    app.droste.source.is_some()
}

/// Draws a circle of radius `r` around (cx, cy) on the bitmap, light with
/// a dark outline, so that it shows on any picture.
fn circle(bm: &mut Bitmap, cx: f64, cy: f64, r: f64) {
    let steps = ((r * 8.0) as usize).clamp(64, 4096);
    for i in 0..steps {
        let a = i as f64 / steps as f64 * std::f64::consts::TAU;
        let (s, c) = a.sin_cos();
        for (rr, col) in [(r + 1.0, 0xFF20_2020u32), (r, 0xFFF0_F0F0)] {
            let (x, y) = ((cx + rr * c).round() as i64, (cy + rr * s).round() as i64);
            if x >= 0 && y >= 0 && (x as usize) < bm.w && (y as usize) < bm.h {
                bm.px[y as usize * bm.w + x as usize] = col;
            }
        }
    }
}

/// Paints the preview: the reduced picture with the effect, the centre
/// and the two radii drawn over it.
pub fn refresh(app: &mut Mb3d, ui: &mut Ui) {
    if !ui.has_form(F) {
        return;
    }
    if !take_source(app) {
        ui.set_picture(F, "Image1", None);
        ui.set_caption(F, "StatusLabel", &tr("@MB:CalculateAPictureFirst"));
        return;
    }
    let Some((src, pw, ph)) = app.droste.source.as_ref() else { return };
    let (pw, ph) = (*pw, *ph);
    let mut v = src.clone();
    let d = app.scene.droste;
    if let Some(d) = d {
        crate::droste::apply(&mut v, pw, ph, &d, 4);
    }
    let mut bm = Bitmap::new(pw, ph, 0xFF00_0000);
    for (p, c) in bm.px.iter_mut().zip(v.chunks_exact(3)) {
        let q = |x: f32| (x * 255.0).round().clamp(0.0, 255.0) as u32;
        *p = 0xFF00_0000 | (q(c[0]) << 16) | (q(c[1]) << 8) | q(c[2]);
    }
    if let Some(d) = d {
        let d = d.clamped();
        let (cx, cy) = (d.center[0] as f64 * pw as f64, d.center[1] as f64 * ph as f64);
        let size = pw.min(ph) as f64;
        circle(&mut bm, cx, cy, d.inner as f64 * size);
        circle(&mut bm, cx, cy, d.outer as f64 * size);
        circle(&mut bm, cx, cy, 2.0);
    }
    ui.set_picture(F, "Image1", Some(bm));
    ui.set_caption(F, "StatusLabel", &tr(if d.is_some() { "@MB:ThePreviewIsTheCalculatedPicture" } else { "@MB:TickRepeatThePictureIn" }));
}

/// The idle work: a new picture calculated while the window is open
/// refreshes the preview.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if !app.droste.open || !ui.has_form(F) {
        return;
    }
    let ver = app.eng.0.out.lock().unwrap().ver;
    if ver != app.droste.ver {
        refresh(app, ui);
    }
}

/// The picture point (as parts of the preview's width and height) under
/// the control point (x, y): the picture is centred in the control.
fn picture_point(app: &Mb3d, ui: &Ui, x: i32, y: i32) -> Option<[f32; 2]> {
    let (_, pw, ph) = app.droste.source.as_ref()?;
    let c = ui.c(F, "Image1");
    let (ox, oy) = ((c.width - *pw as i32) / 2, (c.height - *ph as i32) / 2);
    Some([(x - ox) as f32 / *pw as f32, (y - oy) as f32 / *ph as f32])
}

/// The settings changed by the mouse: into the controls and the scene,
/// and the preview painted again.
fn set(app: &mut Mb3d, ui: &mut Ui, d: Droste) {
    app.scene.droste = Some(d.clamped());
    show_values(app, ui);
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => {
            app.droste.open = true;
            app.droste.source = None;
            show_values(app, ui);
            refresh(app, ui);
        }
        "FormHide" => {
            app.droste.open = false;
            app.droste.dragging = false;
        }
        "OnCheckClick" => {
            if app.droste.filling {
                return;
            }
            enable(ui);
            read_values(app, ui);
            refresh(app, ui);
        }
        "EditChange" => {
            if app.droste.filling {
                return;
            }
            read_values(app, ui);
            if let Some(d) = app.scene.droste {
                app.droste.filling = true;
                ui.set_position(F, "PhaseBar", (d.phase.rem_euclid(1.0) * 100.0).round() as i64);
                app.droste.filling = false;
            }
            refresh(app, ui);
        }
        "PhaseBarChange" => {
            if app.droste.filling {
                return;
            }
            let p = ui.position(F, "PhaseBar") as f32 / 100.0;
            app.droste.filling = true;
            ui.set_text(F, "PhaseEdit", &fts_single(p as f64));
            app.droste.filling = false;
            read_values(app, ui);
            refresh(app, ui);
        }
        "Image1MouseDown" => {
            if let Ev::MouseDown { x, y, button: MouseButton::Left, .. } = &e.ev {
                if let (Some(d), Some(p)) = (app.scene.droste, picture_point(app, ui, *x, *y)) {
                    app.droste.dragging = true;
                    set(app, ui, Droste { center: p, ..d });
                }
            }
        }
        "Image1MouseMove" => {
            if let Ev::MouseMove { x, y, .. } = &e.ev {
                if app.droste.dragging {
                    if let (Some(d), Some(p)) = (app.scene.droste, picture_point(app, ui, *x, *y)) {
                        set(app, ui, Droste { center: p, ..d });
                    }
                }
            }
        }
        "Image1MouseUp" => app.droste.dragging = false,
        "Image1MouseWheel" => {
            if let Ev::Wheel { delta, shift, .. } = &e.ev {
                if let Some(d) = app.scene.droste {
                    let f = 1.08f32.powf(*delta as f32 / 120.0);
                    // Shift: the inner radius; else the outer one
                    let d = if shift & 1 != 0 { Droste { inner: d.inner * f, ..d } } else { Droste { outer: d.outer * f, ..d } };
                    set(app, ui, d);
                }
            }
        }
        "PaintBtnClick" => {
            read_values(app, ui);
            super::postpro_form::repaint(app, ui, &tr("@MB:Finishing"));
        }
        "ResetBtnClick" => set(app, ui, Droste::default()),
        "CloseBtnClick" => ui.hide(F),
        _ => {}
    }
}
