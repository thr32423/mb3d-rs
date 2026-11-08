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
            o => eprintln!("unknown step {o}"),
        }
    }
}
