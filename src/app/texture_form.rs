//! The Texture export window (`TextureFrm`, this program's own, the
//! Internal tab): the maps of the calculated picture (colour, height,
//! normal, ambient occlusion, `crate::texture`) as PNG files in a folder,
//! optionally mirrored to a tileable set, and the camera preset of the
//! group's texture recipe.

use crate::sync::Locked;
use super::{Mb3d, Ui};
use crate::vcl::lang::{tr, trf};
use crate::vcl::{DialogResult, Event};
use std::path::PathBuf;

const F: &str = "TextureFrm";

#[derive(Default)]
pub struct State {
    /// the folder written last
    pub made: Option<PathBuf>,
}

pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    ui.show(F);
    let _ = app;
}

fn default_folder(app: &Mb3d) -> PathBuf {
    match app.ini.get_extra("TextureFolder") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => app.ini.dir(super::ini::DIR_IMG).join("Textures"),
    }
}

fn title(app: &Mb3d) -> String {
    let t = std::path::Path::new(&app.title).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    if t.is_empty() {
        "texture".into()
    } else {
        crate::share::file_name_of(&t)
    }
}

fn write(app: &mut Mb3d, ui: &mut Ui) {
    let Some(b) = app.eng.base() else {
        ui.set_caption(F, "StatusLabel", &tr("@MB:CalculateAPictureFirst"));
        return;
    };
    let rgb = app.eng.0.out.locked().rgb.clone();
    let Some(rgb) = rgb.filter(|r| r.len() == b.w * b.h * 3) else {
        ui.set_caption(F, "StatusLabel", &tr("@MB:CalculateAPictureFirst"));
        return;
    };
    let maps = crate::texture::Maps {
        color: ui.checked(F, "ColorCheck"),
        height: ui.checked(F, "HeightCheck"),
        normal: ui.checked(F, "NormalCheck"),
        ao: ui.checked(F, "AoCheck"),
        tileable: ui.checked(F, "TileCheck"),
    };
    let folder = PathBuf::from(ui.text(F, "FolderEdit").trim());
    let name = ui.text(F, "NameEdit");
    match crate::texture::write(&b.post, &rgb, b.w, b.h, &folder, &name, maps) {
        Ok(files) => {
            let names: Vec<String> = files.iter().map(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).collect();
            ui.set_caption(F, "StatusLabel", &trf("@MB:Written0", &[&names.join(", ")]));
            app.ini.set_extra("TextureFolder", &folder.display().to_string());
            app.texture.made = Some(folder);
            ui.set_enabled(F, "OpenBtn", true);
        }
        Err(e) => ui.set_caption(F, "StatusLabel", &e),
    }
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => {
            ui.set_text(F, "NameEdit", &title(app));
            ui.set_text(F, "FolderEdit", &default_folder(app).display().to_string());
            ui.set_caption(F, "StatusLabel", "");
            ui.set_enabled(F, "OpenBtn", app.texture.made.is_some());
        }
        "CameraBtnClick" => {
            app.make_scene(ui);
            crate::texture::camera_preset(&mut app.scene);
            app.scene_to_forms(ui);
            ui.set_caption(F, "StatusLabel", &tr("@MB:APlanarOpticWithAField"));
        }
        "FolderBtnClick" => {
            let d = PathBuf::from(ui.text(F, "FolderEdit"));
            ui.folder_dialog("texture:folder", &tr("@MB:Folder2"), Some(if d.is_dir() { d } else { app.ini.dir(super::ini::DIR_IMG) }));
        }
        "WriteBtnClick" => write(app, ui),
        "OpenBtnClick" => {
            if let Some(d) = &app.texture.made {
                let _ = open::that_detached(d);
            }
        }
        "CloseBtnClick" => ui.hide(F),
        _ => {}
    }
}

pub fn dialog(_app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    if let ("folder", DialogResult::File(Some(p))) = (what, r) {
        ui.set_text(F, "FolderEdit", &p.display().to_string());
    }
}
