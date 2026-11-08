//! The screenshots of docs/COMMUNITY_WISHES.md, without windows:
//! `communityshots OUTDIR [STEP ...]`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = std::path::PathBuf::from(&args[0]);
    std::fs::create_dir_all(&out).unwrap();
    let steps: Vec<String> = args[1..].to_vec();
    let (mut ui, mut app) = mb3d::app::build(&[]).unwrap().unwrap();
    let wait = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
        for _ in 0..1200 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            mb3d::vcl::App::idle(app, ui);
            app.process(ui);
            if !app.eng.running() && !app.main.calculating {
                break;
            }
        }
    };
    wait(&mut ui, &mut app);
    for s in &steps {
        match s.as_str() {
            "colordlg" => {
                ui.recent_colors = vec![0xE0A030, 0x3060C0, 0x20A070, 0xC03040];
                ui.pick_color("", 0x4080C0);
                app.process(&mut ui);
                ui.save_form_png("ColorDlg", &out.join("color_dialog.png"), 1.0).unwrap();
            }
            "colorform" => {
                ui.show("ColorForm");
                app.process(&mut ui);
                ui.save_form_png("ColorForm", &out.join("color_form.png"), 1.0).unwrap();
            }
            // dIFS mixed with other formulas: the explanation, its fix
            "difs" => {
                mb3d::formulas::add_formula_dir(std::path::PathBuf::from("M3Formulas-rs"));
                app.scene.formulas = [("Integer Power", 8), ("Amazing Box", 2), ("SphereTree", 1)]
                    .iter()
                    .map(|(n, its)| mb3d::scene::FormulaEntry { formula: mb3d::formulas::lookup(n).unwrap(), iterations: *its })
                    .collect();
                app.scene.decomb = None;
                app.scene_to_forms(&mut ui);
                ui.click("Mand3DForm", "Button2");
                app.process(&mut ui);
                wait(&mut ui, &mut app);
                let found = ui.has_form("MessageBoxDlg") && ui.showing("MessageBoxDlg");
                eprintln!("explanation shown: {found}");
                ui.save_form_png("MessageBoxDlg", &out.join("explain_difs.png"), 1.0).unwrap();
                // "Set up DE combination"
                ui.click("MessageBoxDlg", "Btn0");
                app.process(&mut ui);
                wait(&mut ui, &mut app);
                eprintln!("decomb: {:?}", app.scene.decomb.map(|d| (d.kind, d.end1, d.start2, d.end2)));
                ui.save_form_png("Mand3DForm", &out.join("explain_difs_fixed_main.png"), 1.0).unwrap();
                ui.show("FormulaGUIForm");
                app.process(&mut ui);
                ui.save_form_png("FormulaGUIForm", &out.join("explain_difs_fixed_formulas.png"), 1.0).unwrap();
            }
            // parameters that name a formula not installed
            "missing" => {
                mb3d::formulas::add_formula_dir(std::path::PathBuf::from("M3Formulas-rs"));
                let mut sc = mb3d::Scene::default();
                sc.formulas = vec![mb3d::scene::FormulaEntry { formula: mb3d::formulas::lookup("SphereTree").unwrap(), iterations: 1 }];
                let mut data = mb3d::m3p::write(&sc);
                let at = data.windows(10).position(|w| w == b"SphereTree").unwrap();
                data[at..at + 10].copy_from_slice(b"SphereTrex");
                let f = out.join("needs_a_formula.m3p");
                std::fs::write(&f, data).unwrap();
                mb3d::app::main_form::open_file(&mut app, &mut ui, &f);
                app.process(&mut ui);
                ui.save_form_png("MessageBoxDlg", &out.join("explain_missing_formula.png"), 1.0).unwrap();
                ui.click("MessageBoxDlg", "Btn0");
                app.process(&mut ui);
            }
            // the main window (the share icon in its title bar)
            "main" => {
                ui.save_form_png("Mand3DForm", &out.join("main.png"), 1.0).unwrap();
                let f = ui.fm("Mand3DForm");
                f.nc_hot = mb3d::vcl::form::NC_SHARE;
                ui.save_form_png("Mand3DForm", &out.join("main_share_hot.png"), 1.0).unwrap();
                let f = ui.fm("Mand3DForm");
                let r = f.share_rect.get();
                f.hint = Some((f.share_hint.clone(), r.x - f.frame_w() - 200, 2));
                f.led_hint_shown = true;
                ui.save_form_png("Mand3DForm", &out.join("main_share_icon.png"), 1.0).unwrap();
                let f = ui.fm("Mand3DForm");
                f.nc_hot = -1;
                f.hint = None;
                f.led_hint_shown = false;
            }
            // the Share window, and the files it makes
            "share" => {
                app.main.authors = ["Marije".into(), "Sebastian".into()];
                ui.show("ShareFrm");
                app.process(&mut ui);
                ui.set_text("ShareFrm", "TitleEdit", "Blue Bulb");
                ui.post("ShareFrm", "TitleEdit", "FieldChange", mb3d::vcl::Ev::Change);
                app.process(&mut ui);
                ui.save_form_png("ShareFrm", &out.join("share_window.png"), 1.0).unwrap();
                let dir = out.join("share-out");
                ui.set_text("ShareFrm", "FolderEdit", &dir.display().to_string());
                ui.click("ShareFrm", "MakeBtn");
                app.process(&mut ui);
                ui.save_form_png("ShareFrm", &out.join("share_window_made.png"), 1.0).unwrap();
                for e in std::fs::read_dir(dir.join("Blue Bulb")).unwrap().flatten() {
                    eprintln!("share file: {} ({} bytes)", e.path().display(), e.metadata().unwrap().len());
                }
            }
            // a 360 panorama shared: its JPEG gets the tags
            "pano" => {
                app.scene.optic = mb3d::scene::CameraOptic::Panorama;
                app.scene.width = 640;
                app.scene.height = 320;
                app.scene_to_forms(&mut ui);
                ui.click("Mand3DForm", "Button2");
                app.process(&mut ui);
                wait(&mut ui, &mut app);
                ui.save_form_png("Mand3DForm", &out.join("pano_main.png"), 1.0).unwrap();
                ui.show("ShareFrm");
                app.process(&mut ui);
                ui.set_text("ShareFrm", "TitleEdit", "Machine Hall");
                ui.set_text("ShareFrm", "FolderEdit", &out.join("share-out").display().to_string());
                ui.click("ShareFrm", "MakeBtn");
                app.process(&mut ui);
            }
            // a picture of any name chosen as a map
            "maps" => {
                let src_dir = out.join("pictures");
                std::fs::create_dir_all(&src_dir).unwrap();
                let (w, h) = (256usize, 256usize);
                let rgb: Vec<u8> = (0..w * h)
                    .flat_map(|i| {
                        let (x, y) = (i % w, i / w);
                        let brick = ((y / 32) % 2 * 32 + x) % 64 < 60 && y % 32 < 28;
                        if brick { [190u8, 90, 60] } else { [220, 210, 200] }
                    })
                    .collect();
                let pic = src_dir.join("Brick wall (rough).png");
                std::fs::write(&pic, mb3d::png::encode_rgb(w, h, &rgb)).unwrap();
                ui.show("LightAdjustForm");
                app.process(&mut ui);
                ui.set_active_page("LightAdjustForm", "PageControl2", "TabSheet3");
                ui.set_checked("LightAdjustForm", "CheckBox15", true);
                ui.post("LightAdjustForm", "CheckBox15", "CheckBox15Click", mb3d::vcl::Ev::Click);
                app.process(&mut ui);
                ui.post("", "light:map:Edit21", "", mb3d::vcl::Ev::DialogResult(mb3d::vcl::DialogResult::File(Some(pic.clone()))));
                app.process(&mut ui);
                eprintln!("diffuse map: {}", ui.text("LightAdjustForm", "Edit21"));
                ui.save_form_png("LightAdjustForm", &out.join("maps_light_window.png"), 1.0).unwrap();
                ui.save_form_png("Mand3DForm", &out.join("maps_main.png"), 1.0).unwrap();
                // a height map formula: the hint of its map number
                mb3d::formulas::add_formula_dir(std::path::PathBuf::from("M3Formulas-community"));
                app.scene.formulas = vec![mb3d::scene::FormulaEntry { formula: mb3d::formulas::lookup("HeightMapLimIFS").unwrap(), iterations: 1 }];
                app.scene_to_forms(&mut ui);
                ui.show("FormulaGUIForm");
                app.process(&mut ui);
                ui.post("", "formula:map:Edit1", "", mb3d::vcl::Ev::DialogResult(mb3d::vcl::DialogResult::File(Some(pic.clone()))));
                app.process(&mut ui);
                eprintln!("formula map: {}", ui.text("FormulaGUIForm", "Edit1"));
                let f = ui.fm("FormulaGUIForm");
                let id = f.id("Edit1").unwrap();
                let (hx, hy) = (f.ctl[id].left, f.ctl[id].top + 24);
                f.hint = Some((f.ctl[id].hint.clone(), hx - 120, hy + 40));
                ui.save_form_png("FormulaGUIForm", &out.join("maps_formula_window.png"), 1.0).unwrap();
                ui.fm("FormulaGUIForm").hint = None;
            }
            // the eyedropper on the picture
            "eyedropper" => {
                ui.recent_colors = vec![0xE0A030, 0x3060C0, 0x20A070];
                ui.pick_color("light:color:12", 0xFFFFFF);
                app.process(&mut ui);
                ui.click("ColorDlg", "PickBtn");
                app.process(&mut ui);
                ui.save_form_png("ColorDlg", &out.join("eyedropper_on.png"), 1.0).unwrap();
                // a pixel of the fractal in the main window's picture
                let mi = ui.form_index("Mand3DForm").unwrap();
                let theme = ui.theme;
                let f = &ui.forms[mi];
                let img = f.c("Image1");
                let (x, y) = (f.frame_w() + img.left + 300, f.frame_w() + f.caption_h(&theme) + img.top + 230);
                eprintln!("picked: {}", ui.eyedropper_pick(mi, x as f64 + 0.5, y as f64 + 0.5, 1.0));
                app.process(&mut ui);
                ui.save_form_png("ColorDlg", &out.join("eyedropper_picked.png"), 1.0).unwrap();
            }
            "colorform2" => {
                ui.show("ColorForm");
                app.process(&mut ui);
                let f = ui.fm("ColorForm");
                let id = f.id("ColorsFileBtn").unwrap();
                let (hx, hy) = (f.ctl[id].left, f.ctl[id].top);
                f.hint = Some((f.ctl[id].hint.clone(), hx - 300, hy - 40));
                ui.save_form_png("ColorForm", &out.join("color_form_buttons.png"), 1.0).unwrap();
                ui.fm("ColorForm").hint = None;
            }
            // Calculate 3D
            "calc" => {
                ui.click("Mand3DForm", "Button2");
                app.process(&mut ui);
                wait(&mut ui, &mut app);
            }
            "historyhot" => {
                ui.fm("Mand3DForm").nc_hot = mb3d::vcl::form::NC_HISTORY;
                ui.save_form_png("Mand3DForm", &out.join("main_history_hot.png"), 1.0).unwrap();
                ui.fm("Mand3DForm").nc_hot = -1;
            }
            o => eprintln!("unknown step {o}"),
        }
    }
}
