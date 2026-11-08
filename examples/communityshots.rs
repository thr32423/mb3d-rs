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
            o => eprintln!("unknown step {o}"),
        }
    }
}
