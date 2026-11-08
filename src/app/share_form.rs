//! The Share window (`ShareFrm`, this program's own, the share icon of the
//! main window's title bar): the shown picture as a JPEG of the size
//! Facebook keeps sharp, its parameters as a text file with the title, the
//! artist, the credit and the formulas above them, and the text of a post,
//! written into a folder of their own ([`crate::share`]); the post text
//! goes to the clipboard.  What the Mandelbulb Maniacs group asks of a
//! post: the parameters as a .txt file (Facebook turns pasted ones into
//! links and flags them as spam) and the credit of the parameters' artist.

use super::{Mb3d, Ui};
use crate::vcl::lang::{tr, trf};
use crate::vcl::{DialogResult, Event};
use std::path::PathBuf;

const F: &str = "ShareFrm";

#[derive(Default)]
pub struct State {
    /// the folder of the files made last
    pub made: Option<PathBuf>,
}

/// Opens the window (the icon's click).
pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    ui.show(F);
    let _ = app;
}

/// The scene of the shown picture (the parameters as calculated), else
/// the parameters of the windows.
fn shown_scene(app: &mut Mb3d, ui: &mut Ui) -> crate::scene::Scene {
    match app.eng.base() {
        Some(b) => b.scene.clone(),
        None => {
            app.make_scene(ui);
            app.scene.clone()
        }
    }
}

fn title(app: &Mb3d) -> String {
    let t = std::path::Path::new(&app.title).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    if t.is_empty() {
        "Mandelbulb 3D".into()
    } else {
        t
    }
}

/// The lines about the picture: title (with the artist), credit,
/// formulas, program.
fn about_lines(ui: &Ui, sc: &crate::scene::Scene) -> Vec<String> {
    let (t, a, c) = (ui.text(F, "TitleEdit"), ui.text(F, "ArtistEdit"), ui.text(F, "CreditEdit"));
    let mut v = Vec::new();
    match (t.trim().is_empty(), a.trim().is_empty()) {
        (false, false) => v.push(trf("@MB:By2", &[&t.trim(), &a.trim()])),
        (false, true) => v.push(t.trim().to_string()),
        (true, false) => v.push(a.trim().to_string()),
        _ => {}
    }
    if !c.trim().is_empty() {
        v.push(c.trim().to_string());
    }
    v.push(trf("@MB:Formulas2", &[&crate::share::formula_summary(sc)]));
    v.push(trf("@MB:MadeWithMb3dRsMandelbulb3DFor", &[&env!("CARGO_PKG_VERSION")]));
    v
}

fn file_stem(ui: &Ui) -> String {
    crate::share::file_name_of(&ui.text(F, "TitleEdit"))
}

/// The post's text from the fields.
fn post_text(app: &mut Mb3d, ui: &mut Ui) -> String {
    let sc = shown_scene(app, ui);
    let mut v = about_lines(ui, &sc);
    if ui.checked(F, "ParamCheck") {
        v.push(trf("@MB:ParametersAttached", &[&format!("{}.txt", file_stem(ui))]));
    }
    v.join("\n")
}

fn update_post(app: &mut Mb3d, ui: &mut Ui) {
    let t = post_text(app, ui);
    ui.set_text(F, "PostMemo", &t);
    ui.set_enabled(F, "SizeCombo", ui.checked(F, "PicCheck"));
}

fn default_folder(app: &Mb3d) -> PathBuf {
    match app.ini.get_extra("ShareFolder") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => app.ini.dir(super::ini::DIR_IMG).join("Share"),
    }
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => {
            ui.set_text(F, "TitleEdit", &title(app));
            // MB3D's authors: the original one and the one who changed the
            // parameters ("insert" on the Infos page)
            let [orig, modi] = app.main.authors.clone();
            let (orig, modi) = (orig.trim().to_string(), modi.trim().to_string());
            let artist = if modi.is_empty() { orig.clone() } else { modi.clone() };
            ui.set_text(F, "ArtistEdit", &artist);
            let credit = if !modi.is_empty() && !orig.is_empty() && !orig.eq_ignore_ascii_case(&modi) { trf("@MB:TweakOfParametersBy", &[&orig]) } else { String::new() };
            ui.set_text(F, "CreditEdit", &credit);
            ui.set_checked(F, "CamCheck", crate::vr::camera_tag());
            ui.set_text(F, "FolderEdit", &default_folder(app).display().to_string());
            ui.set_caption(F, "StatusLabel", "");
            ui.set_enabled(F, "OpenBtn", app.share.made.is_some());
            update_post(app, ui);
        }
        "FieldChange" => update_post(app, ui),
        "CamCheckClick" => {
            let on = ui.checked(F, "CamCheck");
            crate::vr::set_camera_tag(on);
            app.ini.set_extra("Pano360CameraTag", if on { "1" } else { "0" });
        }
        "FolderBtnClick" => {
            let d = PathBuf::from(ui.text(F, "FolderEdit"));
            ui.folder_dialog("share:folder", &tr("@MB:Folder2"), Some(if d.is_dir() { d } else { app.ini.dir(super::ini::DIR_IMG) }));
        }
        "CopyBtnClick" => {
            crate::vcl::clipboard::set(&ui.text(F, "PostMemo"));
            ui.set_caption(F, "StatusLabel", &tr("@MB:CopiedToTheClipboard"));
        }
        "OpenBtnClick" => {
            if let Some(d) = &app.share.made {
                let _ = open::that_detached(d);
            }
        }
        "CloseBtnClick" => ui.hide(F),
        "MakeBtnClick" => make(app, ui),
        _ => {}
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    if let ("folder", DialogResult::File(Some(p))) = (what, r) {
        ui.set_text(F, "FolderEdit", &p.display().to_string());
        app.ini.set_extra("ShareFolder", &p.display().to_string());
    }
}

/// "Make the files": the folder with the picture, the parameters and the
/// post; the post text to the clipboard.
fn make(app: &mut Mb3d, ui: &mut Ui) {
    let sc = shown_scene(app, ui);
    let pic = ui.checked(F, "PicCheck");
    let image = if pic { super::main_form::shown_picture(app) } else { None };
    if pic && image.is_none() {
        ui.set_caption(F, "StatusLabel", &tr("@MB:CalculateAPictureFirstOrUntick"));
        return;
    }
    let name = file_stem(ui);
    let params = ui.checked(F, "ParamCheck").then(|| {
        let mut t = about_lines(ui, &sc).join("\n");
        t.push_str("\n\n");
        t.push_str(&crate::m3p::raw_to_text(&crate::m3p::write(&sc), &ui.text(F, "TitleEdit")));
        t
    });
    let post = ui.text(F, "PostMemo");
    let picture = image.as_ref().map(|(rgb, w, h)| {
        let long = if ui.item_index(F, "SizeCombo") == 0 { crate::share::facebook_long_side(&sc, *w, *h) } else { 0 };
        (rgb.as_slice(), *w, *h, long)
    });
    let parent = PathBuf::from(ui.text(F, "FolderEdit").trim());
    let s = crate::share::Share { scene: &sc, name, picture, params, post: post.clone() };
    match crate::share::write(&parent, &s) {
        Ok(w) => {
            crate::vcl::clipboard::set(&post);
            let name = w.folder.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            ui.set_caption(F, "StatusLabel", &trf("@MB:SharedInThePostTextIs", &[&name]));
            app.message(ui, &trf("@MB:SharedInThePostTextIs", &[&w.folder.display()]));
            app.share.made = Some(w.folder);
            ui.set_enabled(F, "OpenBtn", true);
        }
        Err(e) => {
            ui.set_caption(F, "StatusLabel", &e);
            app.message(ui, &e);
        }
    }
}
