//! The "Steps and noise" window (`StepsFrm`, this program's own, the
//! Internal tab): the step settings of the passes after the main one
//! (`Scene::post_steps`, kept with the parameters) and the noise doctor
//! (`crate::noise`), which renders the middle of the picture under three
//! step settings and says which gives the fewest specks.

use crate::sync::Locked;
use super::util::{fts_single, ptf};
use super::{Mb3d, Ui};
use crate::noise::Trial;
use crate::scene::PostSteps;
use crate::vcl::lang::tr;
use crate::vcl::Event;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const F: &str = "StepsFrm";

#[derive(Default)]
pub struct State {
    /// the doctor's run: its result when done
    result: Arc<Mutex<Option<Result<Vec<Trial>, String>>>>,
    cancel: Arc<AtomicBool>,
    running: bool,
    /// the trials of the last run, for "apply"
    trials: Vec<Trial>,
    /// (the window writes the scene's values into its controls: their
    /// change events must not write them back meanwhile)
    filling: bool,
}

pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    show_values(app, ui);
    ui.show(F);
}

/// The scene's post step settings into the window.
pub fn show_values(app: &mut Mb3d, ui: &mut Ui) {
    if !ui.has_form(F) {
        return;
    }
    app.steps.filling = true;
    let ps = app.scene.post_steps.unwrap_or_else(|| PostSteps::of(&app.scene));
    ui.set_checked(F, "PostCheck", app.scene.post_steps.is_some());
    ui.set_text(F, "RayEdit", &fts_single(ps.raystep));
    ui.set_text(F, "DeEdit", &fts_single(ps.de_stop));
    enable(ui);
    app.steps.filling = false;
}

/// The window's post step settings into the scene.
pub fn read_values(app: &mut Mb3d, ui: &Ui) {
    if !ui.has_form(F) || app.steps.filling {
        return;
    }
    if ui.checked(F, "PostCheck") {
        let r = ptf(&ui.text(F, "RayEdit"));
        let d = ptf(&ui.text(F, "DeEdit"));
        let base = PostSteps::of(&app.scene);
        app.scene.post_steps = Some(PostSteps {
            raystep: if r.is_finite() && r > 0.0 { r.clamp(0.0001, 100.0) } else { base.raystep },
            de_stop: if d.is_finite() && d > 0.0 { d.clamp(0.001, 1000.0) } else { base.de_stop },
        });
    } else {
        app.scene.post_steps = None;
    }
}

fn enable(ui: &mut Ui) {
    let on = ui.checked(F, "PostCheck");
    for c in ["RayLabel", "RayEdit", "DeLabel", "DeEdit"] {
        ui.set_enabled(F, c, on);
    }
}

/// Starts the doctor on the scene as the windows hold it.
fn run(app: &mut Mb3d, ui: &mut Ui) {
    if app.steps.running {
        return;
    }
    app.make_scene(ui);
    let sc = app.scene.clone();
    if !sc.formulas.iter().any(|f| f.iterations > 0) {
        ui.set_text(F, "ResultMemo", &tr("@MB:FormulaNotFound").replace("{0}", "-"));
        return;
    }
    app.steps.running = true;
    app.steps.trials.clear();
    app.steps.cancel.store(false, Ordering::Relaxed);
    *app.steps.result.locked() = None;
    ui.set_text(F, "ResultMemo", &tr("@MB:RunningTheNoiseDoctor"));
    ui.set_enabled(F, "RunBtn", false);
    ui.set_enabled(F, "ApplyBtn", false);
    let slot = app.steps.result.clone();
    let cancel = app.steps.cancel.clone();
    std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::noise::diagnose(&sc, &|| cancel.load(Ordering::Relaxed))))
            .unwrap_or_else(|_| Err(tr("@MB:TheCalculationFailedLower")));
        *slot.locked() = Some(r);
    });
    ui.set_timer(F, "Timer1", true);
}

/// The idle work: the doctor's result, when it is there.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if app.steps.running && ui.has_form(F) {
        poll(app, ui);
    }
}

/// The doctor's result, when it is there.
fn poll(app: &mut Mb3d, ui: &mut Ui) {
    let done = app.steps.result.locked().take();
    let Some(r) = done else { return };
    ui.set_timer(F, "Timer1", false);
    app.steps.running = false;
    ui.set_enabled(F, "RunBtn", true);
    match r {
        Ok(trials) => {
            ui.set_text(F, "ResultMemo", &crate::noise::report(&trials));
            let best = crate::noise::best(&trials);
            ui.set_enabled(F, "ApplyBtn", best != 0);
            app.steps.trials = trials;
        }
        Err(e) => ui.set_text(F, "ResultMemo", &e),
    }
}

/// The recommended setting into the scene and the windows.
fn apply(app: &mut Mb3d, ui: &mut Ui) {
    let trials = std::mem::take(&mut app.steps.trials);
    let b = crate::noise::best(&trials);
    let Some(t) = trials.get(b) else { return };
    app.make_scene(ui);
    app.scene.z_step_div = t.raystep;
    app.scene.raystep_limiter = t.limiter;
    app.scene_to_forms(ui);
    ui.set_enabled(F, "ApplyBtn", false);
    app.message(ui, &format!("{}: {}", tr("@MB:NoiseDoctor"), t.what));
    app.steps.trials = trials;
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => show_values(app, ui),
        "PostCheckClick" => {
            enable(ui);
            read_values(app, ui);
        }
        "PostEditChange" => read_values(app, ui),
        "RunBtnClick" => run(app, ui),
        "ApplyBtnClick" => apply(app, ui),
        "Timer1Timer" => poll(app, ui),
        "CloseBtnClick" => {
            app.steps.cancel.store(true, Ordering::Relaxed);
            read_values(app, ui);
            ui.hide(F);
        }
        _ => {}
    }
}
