//! Nanoseconds per formula iteration of the hybrids of parameter files
//! (one thread), for comparing formula code generators.
//! `cargo run --release --example formbench -- MB3D_DIR FILES...`
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    for f in &a[1..] {
        let (sc, _) = mb3d::animfile::load_scene(std::path::Path::new(f)).unwrap();
        let p = mb3d::calc::CalcParams::new(&sc).unwrap();
        let mut it = p.new_iteration();
        // the fastest of three passes (less noise from other load and clock changes)
        let (mut best, mut its, mut check) = (f64::MAX, 0u64, 0u64);
        for _ in 0..3 {
            its = 0;
            check = 0;
            let t = Instant::now();
            for k in 0..400_000u64 {
                let s = (k % 2000) as f64 / 2000.0;
                for i in 0..3 {
                    it.c[i] = p.ystart[i] + p.vgrads[0][i] * (p.width as f64 * 0.5) + p.vgrads[1][i] * (p.height as f64 * 0.5) + p.vgrads[2][i] * s * 400.0;
                }
                it.calc_sit = false;
                it.mand_function(p.mode, &p.slots);
                its += it.it_result.max(1) as u64;
                check = (check ^ it.rout.to_bits() ^ ((it.it_result as u64) << 40) ^ it.v[0].to_bits().rotate_left(17)).wrapping_mul(0x100000001b3);
            }
            best = best.min(t.elapsed().as_nanos() as f64);
        }
        let ns = best / its as f64;
        println!("{:42} {:6.1} ns/iteration ({} iterations, check {:016x})", std::path::Path::new(f).file_name().unwrap().to_string_lossy(), ns, its, check);
    }
}
