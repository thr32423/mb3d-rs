//! The benchmark (not in MB3D): the group posts "render this at 640 x 512
//! and post your Main Calc Time and your rig", and the numbers are never
//! comparable because everyone's parameters, version and settings differ.
//! This renders one fixed scene with the main pass alone, on the processor
//! and on the graphics card when there is one, and writes the result as a
//! line to copy: version, picture, times, processor, threads, card, system.

use crate::scene::Scene;

/// The benchmark scene: the sine bulb of the "Integer Power" preset at
/// 640 x 512, 60 iterations, the main pass only (no shadows, occlusion,
/// reflections or depth of field), the same for everyone.
pub fn scene() -> Scene {
    let mut s = Scene::preset("Integer Power").expect("the built-in preset");
    s.width = 640;
    s.height = 512;
    s.iterations = 120;
    s.de_stop = 0.3;
    s.z_step_div = 0.25;
    s.shadows = None;
    s.ao = None;
    s.deao = None;
    s.dof = None;
    s.vol_light = None;
    s.mc.reflections = false;
    s.first_step_random = false;
    s.threads = 0;
    s
}

/// The results of one run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BenchResult {
    /// the main pass on the processor, seconds
    pub cpu_secs: Option<f64>,
    /// the main pass on the card, seconds (the second run: the first
    /// compiles the shaders)
    pub card_secs: Option<f64>,
    pub cpu_name: String,
    pub threads: usize,
    pub card_name: Option<String>,
    pub os: String,
}

/// The processor's name as the system reports it.
pub fn cpu_name() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(t) = std::fs::read_to_string("/proc/cpuinfo") {
            for l in t.lines() {
                if let Some(v) = l.strip_prefix("model name").and_then(|r| r.split(':').nth(1)) {
                    return v.trim().to_string();
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(o) = std::process::Command::new("sysctl").args(["-n", "machdep.cpu.brand_string"]).output() {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    if let Ok(v) = std::env::var("PROCESSOR_IDENTIFIER") {
        return v;
    }
    "unknown processor".to_string()
}

/// The operating system in a word or two.
pub fn os_name() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(t) = std::fs::read_to_string("/etc/os-release") {
            for l in t.lines() {
                if let Some(v) = l.strip_prefix("PRETTY_NAME=") {
                    return v.trim_matches('"').to_string();
                }
            }
        }
        return "Linux".to_string();
    }
    #[allow(unreachable_code)]
    std::env::consts::OS.to_string()
}

/// Runs the benchmark: the processor, then the card when `card` is set
/// and one is there (the card's first run compiles the shaders, the second
/// counts).  `progress` gets the stage names.
pub fn run(card: bool, progress: &(dyn Fn(&str) + Sync), cancel: &(dyn Fn() -> bool + Sync)) -> Result<BenchResult, String> {
    let sc = scene();
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut r = BenchResult { cpu_name: cpu_name(), threads, card_name: crate::gpu::name(), os: os_name(), ..Default::default() };
    #[cfg(feature = "gpu")]
    let was = crate::gpu::enabled();
    #[cfg(feature = "gpu")]
    crate::gpu::set_enabled(false);
    progress("processor");
    let t = std::time::Instant::now();
    let cpu = crate::render::calculate_raw_cancellable(&sc, &|_, _| {}, cancel);
    #[cfg(feature = "gpu")]
    crate::gpu::set_enabled(was);
    cpu?;
    r.cpu_secs = Some(t.elapsed().as_secs_f64());
    #[cfg(feature = "gpu")]
    if card && r.card_name.is_some() && !cancel() {
        // the card alone (not shared with the processor as in "Auto"), its
        // shaders waited for
        let auto = crate::gpu::auto();
        crate::gpu::set_auto(false);
        crate::gpu::set_enabled(true);
        progress("graphics card (compiling the shaders)");
        let first = crate::render::calculate_raw_cancellable(&sc, &|_, _| {}, cancel);
        if first.is_ok() && !cancel() {
            progress("graphics card");
            let t = std::time::Instant::now();
            let second = crate::render::calculate_raw_cancellable(&sc, &|_, _| {}, cancel);
            if second.is_ok() && crate::gpu::last_status().starts_with("GPU") {
                r.card_secs = Some(t.elapsed().as_secs_f64());
            }
        }
        crate::gpu::set_auto(auto);
        crate::gpu::set_enabled(was);
    }
    let _ = card;
    Ok(r)
}

impl BenchResult {
    /// The result as text to post: one line per fact.
    pub fn text(&self) -> String {
        let mut v = vec![format!("mb3d-rs {} benchmark: Integer Power 640 x 512, 120 iterations, main pass", env!("CARGO_PKG_VERSION"))];
        if let Some(s) = self.cpu_secs {
            v.push(format!("Processor: {:.2} s  ({}, {} threads)", s, self.cpu_name, self.threads));
        }
        match (self.card_secs, &self.card_name) {
            (Some(s), Some(n)) => v.push(format!("Graphics card: {s:.2} s  ({n})")),
            (None, Some(n)) => v.push(format!("Graphics card: not run  ({n})")),
            (_, None) => v.push("Graphics card: none".to_string()),
        }
        v.push(format!("System: {}", self.os));
        v.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scene_is_fixed_and_renders() {
        let s = scene();
        assert_eq!((s.width, s.height, s.iterations), (640, 512, 120));
        assert!(s.shadows.is_none() && s.ao.is_none() && !s.mc.reflections);
        let mut small = s.clone();
        small.width = 64;
        small.height = 48;
        let r = crate::render::render(&small, &|_, _| {}).unwrap();
        assert!(r.coverage() > 0.1);
    }

    #[test]
    fn the_text_names_everything() {
        let r = BenchResult { cpu_secs: Some(1.234), card_secs: None, cpu_name: "Test CPU".into(), threads: 8, card_name: None, os: "Test OS".into() };
        let t = r.text();
        assert!(t.contains("1.23 s") && t.contains("Test CPU, 8 threads") && t.contains("Graphics card: none") && t.contains("Test OS"));
        assert!(!cpu_name().is_empty());
    }
}
