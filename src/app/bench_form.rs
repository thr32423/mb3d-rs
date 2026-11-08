//! The Benchmark window (`BenchFrm`, this program's own, the Internal
//! tab): runs the fixed benchmark scene (`crate::bench`) on the processor
//! and the graphics card in a thread of its own and shows the result as
//! text to copy and post.

use super::{Mb3d, Ui};
use crate::vcl::lang::{tr, trf};
use crate::vcl::Event;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const F: &str = "BenchFrm";

#[derive(Default)]
pub struct State {
    result: Arc<Mutex<Option<Result<crate::bench::BenchResult, String>>>>,
    /// the stage running, for the status line
    stage: Arc<Mutex<String>>,
    cancel: Arc<AtomicBool>,
    running: bool,
    text: String,
}

pub fn open(_app: &mut Mb3d, ui: &mut Ui) {
    ui.show(F);
}

fn run(app: &mut Mb3d, ui: &mut Ui) {
    if app.bench.running || app.main.calculating {
        if app.main.calculating {
            ui.set_caption(F, "StatusLabel", &tr("@MB:ACalculationIsRunningStopIt"));
        }
        return;
    }
    app.bench.running = true;
    app.bench.cancel.store(false, Ordering::Relaxed);
    *app.bench.result.lock().unwrap() = None;
    ui.set_enabled(F, "RunBtn", false);
    ui.set_enabled(F, "CopyBtn", false);
    ui.set_text(F, "ResultMemo", "");
    let (slot, stage, cancel) = (app.bench.result.clone(), app.bench.stage.clone(), app.bench.cancel.clone());
    std::thread::spawn(move || {
        let st = stage.clone();
        let r = std::panic::catch_unwind(|| crate::bench::run(true, &|s| *st.lock().unwrap() = s.to_string(), &|| cancel.load(Ordering::Relaxed)))
            .unwrap_or_else(|_| Err(tr("@MB:TheCalculationFailedLower")));
        *slot.lock().unwrap() = Some(r);
    });
    ui.set_timer(F, "Timer1", true);
}

/// The idle work: the result, when it is there.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if app.bench.running && ui.has_form(F) {
        poll(app, ui);
    }
}

fn poll(app: &mut Mb3d, ui: &mut Ui) {
    let done = app.bench.result.lock().unwrap().take();
    match done {
        None => {
            let s = app.bench.stage.lock().unwrap().clone();
            ui.set_caption(F, "StatusLabel", &trf("@MB:Running0", &[&s]));
        }
        Some(r) => {
            ui.set_timer(F, "Timer1", false);
            app.bench.running = false;
            ui.set_enabled(F, "RunBtn", true);
            ui.set_caption(F, "StatusLabel", "");
            match r {
                Ok(res) => {
                    app.bench.text = res.text();
                    ui.set_text(F, "ResultMemo", &app.bench.text);
                    ui.set_enabled(F, "CopyBtn", true);
                    crate::log::info("benchmark", &app.bench.text);
                }
                Err(e) => ui.set_text(F, "ResultMemo", &e),
            }
        }
    }
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "RunBtnClick" => run(app, ui),
        "CopyBtnClick" => {
            crate::vcl::clipboard::set(&app.bench.text);
            ui.set_caption(F, "StatusLabel", &tr("@MB:CopiedToTheClipboard"));
        }
        "Timer1Timer" => poll(app, ui),
        "CloseBtnClick" => {
            app.bench.cancel.store(true, Ordering::Relaxed);
            ui.hide(F);
        }
        _ => {}
    }
}
