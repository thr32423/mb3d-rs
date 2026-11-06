//! Headless check of the non-blocking picture save: calculate a 3D image,
//! then save it as 8 bit and 16 bit PNG through the real dialog handler and
//! confirm the write runs on a background thread (app.main.saving goes up at
//! once) and finishes through the idle loop with a valid file.
//! `cargo run --example savetest -- OUTDIR [file.m3p]`

use mb3d::vcl::form::DialogResult;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = std::path::PathBuf::from(args.first().cloned().unwrap_or_else(|| "savetest".into()));
    std::fs::create_dir_all(&out).unwrap();
    let (mut ui, mut app) = mb3d::app::build(&args[1.min(args.len())..]).unwrap().unwrap();

    let pump = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d, max: usize| {
        for _ in 0..max {
            std::thread::sleep(std::time::Duration::from_millis(50));
            mb3d::vcl::App::idle(app, ui);
            app.process(ui);
            if !app.eng.running() && !app.main.calculating && !app.main.saving {
                return true;
            }
        }
        false
    };

    // let the auto 2D slice settle, then Calculate 3D and wait for the g-buffer
    pump(&mut ui, &mut app, 120);
    ui.click("Mand3DForm", "Button2");
    app.process(&mut ui);
    pump(&mut ui, &mut app, 600);
    assert!(app.eng.base().is_some(), "no 3D image was calculated");

    let shots = std::env::var("SAVESHOTS").is_ok();
    for (name, filter_index) in [("test8.png", 1usize), ("test16.png", 2usize)] {
        let path = out.join(name);
        let _ = std::fs::remove_file(&path);
        ui.filter_index = filter_index;
        // the real save-as-PNG handler (as the save dialog returns a file)
        mb3d::app::main_form::dialog(&mut app, &mut ui, "savepng", &DialogResult::File(Some(path.clone())));

        // the save is on a background thread now, not blocking us
        assert!(app.main.saving, "{name}: save did not go to the background");
        // the window as it is the instant the save begins: live, "Saving …"
        if shots {
            ui.save_form_png("Mand3DForm", &out.join(format!("during_{name}")), 1.0).unwrap();
        }

        let done = pump(&mut ui, &mut app, 400);
        assert!(done && !app.main.saving, "{name}: save did not finish");
        let meta = std::fs::metadata(&path).unwrap_or_else(|e| panic!("{name}: no file: {e}"));
        assert!(meta.len() > 1000, "{name}: file too small ({} bytes)", meta.len());
        // a valid PNG (signature)
        let sig = std::fs::read(&path).unwrap()[..8].to_vec();
        assert_eq!(sig, [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'], "{name}: not a PNG");
        if shots {
            ui.save_form_png("Mand3DForm", &out.join(format!("after_{name}")), 1.0).unwrap();
        }
        eprintln!("{name}: {} bytes, background save OK", meta.len());
    }
    eprintln!("savetest: non-blocking save works");
}
