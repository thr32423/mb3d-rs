//! Nanoseconds per formula iteration of the hybrids of parameter files
//! (one thread, through the marcher, so dIFS uses its own loop), for
//! comparing formula code generators.
//! `cargo run --release --example formbench -- MB3D_DIR FILES...`
//! `--check` instead compares the translated formulas with the interpreter
//! on the same points (checksums must be equal).
use std::time::Instant;

fn run(p: &mb3d::calc::CalcParams, points: u64) -> (u64, u64) {
    let mut m = mb3d::calc::Marcher::new(p, 1);
    let (mut its, mut check) = (0u64, 0u64);
    for k in 0..points {
        let s = (k % 2000) as f64 / 2000.0;
        let mut c = [0.0; 3];
        for i in 0..3 {
            c[i] = p.ystart[i] + p.vgrads[0][i] * (p.width as f64 * 0.5) + p.vgrads[1][i] * (p.height as f64 * 0.5) + p.vgrads[2][i] * s * 400.0;
        }
        let n = m.iterations_at(c);
        its += n.max(1) as u64;
        check = (check ^ m.it.rout.to_bits() ^ ((n as u64) << 40) ^ m.it.v[0].to_bits().rotate_left(17)).wrapping_mul(0x100000001b3);
    }
    (its, check)
}

fn main() {
    let mut a: Vec<String> = std::env::args().skip(1).collect();
    let check_mode = a.iter().any(|x| x == "--check");
    a.retain(|x| x != "--check");
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    for f in &a[1..] {
        let (sc, _) = mb3d::animfile::load_scene(std::path::Path::new(f)).unwrap();
        let p = mb3d::calc::CalcParams::new(&sc).unwrap();
        let name = std::path::Path::new(f).file_name().unwrap().to_string_lossy().to_string();
        if check_mode {
            let (_, native) = run(&p, 20_000);
            mb3d::x86::NATIVE_ENABLED.store(false, std::sync::atomic::Ordering::Relaxed);
            let (_, interp) = run(&p, 20_000);
            mb3d::x86::NATIVE_ENABLED.store(true, std::sync::atomic::Ordering::Relaxed);
            println!("{name:42} {}", if native == interp { "same as the interpreter" } else { "DIFFERS from the interpreter" });
            continue;
        }
        // the fastest of three passes (less noise from other load and clock changes)
        let (mut best, mut its, mut check) = (f64::MAX, 0, 0);
        for _ in 0..3 {
            let t = Instant::now();
            (its, check) = run(&p, 400_000);
            best = best.min(t.elapsed().as_nanos() as f64);
        }
        println!("{name:42} {:6.1} ns/iteration ({its} iterations, check {check:016x})", best / its as f64);
    }
}
