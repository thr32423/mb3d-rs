//! The shadow and ambient occlusion passes of the graphics card against
//! the CPU's: the G-buffers of a scene are calculated both ways and the
//! shadow bits and `amb_shadow` compared pixel by pixel.
//! `cargo run --release --example postcheck -- WIDTH MB3D_DIR FILES...`
use mb3d::gbuffer::SiLight;
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let width: f64 = a[0].parse().unwrap();
    let mb3d = std::path::PathBuf::from(&a[1]);
    mb3d::formulas::add_formula_dir(mb3d.join("M3Formulas"));
    mb3d::maps::add_map_dir(mb3d.join("M3Maps"));
    mb3d::gpu::set_enabled(true);
    for f in &a[2..] {
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        let Ok((mut sc, _)) = mb3d::animfile::load_scene(std::path::Path::new(f)) else { continue };
        sc.scale_image(width / sc.width.max(1) as f64);
        let Ok(p) = mb3d::calc::CalcParams::new(&sc) else { continue };
        let job = mb3d::render::post_job(&sc, &p);
        // screen space AO: the CPU's and the card's on the same G-buffer
        if let Some(ao) = sc.ao.filter(|_| sc.deao.is_none()) {
            let (pc, _) = mb3d::render::calculate_raw_cancellable(&sc, &|_, _| {}, &|| false).map(|(p, g)| (p, g.len())).unwrap();
            let (_, g) = mb3d::render::calculate_raw_cancellable(&sc, &|_, _| {}, &|| false).unwrap();
            let [_, _, w, h] = pc.rect;
            let (w, h) = (w as usize, h as usize);
            let mut a = g.clone();
            let t = Instant::now();
            let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
            if ao.bits15 {
                mb3d::ssao15::ssao15(&mut a, w, h, pc.zc_mul, pc.zcorr, &ao, threads);
            } else {
                mb3d::ssao::ssao24(&mut a, w, h, pc.zc_mul, pc.zcorr, &ao, threads);
            }
            let cpu_s = t.elapsed().as_secs_f64();
            let mut b = g.clone();
            let t = Instant::now();
            let r = if ao.bits15 { mb3d::gpu::ssao15(&mut b, w, h, pc.zc_mul, pc.zcorr, &ao) } else { mb3d::gpu::ssao24(&mut b, w, h, pc.zc_mul, pc.zcorr, &ao) };
            match r {
                Ok(()) => {
                    let gpu_s = t.elapsed().as_secs_f64();
                    let (mut n, mut d, mut bad) = (0usize, 0f64, 0usize);
                    for (x, y) in a.iter().zip(&b) {
                        if x.is_background() {
                            continue;
                        }
                        n += 1;
                        let dd = (x.amb_shadow as f64 - y.amb_shadow as f64).abs() / 16383.0;
                        d += dd;
                        bad += (dd > 0.05) as usize;
                    }
                    let n = n.max(1) as f64;
                    println!("{name}: SSAO{} CPU {cpu_s:.2}s GPU {gpu_s:.2}s; mean diff {:.4}, > 0.05 in {:.2}% (random passes {})", if ao.bits15 { "15" } else { "24" }, d / n, 100.0 * bad as f64 / n, ao.random);
                }
                Err(e) => println!("{name}: SSAO on the GPU failed: {e}"),
            }
        }
        if job.is_empty() {
            println!("{name}: no shadows or DE ambient occlusion");
            continue;
        }
        let t = Instant::now();
        let Some(g) = mb3d::gpu::march(&p, &job, &|_, _| {}, &|| false, |_, _| {}) else {
            println!("{name}: {}", mb3d::gpu::last_status());
            continue;
        };
        let gpu_s = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let [x0, y0, w, h] = p.rect;
        let (w, h) = (w as usize, h as usize);
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let mut c = vec![SiLight::default(); w * h];
        std::thread::scope(|s| {
            for (t, rows) in c.chunks_mut(w * h.div_ceil(threads)).enumerate() {
                let p = &p;
                s.spawn(move || {
                    for (r, row) in rows.chunks_mut(w).enumerate() {
                        let y = y0 + (t * h.div_ceil(threads) + r) as i32;
                        let seed = (0x24563487u32 as i64 + (y as i64 + 1) * 0x324594A1i64) as i32;
                        let mut m = mb3d::calc::Marcher::new(p, seed);
                        for (x, px) in row.iter_mut().enumerate() {
                            *px = m.march_pixel(x0 + x as i32, y);
                        }
                    }
                });
            }
        });
        let march_s = t.elapsed().as_secs_f64();
        let t = Instant::now();
        mb3d::render::post_process(&sc, &p, &mut c, threads);
        let post_s = t.elapsed().as_secs_f64();
        // compare on the pixels both see as object
        let (mut n, mut sh_same, mut soft_d, mut ao_d, mut ao_bad) = (0usize, 0usize, 0f64, 0f64, 0usize);
        let soft = job.shadows.as_ref().is_some_and(|s| s.soft_radius > 0.0);
        for (a, b) in g.iter().zip(&c) {
            if a.is_background() || b.is_background() || a.si_gradient >= 32768 || b.si_gradient >= 32768 {
                continue;
            }
            n += 1;
            if soft {
                soft_d += ((a.shadow >> 10) as f64 - (b.shadow >> 10) as f64).abs() / 63.0;
            } else if a.shadow & 0xFC00 == b.shadow & 0xFC00 {
                sh_same += 1;
            }
            let d = (a.amb_shadow as f64 - b.amb_shadow as f64).abs() / 16383.0;
            ao_d += d;
            ao_bad += (d > 0.1) as usize;
        }
        // POSTCHECK_DIFF=dir: an image of the differences: red where only
        // the CPU shadows, green where only the card does (grey: both),
        // blue for the AO difference
        if let Ok(dir) = std::env::var("POSTCHECK_DIFF") {
            let img: Vec<u8> = g
                .iter()
                .zip(&c)
                .flat_map(|(a, b)| {
                    if a.is_background() || b.is_background() {
                        return [0, 0, 0];
                    }
                    let (sa, sb) = (a.shadow & 0xFC00 != 0, b.shadow & 0xFC00 != 0);
                    let ao = if job.deao.is_some() { ((a.amb_shadow as f64 - b.amb_shadow as f64).abs() / 16383.0 * 2000.0).min(255.0) as u8 } else { 0 };
                    match (sa, sb) {
                        (true, true) => [96, 96, 96u8.max(ao)],
                        (true, false) => [0, 255, ao],
                        (false, true) => [255, 0, ao],
                        _ => [32, 32, 32u8.max(ao)],
                    }
                })
                .collect();
            let _ = std::fs::write(format!("{dir}/{name}.diff.png"), mb3d::png::encode_rgb(w, h, &img));
        }
        let n = n.max(1) as f64;
        let mut s = format!("{name}: GPU {gpu_s:.1}s, CPU march {march_s:.1}s + post {post_s:.1}s;");
        if let Some(sh) = &job.shadows {
            if soft {
                s += &format!(" soft shadow mean diff {:.3};", soft_d / n);
            } else {
                s += &format!(" shadow bits same {:.2}% ({} lights);", 100.0 * sh_same as f64 / n, sh.lights.len());
            }
        }
        if job.deao.is_some() {
            s += &format!(" AO mean diff {:.4}, > 0.1 in {:.2}%", ao_d / n, 100.0 * ao_bad as f64 / n);
        }
        println!("{s}");
    }
}
