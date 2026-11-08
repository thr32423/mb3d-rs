//! The screenshots of the Droste effect's documentation, without windows:
//! `drosteshots OUTDIR [file.m3p]` calculates the file (or the start
//! scene), switches the effect on, opens the Droste window and writes it
//! and the main window as PNG files; then *Paint now* and the main window
//! again with the effect painted at full size.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = std::path::PathBuf::from(args.first().cloned().unwrap_or_else(|| "drosteshots".into()));
    std::fs::create_dir_all(&out).unwrap();
    let (mut ui, mut app) = mb3d::app::build(&args[1.min(args.len())..]).unwrap().unwrap();
    let wait = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
        for _ in 0..3000 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            mb3d::vcl::App::idle(app, ui);
            app.process(ui);
            if !app.eng.running() && !app.main.calculating {
                break;
            }
        }
    };
    let settle = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
        for _ in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            mb3d::vcl::App::idle(app, ui);
            app.process(ui);
        }
    };
    wait(&mut ui, &mut app);
    // Calculate 3D
    ui.click("Mand3DForm", "Button2");
    app.process(&mut ui);
    wait(&mut ui, &mut app);
    ui.save_form_png("Mand3DForm", &out.join("main_before.png"), 1.0).unwrap();
    // the Post processing window's Finishing section with the button
    ui.show("PostProForm");
    settle(&mut ui, &mut app);
    ui.click("PostProForm", "FinishBtn");
    settle(&mut ui, &mut app);
    ui.save_form_png("PostProForm", &out.join("postpro_finishing.png"), 1.0).unwrap();
    // the Droste window, the effect on
    ui.click("PostProForm", "DrosteBtn");
    settle(&mut ui, &mut app);
    // (a click toggles the check box: on)
    ui.click("DrosteFrm", "OnCheck");
    settle(&mut ui, &mut app);
    assert!(app.scene.droste.is_some(), "the click did not switch the effect on");
    ui.save_form_png("DrosteFrm", &out.join("droste_window.png"), 1.0).unwrap();
    // Paint now: the effect at full size
    ui.click("DrosteFrm", "PaintBtn");
    app.process(&mut ui);
    wait(&mut ui, &mut app);
    settle(&mut ui, &mut app);
    ui.save_form_png("Mand3DForm", &out.join("main_after.png"), 1.0).unwrap();
    ui.save_form_png("DrosteFrm", &out.join("droste_window_painted.png"), 1.0).unwrap();
    // the picture as the program saves it
    if let Some(rgb) = app.eng.0.out.lock().unwrap().rgb.clone() {
        let o = app.eng.0.out.lock().unwrap();
        let (w, h) = (o.w, o.h);
        drop(o);
        let mut f = std::fs::File::create(out.join("picture_after.ppm")).unwrap();
        use std::io::Write;
        write!(f, "P6\n{w} {h}\n255\n").unwrap();
        f.write_all(&rgb).unwrap();
    }
    eprintln!("written to {}", out.display());
}
