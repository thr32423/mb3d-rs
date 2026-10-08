//! Command line front end: `mb3d [options] [scene.m3s]`

use mb3d::formulas::Formula;
use mb3d::scene::Scene;
use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;

const USAGE: &str = "\
mb3d - Mandelbulb3D renderer (Rust port)

USAGE:
    mb3d [OPTIONS] [SCENE_FILE]
    mb3d gui [--port 8080] [--formulas DIR] [--maps DIR] [FILE]    editor in the browser
    mb3d animate ANIMATION | KEYFRAME_FILES... [OPTIONS]           render an animation
    mb3d batch FILES... | --list LISTFILE [OPTIONS]                render many files
    (mb3d animate --help, mb3d batch --help)

SCENE_FILE is an INI-style scene description (see examples/*.m3s), a
Mandelbulb3D parameter file (.m3p), image file (.m3i, parameters only) or a
text file containing MB3D text parameters (Mandelbulb3Dv18{...}).
Without a scene file the preset for --formula is rendered.

OPTIONS:
    -f, --formula <NAME>     Built-in formula preset (default: \"Integer Power\")
    -W, --width <N>          Image width
    -H, --height <N>         Image height
    -i, --iterations <N>     Max iterations
        --scale <F>          Scale the image size (e.g. 0.1 for a preview of a .m3p)
        --aa <N>             Anti-aliasing: render N times larger and downsample
    -o, --output <FILE>      Output PNG (default: mb3d.png)
        --depth <FILE>       Also write a 16 bit depth map PNG
    -s, --set <KEY=VALUE>    Override a scene key (may be repeated)
    -t, --threads <N>        Number of threads (default: all cores)
        --auto-color         Fit the colour range to the rendered surface
        --stats              Print G-buffer statistics
        --save-scene <FILE>  Write the final scene as .m3s text (e.g. to convert a .m3p)
        --save-text <FILE>   Write MB3D text parameters (Mandelbulb3Dv18{...})
        --save-m3p <FILE>    Write a MB3D parameter file (.m3p); parameter inputs are
                             copied unchanged, .m3s scenes are encoded
        --tiles <CxR>        Tiled rendering: calculate the image in C x R tiles one after
                             another (less memory for big images) and stitch them
        --tile <C,R>         Only render tile C,R (1-based) of --tiles; with --save-m3p
                             this writes a MB3D tile parameter file
        --render             Also render when converting with --save-scene/--save-text/--save-m3p
    -q, --quiet              No progress output
        --formulas <DIR>     Directory with .m3f formula files (also: MB3D_FORMULAS)
        --maps <DIR>         Directory with maps and background pictures (also: MB3D_MAPS;
                             M3Maps next to the formula directory is searched too)
        --list-formulas      List the built-in formulas and their options
    -h, --help               Show this help
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let all: Vec<String> = std::env::args().skip(1).collect();
    match all.first().map(String::as_str) {
        Some("gui") => return mb3d::gui::run(&all[1..]),
        Some("animate") | Some("anim") => return animate(&all[1..]),
        Some("batch") => return batch(&all[1..]),
        _ => {}
    }
    let mut args = std::env::args().skip(1);
    let mut formula: Option<String> = None;
    let mut scene_file: Option<String> = None;
    let mut output = "mb3d.png".to_string();
    let mut depth: Option<String> = None;
    let mut overrides: Vec<String> = Vec::new();
    let (mut quiet, mut stats, mut auto_color) = (false, false, false);
    let mut save_scene: Option<String> = None;
    let (mut save_text, mut save_m3p): (Option<String>, Option<String>) = (None, None);
    let mut render_too = false;
    let mut scale: Option<f64> = None;
    let mut aa: usize = 1;
    while let Some(a) = args.next() {
        let mut val = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match a.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(());
            }
            "--list-formulas" => {
                for n in Formula::all_names() {
                    let f = Formula::default_for(n).unwrap();
                    let opts: Vec<String> =
                        f.options().iter().map(|(k, v)| format!("{k} = {v}")).collect();
                    println!("{n:16} {}", opts.join(", "));
                }
                return Ok(());
            }
            "-f" | "--formula" => formula = Some(val(&a)?),
            "-W" | "--width" => overrides.push(format!("width = {}", val(&a)?)),
            "-H" | "--height" => overrides.push(format!("height = {}", val(&a)?)),
            "-i" | "--iterations" => overrides.push(format!("iterations = {}", val(&a)?)),
            "-t" | "--threads" => overrides.push(format!("threads = {}", val(&a)?)),
            "-s" | "--set" => overrides.push(val(&a)?),
            "-o" | "--output" => output = val(&a)?,
            "--depth" => depth = Some(val(&a)?),
            "--formulas" => mb3d::formulas::add_formula_dir(val(&a)?.into()),
            "--maps" => mb3d::maps::add_map_dir(val(&a)?.into()),
            "--auto-color" => auto_color = true,
            "--aa" => aa = val(&a)?.parse::<usize>().map_err(|_| "bad --aa value".to_string())?.clamp(1, 8),
            "--scale" => scale = Some(val(&a)?.parse::<f64>().map_err(|_| "bad --scale value".to_string())?),
            "--stats" => stats = true,
            "--save-scene" => save_scene = Some(val(&a)?),
            "--save-text" => save_text = Some(val(&a)?),
            "--save-m3p" => save_m3p = Some(val(&a)?),
            "--render" => render_too = true,
            "--tiles" => overrides.push(format!("tiles = {}", val(&a)?)),
            "--tile" => overrides.push(format!("tile = {}", val(&a)?)),
            "-q" | "--quiet" => quiet = true,
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n\n{USAGE}")),
            s => scene_file = Some(s.to_string()),
        }
    }

    let is_m3p = scene_file
        .as_deref()
        .map(|f| {
            let l = f.to_ascii_lowercase();
            l.ends_with(".m3p")
                || l.ends_with(".m3i")
                || std::fs::read(f).map(|d| String::from_utf8_lossy(&d).contains("Mandelbulb3Dv")).unwrap_or(false)
        })
        .unwrap_or(false);
    let raw_copy = is_m3p && overrides.is_empty() && scale.is_none();
    if raw_copy && (save_text.is_some() || save_m3p.is_some()) {
        let path = scene_file.clone().unwrap();
        let (raw, title) = mb3d::m3p::read_raw(std::path::Path::new(&path))?;
        if let Some(f) = &save_m3p {
            std::fs::write(f, &raw).map_err(|e| format!("{f}: {e}"))?;
            if !quiet {
                eprintln!("  wrote parameters {f}");
            }
        }
        if let Some(f) = &save_text {
            let title = title.unwrap_or_else(|| {
                std::path::Path::new(&path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
            });
            std::fs::write(f, mb3d::m3p::raw_to_text(&raw, &title)).map_err(|e| format!("{f}: {e}"))?;
            if !quiet {
                eprintln!("  wrote text parameters {f}");
            }
        }
        if !render_too {
            return Ok(());
        }
    }
    let mut scene = if is_m3p {
        let path = scene_file.clone().unwrap();
        let m = mb3d::m3p::load(std::path::Path::new(&path))?;
        if !quiet {
            eprintln!(
                "loaded {path} (MB3D parameter version {}, {}x{})",
                m.mand_id, m.scene.width, m.scene.height
            );
            for w in &m.warnings {
                eprintln!("  note: {w}");
            }
        }
        let mut s = m.scene;
        if let Some(f) = scale {
            s.scale_image(f);
        }
        s.apply(&overrides.join("\n"))?
    } else {
        // file (or preset) text + command line overrides, which are inserted
        // in front of the first [section]
        let mut text = match (&scene_file, &formula) {
            (Some(f), _) => std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?,
            (None, f) => Scene::preset(f.as_deref().unwrap_or("Integer Power"))?.to_text(),
        };
        if scene_file.is_some() {
            if let Some(f) = &formula {
                if let Some(i) = text.find("[formula]") {
                    text.truncate(i);
                }
                text.push_str(&format!("[formula]\nname = {f}\n"));
            }
        }
        let idx = if text.starts_with('[') { 0 } else { text.find("\n[").map(|i| i + 1).unwrap_or(text.len()) };
        text.insert_str(idx, &format!("{}\n", overrides.join("\n")));
        let mut s = Scene::parse(&text)?;
        if let Some(f) = scale {
            s.scale_image(f);
        }
        s
    };
    if let Some(f) = &save_scene {
        std::fs::write(f, scene.to_text()).map_err(|e| format!("{f}: {e}"))?;
        if !quiet {
            eprintln!("  wrote scene {f}");
        }
        if !render_too && save_text.is_none() && save_m3p.is_none() {
            return Ok(());
        }
    }
    if !raw_copy && (save_text.is_some() || save_m3p.is_some()) {
        // a scene: encode it as MB3D parameters
        let raw = mb3d::m3p::write(&scene);
        if let Some(f) = &save_m3p {
            std::fs::write(f, &raw).map_err(|e| format!("{f}: {e}"))?;
            if !quiet {
                eprintln!("  wrote parameters {f}");
            }
        }
        if let Some(f) = &save_text {
            let title = scene_file
                .as_deref()
                .and_then(|p| std::path::Path::new(p).file_stem().map(|s| s.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "mb3d-rs".into());
            std::fs::write(f, mb3d::m3p::raw_to_text(&raw, &title)).map_err(|e| format!("{f}: {e}"))?;
            if !quiet {
                eprintln!("  wrote text parameters {f}");
            }
        }
        if !render_too {
            return Ok(());
        }
    }
    if aa > 1 {
        scene.scale_image(aa as f64);
    }
    if !quiet {
        let names: Vec<String> =
            scene.formulas.iter().map(|f| format!("{} x{}", f.formula.name(), f.iterations)).collect();
        eprintln!(
            "rendering {}x{}  formulas: {}  max its: {}",
            scene.width,
            scene.height,
            names.join(" + "),
            scene.iterations
        );
    }
    let progress = |done: usize, total: usize| {
        if !quiet && (done % 8 == 0 || done == total) {
            eprint!("\r  {:5.1}%", 100.0 * done as f64 / total as f64);
            let _ = std::io::stderr().flush();
        }
    };
    if let Some(tl) = scene.tiling {
        // anti-aliasing of tile renders: --aa and the tile downscale
        let down = aa * tl.downscale.max(1) as usize;
        if tl.downscale > 1 {
            scene.scale_image(tl.downscale as f64);
        }
        if auto_color {
            // colour range from a small preview of the whole image
            let mut pv = scene.clone();
            pv.tiling = None;
            pv.scale_image((320.0 / pv.width as f64).min(1.0));
            let (_, g) = mb3d::render::calculate(&pv, &|_, _| {})?;
            mb3d::render::auto_color_range(&mut scene, &g);
        }
        if !quiet {
            match tl.pos {
                Some((c, r)) => eprintln!("  tile {}, {} of {}x{}", c + 1, r + 1, tl.cols, tl.rows),
                None => eprintln!("  {}x{} tiles", tl.cols, tl.rows),
            }
        }
        let tile_progress = |done: usize, total: usize, t: [i32; 4]| {
            if !quiet {
                eprintln!("\r  tile {done}/{total} done ({}x{} at {}, {})          ", t[2], t[3], t[0], t[1]);
            }
        };
        let res = mb3d::render::render_tiled(&scene, stats || depth.is_some(), &progress, &tile_progress)?;
        if !quiet {
            eprintln!("  done: calc {:.2}s, paint {:.2}s", res.calc_seconds, res.paint_seconds);
        }
        if let (true, Some(g)) = (stats, &res.gbuffer) {
            print_stats(&mb3d::RenderResult {
                width: res.width,
                height: res.height,
                gbuffer: g.clone(),
                rgb: Vec::new(),
                calc_seconds: 0.0,
                paint_seconds: 0.0,
            });
        }
        let (rgb, ow, oh) = if down > 1 {
            mb3d::render::downsample(&res.rgb, res.width, res.height, down)
        } else {
            (res.rgb, res.width, res.height)
        };
        mb3d::png::write_rgb(&output, ow, oh, &rgb).map_err(|e| format!("{output}: {e}"))?;
        if let (Some(d), Some(g)) = (depth, &res.gbuffer) {
            write_depth(&d, g, res.width, res.height)?;
        }
    } else {
        render_single(&mut scene, auto_color, stats, quiet, aa, &output, depth, &progress)?;
    }
    let errs = mb3d::iteration::CUSTOM_ERRORS.load(std::sync::atomic::Ordering::Relaxed);
    if errs > 0 {
        eprintln!("  warning: {errs} custom formula calls failed in the interpreter");
    }
    if !quiet {
        eprintln!("  wrote {output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_single(
    scene: &mut Scene,
    auto_color: bool,
    stats: bool,
    quiet: bool,
    aa: usize,
    output: &str,
    depth: Option<String>,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<(), String> {
    let t0 = Instant::now();
    let (params, gbuffer) = mb3d::render::calculate(scene, progress)?;
    let t1 = Instant::now();
    if auto_color {
        mb3d::render::auto_color_range(scene, &gbuffer);
        if !quiet {
            eprintln!(
                "\r  auto colour range: color_start = {:.1}, color_end = {:.1}",
                scene.lighting.color_start, scene.lighting.color_end
            );
        }
    }
    let rgb = mb3d::render::paint(scene, &params, &gbuffer);
    let res = mb3d::RenderResult {
        width: scene.width as usize,
        height: scene.height as usize,
        gbuffer,
        rgb,
        calc_seconds: (t1 - t0).as_secs_f64(),
        paint_seconds: t1.elapsed().as_secs_f64(),
    };
    if !quiet {
        eprintln!(
            "\r  done: calc {:.2}s, paint {:.2}s, object coverage {:.1}%",
            res.calc_seconds,
            res.paint_seconds,
            res.coverage() * 100.0
        );
    }
    if stats {
        print_stats(&res);
        // the DOF focus value (sDOFZsharp) that makes the image centre sharp
        let c = &res.gbuffer[res.height / 2 * res.width + res.width / 2];
        if !c.is_background() {
            let z = (((8388352 - (c.zpos_fine >> 8) as i64) as f64 / params.zc_mul + 1.0).powi(2) - 1.0) / params.zcorr;
            eprintln!("  dof_focus for the image centre: {:.5}", z / res.width as f64);
        }
    }
    let (rgb, ow, oh) = if aa > 1 {
        mb3d::render::downsample(&res.rgb, res.width, res.height, aa)
    } else {
        (res.rgb.clone(), res.width, res.height)
    };
    mb3d::png::write_rgb(&output, ow, oh, &rgb).map_err(|e| format!("{output}: {e}"))?;
    if let Some(d) = depth {
        write_depth(&d, &res.gbuffer, res.width, res.height)?;
    }
    Ok(())
}

fn write_depth(path: &str, g: &[mb3d::gbuffer::SiLight], w: usize, h: usize) -> Result<(), String> {
    let z: Vec<u16> =
        g.iter().map(|s| if s.is_background() { 0 } else { (65535 - (s.zpos() * 2).min(65535)) as u16 }).collect();
    mb3d::png::write_gray16(path, w, h, &z).map_err(|e| format!("{path}: {e}"))
}

fn print_stats(res: &mb3d::RenderResult) {
    let hits: Vec<&mb3d::gbuffer::SiLight> =
        res.gbuffer.iter().filter(|s| !s.is_background()).collect();
    if hits.is_empty() {
        eprintln!("  no object pixels");
        return;
    }
    let pct = |mut v: Vec<u32>| -> String {
        v.sort_unstable();
        let q = |f: f64| v[((v.len() - 1) as f64 * f) as usize];
        format!("min {} p10 {} p50 {} p90 {} max {}", q(0.0), q(0.1), q(0.5), q(0.9), q(1.0))
    };
    eprintln!("  si_gradient: {}", pct(hits.iter().map(|s| s.si_gradient as u32).collect()));
    eprintln!("  otrap:       {}", pct(hits.iter().map(|s| s.otrap as u32).collect()));
    eprintln!("  zpos:        {}", pct(hits.iter().map(|s| s.zpos()).collect()));
    eprintln!("  de steps:    {}", pct(res.gbuffer.iter().map(|s| (s.shadow & 0x3FF) as u32).collect()));
    eprintln!("  amb shadow:  {}", pct(hits.iter().map(|s| s.amb_shadow as u32).collect()));
    let lit: Vec<String> = (0..6)
        .map(|i| {
            let n = hits.iter().filter(|s| s.shadow & (0x400 << i) != 0).count();
            format!("{:.0}%", 100.0 * n as f64 / hits.len() as f64)
        })
        .collect();
    eprintln!("  shadow bits: {} (lights 1-6, or soft value bits)", lit.join(" "));
}

// ---------------------------------------------------------------------------
// mb3d animate

const ANIMATE_USAGE: &str = "\
mb3d animate - render the frames of an animation (MB3D's animation maker)

USAGE:
    mb3d animate ANIMATION [OPTIONS]            a .m3k animation (text) or MB3D .m3a file
    mb3d animate KEYFRAME KEYFRAME... [OPTIONS] keyframes from parameter files (.m3p, .m3s,
                                                text parameters), in this order

Frames are written as <output>/<name><index>.<format> with a 6 digit index,
like MB3D (e.g. frames/flight000001.png).

OPTIONS:
    -o, --output <DIR>       Output folder (default: the animation's, else the current folder)
        --name <NAME>        Project name = file name prefix (default: name of the animation)
        --format <F>         png (default), bmp, or m3p: a parameter file per frame (to
                             render the frames elsewhere, e.g. with mb3d batch)
    -W, --width <N>          Frame width   (default: the animation's / first keyframe's)
    -H, --height <N>         Frame height
        --aa <N>             Calculate frames N times larger and reduce them
                             (MB3D's image scale, anti-aliasing)
        --frames <N>         Sub-frames from each keyframe to the next (keyframe files;
                             with an animation file: for all keyframes)
        --linear             Linear interpolation (default: MB3D's quadratic bezier)
        --bezier             Quadratic bezier interpolation
        --loop               Loop: after the last keyframe the first follows again
        --no-loop
        --start-index <N>    File index of the first frame (default 1)
        --index-step <N>     Increment of the file index (default 1)
        --from <INDEX>       Render only frames from this file index ...
        --to <INDEX>         ... up to this file index
        --frame <INDEX>      Render only this frame
        --every <N>          Render only every N-th frame (quick preview)
        --scale <F>          Render the frames F times the size (e.g. 0.25 for a preview)
        --skip-existing      Keep frames that exist already; continues an interrupted
                             render, and lets several processes share the work
        --overwrite          Render all frames again (default unless the animation says no)
        --depth              Also write depth images (ZBuf <name><index>.png)
    -s, --set <KEY=VALUE>    Change a scene key in every keyframe (may be repeated)
        --save <FILE>        Save the animation: .m3k (text) or .m3a (MB3D); with
                             keyframe files only the file is written unless --render
        --render             Render too when saving
        --list               Print the frames (file index, keyframe, sub-frame) and exit
    -t, --threads <N>        Number of threads (default: all cores)
    -q, --quiet              No progress output
        --formulas <DIR>     Directory with .m3f formula files
        --maps <DIR>         Directory with maps and background pictures
";

fn animate(argv: &[String]) -> Result<(), String> {
    use mb3d::anim::{Animation, Interpolation, Keyframe, OutputFormat};
    let mut files: Vec<String> = Vec::new();
    let mut overrides: Vec<String> = Vec::new();
    let mut run = mb3d::frames::FrameRun::default();
    let (mut output, mut name, mut format): (Option<String>, Option<String>, Option<String>) = (None, None, None);
    let (mut width, mut height, mut aa, mut frames): (Option<i32>, Option<i32>, Option<u32>, Option<u32>) = (None, None, None, None);
    let (mut ipol, mut looped): (Option<Interpolation>, Option<bool>) = (None, None);
    let (mut start_index, mut index_step): (Option<i64>, Option<i64>) = (None, None);
    let mut overwrite: Option<bool> = None;
    let (mut depth, mut list, mut render_too, mut quiet) = (false, false, false, false);
    let mut save: Option<String> = None;
    let mut args = argv.iter();
    let num = |a: &str, v: Option<&String>| -> Result<i64, String> {
        v.ok_or_else(|| format!("{a} needs a value"))?.trim().parse::<i64>().map_err(|_| format!("bad value for {a}"))
    };
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{ANIMATE_USAGE}");
                return Ok(());
            }
            "-o" | "--output" => output = Some(args.next().ok_or("--output needs a value")?.clone()),
            "--name" => name = Some(args.next().ok_or("--name needs a value")?.clone()),
            "--format" => format = Some(args.next().ok_or("--format needs a value")?.clone()),
            "-W" | "--width" => width = Some(num(a, args.next())?.clamp(1, 65535) as i32),
            "-H" | "--height" => height = Some(num(a, args.next())?.clamp(1, 65535) as i32),
            "--aa" => aa = Some(num(a, args.next())?.clamp(1, 16) as u32),
            "--frames" => frames = Some(num(a, args.next())?.clamp(0, 1_000_000) as u32),
            "--linear" => ipol = Some(Interpolation::Linear),
            "--bezier" => ipol = Some(Interpolation::Bezier),
            "--loop" => looped = Some(true),
            "--no-loop" => looped = Some(false),
            "--start-index" => start_index = Some(num(a, args.next())?),
            "--index-step" => index_step = Some(num(a, args.next())?.max(1)),
            "--from" => run.from_index = Some(num(a, args.next())?),
            "--to" => run.to_index = Some(num(a, args.next())?),
            "--frame" => {
                let i = num(a, args.next())?;
                run.from_index = Some(i);
                run.to_index = Some(i);
            }
            "--every" => run.every = num(a, args.next())?.max(1) as usize,
            "--scale" => {
                run.preview = args
                    .next()
                    .ok_or("--scale needs a value")?
                    .parse::<f64>()
                    .ok()
                    .filter(|f| *f > 0.0)
                    .ok_or("bad --scale value")?
            }
            "--skip-existing" => overwrite = Some(false),
            "--overwrite" => overwrite = Some(true),
            "--depth" => depth = true,
            "-s" | "--set" => overrides.push(args.next().ok_or("--set needs a value")?.clone()),
            "--save" => save = Some(args.next().ok_or("--save needs a value")?.clone()),
            "--render" => render_too = true,
            "--list" => list = true,
            "-t" | "--threads" => run.threads = num(a, args.next())?.max(0) as usize,
            "-q" | "--quiet" => quiet = true,
            "--formulas" => mb3d::formulas::add_formula_dir(args.next().ok_or("--formulas needs a value")?.into()),
            "--maps" => mb3d::maps::add_map_dir(args.next().ok_or("--maps needs a value")?.into()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n\n{ANIMATE_USAGE}")),
            s => files.push(s.to_string()),
        }
    }
    if files.is_empty() {
        return Err(format!("no animation or keyframe files given\n\n{ANIMATE_USAGE}"));
    }
    let is_anim = |f: &str| {
        let l = f.to_ascii_lowercase();
        l.ends_with(".m3a") || l.ends_with(".m3k")
    };
    let note = |m: &str| {
        if !quiet {
            eprintln!("  note: {m}");
        }
    };
    let mut previews: Vec<Option<mb3d::animfile::Preview>>;
    let from_file = files.len() == 1 && is_anim(&files[0]);
    let mut anim = if from_file {
        let path = std::path::Path::new(&files[0]);
        let f = mb3d::animfile::load(path)?;
        for w in &f.warnings {
            note(w);
        }
        previews = f.previews;
        let mut a = f.anim;
        if !quiet {
            eprintln!("loaded {} ({} keyframes, {} frames)", files[0], a.keyframes.len(), a.frame_count());
        }
        // MB3D's output folder is an absolute Windows path
        let foreign = a.output_folder.contains('\\') || a.output_folder.get(1..2) == Some(":");
        if foreign && !cfg!(windows) && output.is_none() {
            note(&format!("output folder '{}' of the .m3a file is not usable here, using the current folder (see --output)", a.output_folder));
            a.output_folder.clear();
        }
        if let Some(n) = frames {
            for k in a.keyframes.iter_mut() {
                k.frames = n;
            }
        }
        a
    } else {
        if files.iter().any(|f| is_anim(f)) {
            return Err("give either one animation file or keyframe parameter files".into());
        }
        let mut a = Animation { name: "anim".into(), ..Default::default() };
        for f in &files {
            let (s, w) = mb3d::animfile::load_scene(std::path::Path::new(f))?;
            for w in w {
                note(&format!("{f}: {w}"));
            }
            let mut k = Keyframe::new(s, frames.unwrap_or(50));
            k.source = Some(f.clone());
            a.keyframes.push(k);
        }
        let first = a.keyframes[0].scene.clone();
        a.set_size_from(&first);
        previews = vec![None; a.keyframes.len()];
        a
    };
    if !overrides.is_empty() {
        let text = overrides.join("\n");
        for (i, k) in anim.keyframes.iter_mut().enumerate() {
            k.scene = k.scene.clone().apply(&text).map_err(|e| format!("keyframe {}: {e}", i + 1))?;
            if let Some(src) = &mut k.source {
                for o in &overrides {
                    src.push('\u{0}');
                    src.push_str(o);
                }
            }
        }
        previews.iter_mut().for_each(|p| *p = None);
    }
    if let Some(w) = width {
        anim.width = w;
    }
    if let Some(h) = height {
        anim.height = h;
    }
    if let Some(n) = aa {
        anim.scale = n;
    }
    if let Some(i) = ipol {
        anim.interpolation = i;
    }
    if let Some(l) = looped {
        anim.looped = l;
    }
    if let Some(v) = start_index {
        anim.start_index = v;
    }
    if let Some(v) = index_step {
        anim.index_step = v;
    }
    if let Some(o) = overwrite {
        anim.overwrite = o;
    }
    if depth {
        anim.save_depth = true;
    }
    if let Some(n) = name {
        anim.name = n;
    }
    if let Some(o) = output {
        anim.output_folder = o;
    }
    if let Some(f) = format {
        anim.format = OutputFormat::parse(&f)?;
    }
    if anim.format == OutputFormat::Jpg {
        note("JPEG output is not supported, writing PNG");
        anim.format = OutputFormat::Png;
    }
    if anim.keyframes.len() < 2 && !list && save.is_none() {
        return Err("an animation needs at least 2 keyframes".into());
    }

    if let Some(path) = &save {
        let p = std::path::Path::new(path);
        let m3a = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3a"));
        if m3a {
            for (i, pv) in previews.iter_mut().enumerate() {
                if pv.is_none() {
                    *pv = mb3d::frames::keyframe_preview(&anim, i).ok();
                }
            }
        }
        let mut a2 = anim.clone();
        if !m3a {
            // keyframe file references relative to the saved file
            if let Some(dir) = p.parent().and_then(|d| std::fs::canonicalize(if d.as_os_str().is_empty() { std::path::Path::new(".") } else { d }).ok()) {
                for k in a2.keyframes.iter_mut() {
                    if let Some(src) = &mut k.source {
                        let (f, rest) = src.split_once('\u{0}').map(|(a, b)| (a.to_string(), format!("\u{0}{b}"))).unwrap_or((src.clone(), String::new()));
                        if let Ok(abs) = std::fs::canonicalize(&f) {
                            let r = abs.strip_prefix(&dir).map(|r| r.to_path_buf()).unwrap_or(abs);
                            *src = format!("{}{rest}", r.to_string_lossy());
                        }
                    }
                }
                if let Ok(abs) = std::fs::canonicalize(if a2.output_folder.is_empty() { "." } else { &a2.output_folder }) {
                    if !a2.output_folder.is_empty() {
                        a2.output_folder = abs.strip_prefix(&dir).map(|r| r.to_path_buf()).unwrap_or(abs).to_string_lossy().into_owned();
                    }
                }
            }
        }
        mb3d::animfile::save(p, &a2, &previews)?;
        if !quiet {
            eprintln!("  wrote {path} ({} keyframes, {} frames)", anim.keyframes.len(), anim.frame_count());
        }
        if !render_too && !list {
            return Ok(());
        }
    }

    let frames_to_do = run.frames(&anim);
    if list {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{} frames, {} keyframes, {}x{}{}", anim.frame_count(), anim.keyframes.len(), anim.width, anim.height,
            if anim.scale > 1 { format!(" (calculated x{})", anim.scale) } else { String::new() });
        for &f in &frames_to_do {
            let p = anim.frame_pos(f).unwrap();
            let k = &anim.keyframes[p.key];
            if writeln!(out, "{:6}  keyframe {:3}  {:4}/{:<4}  {}", anim.file_index(f), p.key + 1, p.sub, k.frames, anim.frame_file(f).display()).is_err() {
                break;
            }
        }
        return Ok(());
    }
    if frames_to_do.is_empty() {
        return Err("no frames to render (check --from/--to/--frame)".into());
    }
    if !quiet {
        eprintln!(
            "rendering {} of {} frames at {}x{}{} to {}",
            frames_to_do.len(),
            anim.frame_count(),
            anim.width,
            anim.height,
            if anim.scale > 1 { format!(" (x{} anti-aliasing)", anim.scale) } else { String::new() },
            anim.frame_file(frames_to_do[0]).parent().map(|p| p.display().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| ".".into())
        );
    }
    let t0 = Instant::now();
    let (mut done, mut skipped, mut secs) = (0usize, 0usize, 0.0f64);
    let tty = std::io::IsTerminal::is_terminal(&std::io::stderr());
    for (n, &f) in frames_to_do.iter().enumerate() {
        let label = format!("frame {}/{} ({:06})", n + 1, frames_to_do.len(), anim.file_index(f));
        let progress = |d: usize, t: usize| {
            if !quiet && tty && (d % 8 == 0 || d == t) {
                eprint!("\r  {label} {:5.1}%", 100.0 * d as f64 / t as f64);
                let _ = std::io::stderr().flush();
            }
        };
        match mb3d::frames::render_frame_to_file(&anim, f, &run, &progress, &|| false)? {
            mb3d::frames::FrameResult::Written { path, seconds } => {
                done += 1;
                secs += seconds;
                if !quiet {
                    let left = frames_to_do.len() - n - 1;
                    let eta = if done > 0 { secs / done as f64 * left as f64 } else { 0.0 };
                    let cr = if tty { "\r" } else { "" };
                    eprintln!("{cr}  {label} {:.1}s  {}  (left: {})          ", seconds, path.display(), fmt_dur(eta));
                }
            }
            mb3d::frames::FrameResult::Skipped { path, why } => {
                skipped += 1;
                if !quiet {
                    let w = if why == mb3d::frames::Skip::Busy { "being rendered by another process" } else { "exists" };
                    eprintln!("  {label} skipped, {} {w}", path.display());
                }
            }
        }
    }
    if !quiet {
        eprintln!("done: {done} frames rendered, {skipped} skipped, {}", fmt_dur(t0.elapsed().as_secs_f64()));
    }
    Ok(())
}

fn fmt_dur(s: f64) -> String {
    let s = s.max(0.0).round() as u64;
    if s >= 3600 {
        format!("{}h{:02}m", s / 3600, (s / 60) % 60)
    } else if s >= 60 {
        format!("{}m{:02}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

const BATCH_USAGE: &str = "\
mb3d batch - render many parameter files (MB3D's batch rendering)

USAGE:
    mb3d batch FILE_OR_DIR... [OPTIONS]
    mb3d batch --list LISTFILE [OPTIONS]

Inputs are .m3p, .m3i, .m3s or text parameter files; a directory stands for
all parameter files in it. A list file has one parameter file per line,
optionally followed by scene changes separated by '|':
    6 AM - Torii temple.m3p | width = 1920 | height = 1080
Each file is rendered to <name>.png next to it (or in --output). Files that
fail are reported and the batch goes on.

OPTIONS:
        --list <FILE>        Read the files from a list file (may be repeated)
    -o, --output <DIR>       Output folder (default: next to each file)
        --format <F>         png (default), bmp, or m3p (convert to MB3D parameter files)
        --skip-existing      Keep images that exist already: continues an interrupted
                             batch, and lets several processes share a list
    -s, --set <KEY=VALUE>    Change a scene key in every file (may be repeated)
        --scale <F>          Size factor (e.g. 0.25 for previews)
        --aa <N>             Anti-aliasing: render N times larger and reduce
        --depth              Also write <name>_depth.png
        --dry-run            Only list what would be rendered
    -t, --threads <N>        Number of threads (default: all cores)
    -q, --quiet              No progress output
        --formulas <DIR>     Directory with .m3f formula files
        --maps <DIR>         Directory with maps and background pictures
";

fn batch(argv: &[String]) -> Result<(), String> {
    use mb3d::batch::{BatchOptions, ItemResult};
    let mut opts = BatchOptions::default();
    let mut inputs: Vec<String> = Vec::new();
    let mut lists: Vec<String> = Vec::new();
    let (mut quiet, mut dry) = (false, false);
    let mut args = argv.iter();
    while let Some(a) = args.next() {
        let mut val = || args.next().cloned().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "-h" | "--help" => {
                print!("{BATCH_USAGE}");
                return Ok(());
            }
            "--list" => lists.push(val()?),
            "-o" | "--output" => opts.output_dir = Some(val()?.into()),
            "--format" => opts.format = mb3d::anim::OutputFormat::parse(&val()?)?,
            "--skip-existing" => opts.skip_existing = true,
            "-s" | "--set" => opts.overrides.push(val()?),
            "--scale" => opts.scale = Some(val()?.parse::<f64>().ok().filter(|f| *f > 0.0).ok_or("bad --scale value")?),
            "--aa" => opts.aa = val()?.parse::<usize>().map_err(|_| "bad --aa value".to_string())?.clamp(1, 8),
            "--depth" => opts.depth = true,
            "--dry-run" => dry = true,
            "-t" | "--threads" => opts.threads = val()?.parse::<usize>().map_err(|_| "bad --threads value".to_string())?,
            "-q" | "--quiet" => quiet = true,
            "--formulas" => mb3d::formulas::add_formula_dir(val()?.into()),
            "--maps" => mb3d::maps::add_map_dir(val()?.into()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n\n{BATCH_USAGE}")),
            s => inputs.push(s.to_string()),
        }
    }
    if opts.format == mb3d::anim::OutputFormat::Jpg {
        return Err("JPEG output is not supported, use png or bmp".into());
    }
    let mut items = mb3d::batch::expand_inputs(&inputs)?;
    for l in &lists {
        let text = std::fs::read_to_string(l).map_err(|e| format!("{l}: {e}"))?;
        let base = std::path::Path::new(l).parent().unwrap_or(std::path::Path::new(""));
        items.extend(mb3d::batch::parse_list(&text, base).map_err(|e| format!("{l}: {e}"))?);
    }
    if items.is_empty() {
        return Err(format!("no parameter files given\n\n{BATCH_USAGE}"));
    }
    let outputs = mb3d::batch::output_paths(&items, &opts);
    if dry {
        let mut out = std::io::stdout().lock();
        for (it, o) in items.iter().zip(&outputs) {
            let extra = if it.overrides.is_empty() { String::new() } else { format!("  ({})", it.overrides.join(", ")) };
            let state = if o.exists() && opts.skip_existing { "  [exists, skipped]" } else { "" };
            if writeln!(out, "{} -> {}{extra}{state}", it.input.display(), o.display()).is_err() {
                break; // e.g. piped into head
            }
        }
        return Ok(());
    }
    let tty = std::io::IsTerminal::is_terminal(&std::io::stderr());
    let t0 = Instant::now();
    let (mut done, mut skipped) = (0usize, 0usize);
    let mut failed: Vec<(String, String)> = Vec::new();
    for (n, (it, out)) in items.iter().zip(&outputs).enumerate() {
        let name = it.input.display().to_string();
        let label = format!("[{}/{}] {name}", n + 1, items.len());
        let progress = |d: usize, t: usize| {
            if !quiet && tty && (d % 8 == 0 || d == t) {
                eprint!("\r  {label} {:5.1}%", 100.0 * d as f64 / t as f64);
                let _ = std::io::stderr().flush();
            }
        };
        let cr = if tty { "\r" } else { "" };
        match mb3d::batch::render_item(it, out, &opts, &progress, &|| false) {
            ItemResult::Done { output, seconds, notes } => {
                done += 1;
                if !quiet {
                    eprintln!("{cr}  {label} -> {} ({:.1}s)          ", output.display(), seconds);
                    for w in notes {
                        eprintln!("      note: {w}");
                    }
                }
            }
            ItemResult::Skipped { output, why } => {
                skipped += 1;
                if !quiet {
                    let w = if why == mb3d::frames::Skip::Busy { "being rendered by another process" } else { "exists" };
                    eprintln!("  {label} skipped, {} {w}", output.display());
                }
            }
            ItemResult::Failed { error } => {
                if !quiet {
                    eprintln!("{cr}  {label} FAILED: {error}          ");
                }
                failed.push((name, error));
            }
        }
    }
    if !quiet {
        eprintln!(
            "batch done: {done} rendered, {skipped} skipped, {} failed, {}",
            failed.len(),
            fmt_dur(t0.elapsed().as_secs_f64())
        );
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} of {} files failed:\n{}",
            failed.len(),
            items.len(),
            failed.iter().map(|(n, e)| format!("  {n}: {e}")).collect::<Vec<_>>().join("\n")
        ))
    }
}
