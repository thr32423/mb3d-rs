// times the GPU march against the CPU threads: gpubench scene [scale]
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut sc = mb3d::scene::Scene::parse(&std::fs::read_to_string(&a[0]).unwrap()).unwrap();
    if let Some(s) = a.get(1) {
        sc.scale_image(s.parse().unwrap());
    }
    for gpu in [true, true, true, false] {
        mb3d::gpu::set_enabled(gpu);
        let t = std::time::Instant::now();
        mb3d::render::calculate_raw_cancellable(&sc, &|_, _| {}, &|| false).unwrap();
        println!("{} {}x{}: {:.3}s  {}", if gpu { "gpu" } else { "cpu" }, sc.width, sc.height, t.elapsed().as_secs_f64(), if gpu { mb3d::gpu::last_status() } else { String::new() });
    }
}
