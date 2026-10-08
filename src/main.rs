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
    if all.first().map(String::as_str) == Some("gui") {
        return mb3d::gui::run(&all[1..]);
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
