//! The Formula book window (`FormulaBookFrm`, this program's own): every
//! formula the Formulas window lists, with a picture, its kind in words,
//! the author's description, its values explained by their names, hints
//! and the example files that use it ([`crate::formula_book`]).  The
//! Mandelbulb Maniacs group asks for plain-language documentation of each
//! formula and parameter; this is what the program can say from what it
//! has.  Opened from the Formulas window's book button and the right-click
//! menu of a formula name.

use super::{Mb3d, Ui};
use crate::formula_book::{self, Category, Topic, CATEGORIES};
use crate::vcl::lang::{tr, trf};
use crate::vcl::Event;
use std::collections::HashMap;
use std::path::PathBuf;

const F: &str = "FormulaBookFrm";

#[derive(Default)]
pub struct State {
    topics: Vec<Topic>,
    /// the formulas shown in the list (indices into `topics`)
    shown: Vec<usize>,
    /// the example files by lower-case formula name
    examples: HashMap<String, Vec<PathBuf>>,
    /// the folders the examples were read from, with their change times
    examples_of: Vec<(PathBuf, Option<std::time::SystemTime>)>,
    /// the formula to select when the window opens
    want: Option<String>,
}

/// Opens the book, at `name` when given.
pub fn open(app: &mut Mb3d, ui: &mut Ui, name: Option<&str>) {
    app.book.want = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    if ui.showing(F) {
        reload(app, ui);
    } else {
        ui.show(F);
    }
}

/// The folders whose files count as examples: the parameter folder and
/// the folder of the parameters opened last.
fn example_folders(app: &Mb3d) -> Vec<PathBuf> {
    let mut v = vec![app.ini.dir(super::ini::DIR_M3P)];
    // the shipped examples (next to the program, or where download.sh
    // installed the release)
    v.extend(crate::appdirs::resource("M3Parameter"));
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        v.push(PathBuf::from(home).join("mb3d-rs").join("M3Parameter"));
    }
    if let Some(d) = std::path::Path::new(&app.title).parent().filter(|d| d.is_dir() && !d.as_os_str().is_empty()) {
        v.push(d.to_path_buf());
    }
    v.retain(|d| d.is_dir());
    v.dedup();
    v
}

fn modified(p: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
}

/// Reads the formulas and the example files again when something changed.
fn reload(app: &mut Mb3d, ui: &mut Ui) {
    let folders = example_folders(app);
    let stamps: Vec<(PathBuf, Option<std::time::SystemTime>)> = folders.iter().map(|d| (d.clone(), modified(d))).collect();
    if app.book.examples_of != stamps {
        ui.set_caption(F, "CountLabel", &tr("@MB:ReadingTheExampleFiles"));
        app.book.examples = formula_book::example_index(&folders);
        app.book.examples_of = stamps;
    }
    app.book.topics = formula_book::topics();
    if ui.c(F, "CatCombo").items.is_empty() {
        let mut items = vec![tr("@MB:AllFormulas")];
        items.extend(CATEGORIES.iter().map(|c| tr(c.label())));
        ui.set_items(F, "CatCombo", items);
        ui.set_item_index(F, "CatCombo", 0);
    }
    fill_list(app, ui);
}

/// The category chosen in the filter list, None for all.
fn chosen_category(ui: &Ui) -> Option<Category> {
    let i = ui.item_index(F, "CatCombo");
    (i >= 1).then(|| CATEGORIES.get(i as usize - 1).copied()).flatten()
}

/// The names list from the search text and the category; the wanted or
/// the previously selected formula stays selected.
fn fill_list(app: &mut Mb3d, ui: &mut Ui) {
    let search = ui.text(F, "SearchEdit").trim().to_lowercase();
    let cat = chosen_category(ui);
    let prev = app.book.want.take().or_else(|| selected(app, ui).map(|t| t.name.clone()));
    app.book.shown = app
        .book
        .topics
        .iter()
        .enumerate()
        .filter(|(_, t)| cat.is_none_or(|c| t.category == c))
        .filter(|(_, t)| search.is_empty() || t.name.to_lowercase().contains(&search))
        .map(|(i, _)| i)
        .collect();
    let items: Vec<String> = app.book.shown.iter().map(|&i| app.book.topics[i].name.clone()).collect();
    ui.set_items(F, "NameList", items);
    let sel = prev.and_then(|n| app.book.shown.iter().position(|&i| app.book.topics[i].name.eq_ignore_ascii_case(&n)));
    let sel = sel.map_or(if app.book.shown.is_empty() { -1 } else { 0 }, |i| i as i32);
    ui.set_item_index(F, "NameList", sel);
    // (the selection in sight, a few rows from the top)
    ui.cm(F, "NameList").top_index = (sel - 3).max(0);
    ui.set_caption(F, "CountLabel", &trf("@MB:N0Of1Formulas", &[&app.book.shown.len().to_string(), &app.book.topics.len().to_string()]));
    show_selected(app, ui);
}

fn selected<'a>(app: &'a Mb3d, ui: &Ui) -> Option<&'a Topic> {
    let i = ui.item_index(F, "NameList");
    (i >= 0).then(|| app.book.shown.get(i as usize)).flatten().map(|&k| &app.book.topics[k])
}

fn examples_of<'a>(app: &'a Mb3d, t: &Topic) -> &'a [PathBuf] {
    app.book.examples.get(&t.name.to_ascii_lowercase()).map_or(&[], |v| v.as_slice())
}

/// The page of the selected formula: name, kind, examples, text, picture.
fn show_selected(app: &mut Mb3d, ui: &mut Ui) {
    let Some(t) = selected(app, ui).cloned() else {
        for c in ["NameLabel", "KindLabel", "PicLabel"] {
            ui.set_caption(F, c, "");
        }
        ui.set_text(F, "PageMemo", "");
        ui.set_items(F, "ExampleList", Vec::new());
        ui.set_picture(F, "Image1", None);
        ui.set_enabled(F, "UseBtn", false);
        ui.set_enabled(F, "OpenBtn", false);
        ui.set_enabled(F, "CopyBtn", false);
        return;
    };
    let ex: Vec<PathBuf> = examples_of(app, &t).to_vec();
    ui.set_caption(F, "NameLabel", &t.name);
    ui.set_caption(F, "KindLabel", &tr(t.category.label()));
    ui.set_text(F, "PageMemo", &formula_book::page_text(&t, &ex));
    ui.set_items(F, "ExampleList", ex.iter().map(|p| formula_book::file_title(p)).collect());
    if !ex.is_empty() {
        ui.set_item_index(F, "ExampleList", 0);
    }
    ui.set_enabled(F, "UseBtn", true);
    ui.set_enabled(F, "OpenBtn", !ex.is_empty());
    ui.set_enabled(F, "CopyBtn", true);
    // the picture: the first example, else the formula alone (its scene
    // written to the cache folder, calculated by the preview queue like
    // any file)
    match ex.first().cloned().map(Ok).unwrap_or_else(|| formula_book::starter_file(&t.name)) {
        Ok(p) => {
            super::file_preview::request(app, ui, p.clone(), F, "Image1", "PicLabel");
            // (the queue's own caption, size and formulas, is replaced
            // once the picture is there: idle_done)
            let _ = p;
        }
        Err(e) => {
            ui.set_picture(F, "Image1", None);
            ui.set_caption(F, "PicLabel", &e);
        }
    }
}

/// The example clicked in the list: its picture.
fn show_example(app: &mut Mb3d, ui: &mut Ui) {
    if let Some(p) = chosen_example(app, ui) {
        super::file_preview::request(app, ui, p, F, "Image1", "PicLabel");
    }
}

/// The caption under the picture, in the book's words rather than the
/// preview queue's (called from the idle work when a preview arrived).
pub fn picture_caption(app: &Mb3d, ui: &mut Ui) {
    if !ui.showing(F) {
        return;
    }
    let Some(t) = selected(app, ui) else { return };
    let ex = examples_of(app, t);
    let i = ui.item_index(F, "ExampleList").max(0) as usize;
    let text = match ex.get(i).or(ex.first()) {
        Some(p) => trf("@MB:Example0", &[&formula_book::file_title(p)]),
        None => tr("@MB:TheFormulaAloneAtItsDefault"),
    };
    if ui.c(F, "Image1").picture.is_some() {
        ui.set_caption(F, "PicLabel", &text);
    }
}

/// The example chosen in the list, else the first.
fn chosen_example(app: &Mb3d, ui: &Ui) -> Option<PathBuf> {
    let t = selected(app, ui)?;
    let ex = examples_of(app, t);
    let i = ui.item_index(F, "ExampleList").max(0) as usize;
    ex.get(i).or(ex.first()).cloned()
}

fn use_formula(app: &mut Mb3d, ui: &mut Ui) {
    let Some(name) = selected(app, ui).map(|t| t.name.clone()) else { return };
    if !ui.showing("FormulaGUIForm") {
        ui.show("FormulaGUIForm");
    }
    super::formula_form::select_formula(app, ui, &name);
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => reload(app, ui),
        "FilterChange" => fill_list(app, ui),
        "NameListClick" => show_selected(app, ui),
        "NameListDblClick" => use_formula(app, ui),
        "ExampleListClick" => show_example(app, ui),
        "UseBtnClick" => use_formula(app, ui),
        "OpenBtnClick" | "ExampleListDblClick" => {
            if let Some(p) = chosen_example(app, ui) {
                super::main_form::open_file(app, ui, &p);
            }
        }
        "CopyBtnClick" => {
            if let Some(t) = selected(app, ui) {
                crate::vcl::clipboard::set(&t.name);
            }
        }
        "CloseBtnClick" => ui.hide(F),
        _ => {}
    }
}
