//! Ray marching on the graphics card (prototype, cargo feature `gpu`).
//!
//! The main calculation (`Marcher::march_pixel` for every pixel) runs as a
//! wgpu compute shader (`gpu_march.wgsl`: Vulkan, Metal or DirectX 12) and
//! fills the same G-buffer as the CPU; the post calculations (shadows,
//! ambient occlusion) and the painting stay on the CPU.  The shader
//! calculates in single precision and supports one 'Integer Power' formula
//! (the Mandelbulbs) with the numerical DE; [`unsupported`] says why a scene
//! is calculated on the CPU instead.

use crate::calc::CalcParams;
use crate::formulas::Formula;
use crate::gbuffer::SiLight;
use crate::iteration::HybridMode;
use crate::scene::CameraOptic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static ENABLED: AtomicBool = AtomicBool::new(false);
static LAST: Mutex<String> = Mutex::new(String::new());
/// The last error wgpu reported outside an error scope.
static ERROR: Mutex<Option<String>> = Mutex::new(None);

fn set_error(e: String) {
    if let Ok(mut l) = ERROR.lock() {
        l.get_or_insert(e);
    }
}

fn take_error() -> Option<String> {
    ERROR.lock().ok().and_then(|mut l| l.take())
}

/// Pixels per GPU submission: keeps every submission short (drivers reset
/// the card when one takes seconds) and gives progress steps.
const BAND_PIXELS: usize = 1 << 17;

/// Calculate on the graphics card when the scene allows it (also on with
/// the environment variable `MB3D_GPU=1`).
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed) || std::env::var_os("MB3D_GPU").is_some_and(|v| v != "0")
}

/// How the last calculation ran: "GPU (name)" or "CPU: reason".
pub fn last_status() -> String {
    LAST.lock().map(|s| s.clone()).unwrap_or_default()
}

fn set_status(s: String) {
    if let Ok(mut l) = LAST.lock() {
        *l = s;
    }
}

/// Why the scene cannot be calculated by the shader, `None` if it can.
pub fn unsupported(p: &CalcParams) -> Option<&'static str> {
    if p.slice_2d != 0 {
        return Some("2D calculation");
    }
    if p.slots.len() != 1 || p.end_to != 0 || p.decomb.is_some() {
        return Some("more than one formula");
    }
    let s = &p.slots[0];
    if !matches!(s.formula, Formula::IntPow { .. }) || s.iterations <= 0 || s.uncounted {
        return Some("only the Integer Power formula is ported");
    }
    if p.mode != HybridMode::Alt3D || p.is_custom_de || p.difs || p.machine.is_some() {
        return Some("formula mode");
    }
    if p.cut_options != 0 {
        return Some("cutting planes");
    }
    if p.inside_rendering || p.in_and_outside {
        return Some("inside rendering");
    }
    if p.color_on_it != 0 {
        return Some("colour on iteration");
    }
    if p.vol.is_some() {
        return Some("volumetric light");
    }
    if precision(p) < MIN_PRECISION {
        return Some("the zoom needs double precision");
    }
    None
}

/// The smallest [`precision`] for the GPU: below it the single precision
/// noise shows (in the example Mandelbulb about 2.5% of the pixels differ
/// clearly from the CPU at 3, 4.5% at 1, noise everywhere below 0.1).
const MIN_PRECISION: f64 = 3.0;

/// How many single precision steps (at the size of the coordinates) the
/// offset of the numerical DE gradient spans: the larger, the less the
/// rounding of the shader shows.  It falls with the zoom (the pixel size).
pub fn precision(p: &CalcParams) -> f64 {
    let m = p.ystart.iter().fold(0.0f64, |a, v| a.max(v.abs()))
        + p.step_width * (p.width + p.height) as f64
        + 2.0;
    p.de_offset as f64 / (m * f32::EPSILON as f64)
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    name: String,
}

fn gpu() -> Result<&'static Gpu, String> {
    static G: OnceLock<Result<Gpu, String>> = OnceLock::new();
    G.get_or_init(|| pollster::block_on(init())).as_ref().map_err(|e| e.clone())
}

async fn init() -> Result<Gpu, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .map_err(|e| format!("no graphics card found: {e}"))?;
    let info = adapter.get_info();
    if info.device_type == wgpu::DeviceType::Cpu {
        // llvmpipe, WARP: slower than the CPU renderer
        return Err(format!("only a software renderer ({})", info.name));
    }
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("mb3d"),
            required_limits: adapter.limits(),
            ..Default::default()
        })
        .await
        .map_err(|e| format!("{}: {e}", info.name))?;
    // errors (a shader the driver cannot compile, a lost device) must not
    // end the program: they are kept and the calculation goes to the CPU
    device.on_uncaptured_error(std::sync::Arc::new(|e: wgpu::Error| set_error(e.to_string())));
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("march"),
        source: wgpu::ShaderSource::Wgsl(include_str!("gpu_march.wgsl").into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("march"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    if let Some(e) = scope.pop().await {
        return Err(format!("{}: the shader does not compile: {e}", info.name));
    }
    Ok(Gpu { device, queue, pipeline, name: format!("{} ({:?})", info.name, info.backend) })
}

/// The parameters in the order of the indices in `gpu_march.wgsl`.
fn params(p: &CalcParams, row0: usize, rows: usize) -> Vec<u32> {
    let (power, z_mul) = match p.slots[0].formula {
        Formula::IntPow { power, z_mul } => (power, z_mul),
        _ => (8, -1.0),
    };
    let optic = match p.optic {
        CameraOptic::Common => 0,
        CameraOptic::Planar => 1,
        CameraOptic::Panorama => 2,
    };
    let mut v: Vec<u32> = Vec::with_capacity(64);
    let i = |v: &mut Vec<u32>, x: i32| v.push(x as u32);
    let f = |v: &mut Vec<u32>, x: f64| v.push((x as f32).to_bits());
    i(&mut v, p.rect[0]);
    i(&mut v, p.rect[1]);
    i(&mut v, p.rect[2]);
    i(&mut v, row0 as i32);
    i(&mut v, rows as i32);
    i(&mut v, p.height);
    i(&mut v, optic);
    f(&mut v, p.fov_y);
    f(&mut v, p.fovx_off as f64);
    f(&mut v, p.fovx_mul as f64);
    f(&mut v, p.pl_optic_z as f64);
    for r in &p.vgrads {
        for &x in r {
            f(&mut v, x);
        }
    }
    for &x in &p.ystart {
        f(&mut v, x);
    }
    i(&mut v, p.max_it);
    i(&mut v, p.min_it);
    f(&mut v, p.d_rstop);
    f(&mut v, p.rstop3d);
    i(&mut v, p.do_julia as i32);
    for &x in &p.ju[..3] {
        f(&mut v, x);
    }
    i(&mut v, power);
    f(&mut v, z_mul);
    f(&mut v, p.de_stop as f64);
    f(&mut v, p.de_stop_factor as f64);
    f(&mut v, p.z_step_div as f64);
    f(&mut v, p.ms_de_sub as f64);
    f(&mut v, p.mh04zsd as f64);
    i(&mut v, p.dfog_on_it as i32);
    i(&mut v, p.first_step_random as i32);
    i(&mut v, p.de_add_steps);
    i(&mut v, p.normals_on_de as i32);
    i(&mut v, p.sm_normals);
    f(&mut v, p.de_offset as f64);
    f(&mut v, p.de_offset006 as f64);
    f(&mut v, p.d_de_scale as f64);
    f(&mut v, p.zcorr);
    f(&mut v, p.zc_mul);
    f(&mut v, p.zend);
    f(&mut v, p.col_var_de_stop_mul as f64);
    f(&mut v, p.d_col_plus as f64);
    f(&mut v, p.mcts_m as f64);
    f(&mut v, p.step_width);
    i(&mut v, p.color_option as i32);
    f(&mut v, p.mct_color_mul as f64);
    f(&mut v, p.ln_rstop as f64);
    f(&mut v, p.fhln[0] as f64);
    debug_assert_eq!(v.len(), 57);
    v
}

fn bytes(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// Calculates the G-buffer of `p.rect` on the graphics card.  Returns
/// `None` when the scene or the computer cannot use it (the reason is in
/// [`last_status`]); the caller then calculates on the CPU.
pub fn march(
    p: &CalcParams,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    mut sink: impl FnMut(usize, &[SiLight]),
) -> Option<Vec<SiLight>> {
    if let Some(r) = unsupported(p) {
        set_status(format!("CPU: {r} (not on the GPU yet)"));
        return None;
    }
    let g = match gpu() {
        Ok(g) => g,
        Err(e) => {
            set_status(format!("CPU: {e}"));
            return None;
        }
    };
    match run(g, p, progress, cancel, &mut sink) {
        Ok(v) => {
            set_status(format!("GPU {}", g.name));
            Some(v)
        }
        Err(e) => {
            set_status(format!("CPU: GPU error: {e}"));
            None
        }
    }
}

fn run(
    g: &Gpu,
    p: &CalcParams,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    sink: &mut dyn FnMut(usize, &[SiLight]),
) -> Result<Vec<SiLight>, String> {
    let (w, h) = (p.rect[2].max(0) as usize, p.rect[3].max(0) as usize);
    let band = (BAND_PIXELS / w.max(1)).clamp(8, h.max(8));
    let out_size = (w * band * 16) as u64;
    let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
    let out = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("gbuffer"), size: out_size.max(16), usage, mapped_at_creation: false });
    let read = g.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("read"),
        size: out_size.max(16),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let par = g.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("params"),
        size: 64 * 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind = g.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &g.pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: par.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: out.as_entire_binding() },
        ],
    });
    let mut gbuf = Vec::with_capacity(w * h);
    let mut row0 = 0;
    while row0 < h {
        if cancel() {
            return Err("cancelled".into());
        }
        let rows = band.min(h - row0);
        g.queue.write_buffer(&par, 0, &bytes(&params(p, row0, rows)));
        let mut enc = g.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&g.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(w.div_ceil(8) as u32, rows.div_ceil(8) as u32, 1);
        }
        let n = (w * rows * 16) as u64;
        enc.copy_buffer_to_buffer(&out, 0, &read, 0, n);
        g.queue.submit([enc.finish()]);
        let slice = read.slice(..n);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        g.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
        if let Some(e) = take_error() {
            return Err(e);
        }
        rx.recv().map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
        {
            let data = slice.get_mapped_range().map_err(|e| e.to_string())?;
            for (r, row) in data.chunks_exact(w * 16).enumerate() {
                let start = gbuf.len();
                for px in row.chunks_exact(16) {
                    let u = |k: usize| u32::from_le_bytes(px[k * 4..k * 4 + 4].try_into().unwrap());
                    let (a, b, z, s) = (u(0), u(1), u(2), u(3));
                    gbuf.push(SiLight {
                        normal: [a as u16 as i16, (a >> 16) as u16 as i16, b as u16 as i16],
                        zpos_fine: z,
                        shadow: (b >> 16) as u16,
                        amb_shadow: 5000,
                        si_gradient: s as u16,
                        otrap: (s >> 16) as u16,
                    });
                }
                sink(row0 + r, &gbuf[start..]);
            }
        }
        read.unmap();
        row0 += rows;
        progress(row0, h);
    }
    Ok(gbuf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::Marcher;

    /// The GPU G-buffer against the CPU's for the example Mandelbulb (skipped
    /// when the computer has no usable graphics card).
    #[test]
    fn mandelbulb_matches_cpu() {
        let mut sc = crate::scene::Scene::parse(include_str!("../examples/mandelbulb.m3s")).unwrap();
        sc.scale_image(0.5);
        let p = CalcParams::new(&sc).unwrap();
        assert_eq!(unsupported(&p), None);
        if let Err(e) = gpu() {
            eprintln!("no GPU, test skipped: {e}");
            return;
        }
        let t = std::time::Instant::now();
        let g = march(&p, &|_, _| {}, &|| false, |_, _| {}).expect(&last_status());
        let gpu_s = t.elapsed().as_secs_f64();
        let [x0, y0, w, h] = p.rect;
        let t = std::time::Instant::now();
        let mut cpu = Vec::new();
        for y in y0..y0 + h {
            let seed = (0x24563487u32 as i64 + (y as i64 + 1) * 0x324594A1i64) as i32;
            let mut m = Marcher::new(&p, seed);
            cpu.extend((x0..x0 + w).map(|x| m.march_pixel(x, y)));
        }
        let cpu_s = t.elapsed().as_secs_f64();
        assert_eq!(g.len(), cpu.len());
        let (mut same_hit, mut both, mut zdiff, mut ndot, mut bad_n) = (0usize, 0usize, 0f64, 0f64, 0usize);
        for (a, b) in g.iter().zip(&cpu) {
            if a.is_background() == b.is_background() {
                same_hit += 1;
            }
            if !a.is_background() && !b.is_background() {
                both += 1;
                zdiff += (a.zpos() as f64 - b.zpos() as f64).abs();
                let na = a.normal.map(|v| v as f64 / 32767.0);
                let nb = b.normal.map(|v| v as f64 / 32767.0);
                let d = na[0] * nb[0] + na[1] * nb[1] + na[2] * nb[2];
                ndot += d;
                if d < 0.9 {
                    bad_n += 1;
                }
            }
        }
        let n = g.len() as f64;
        let same = same_hit as f64 / n;
        let zd = zdiff / both.max(1) as f64;
        let nd = ndot / both.max(1) as f64;
        let bad = bad_n as f64 / both.max(1) as f64;
        eprintln!(
            "{}: GPU {gpu_s:.3}s (one thread on the CPU {cpu_s:.3}s); same background/object {:.2}%, \
             mean |zpos difference| {zd:.1} of 32768, mean normal cosine {nd:.4}, normals off by more than 25 degrees {:.2}%",
            last_status(),
            same * 100.0,
            bad * 100.0
        );
        assert!(same > 0.99, "background/object differ in {:.2}% of the pixels", (1.0 - same) * 100.0);
        assert!(zd < 20.0, "depth differs by {zd:.1} on average");
        assert!(nd > 0.97 && bad < 0.03, "normals differ: mean cosine {nd:.4}, {:.2}% off", bad * 100.0);
    }
}
