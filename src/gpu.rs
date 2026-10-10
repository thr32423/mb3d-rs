//! Ray marching on the graphics card (prototype, cargo feature `gpu`).
//!
//! The main calculation (`Marcher::march_pixel` for every pixel) runs as a
//! wgpu compute shader (`gpu_march.wgsl`: Vulkan, Metal or DirectX 12) and
//! fills the same G-buffer as the CPU, then runs the hard shadows and the
//! DE ambient occlusion on it (the painting stays on the CPU).  The shader
//! calculates in single precision: the alternating 3D hybrid of 'Integer
//! Power' and the .m3f formulas the lifter translates to WGSL
//! (`x86::lift_wgsl`), with the numerical or the analytic DE (options 0,
//! 2, 11); [`unsupported`] says why a scene is calculated on the CPU.

use crate::calc::{CalcParams, PostJob};
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

/// Calculate on the graphics card when the scene allows it.  Off in the
/// library; the programs switch it on (see [`default_on`]).
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// The programs' default: on, unless the environment variable `MB3D_GPU`
/// is 0.
pub fn default_on() -> bool {
    std::env::var_os("MB3D_GPU").is_none_or(|v| v != "0")
}

/// How the last calculation ran: "GPU (name)" or "CPU: reason".
pub fn last_status() -> String {
    LAST.lock().map(|s| s.clone()).unwrap_or_default()
}

pub fn set_status(s: String) {
    if let Ok(mut l) = LAST.lock() {
        *l = s;
    }
}

/// Why the scene cannot be calculated by the shader, `None` if it can.
pub fn unsupported(p: &CalcParams) -> Option<String> {
    let r = |s: &str| Some(s.to_string());
    if p.slice_2d != 0 {
        return r("2D calculation (not on the GPU yet)");
    }
    if p.decomb.is_some() {
        return r("DE combination (not on the GPU yet)");
    }
    if p.mode != HybridMode::Alt3D {
        return r("4D or interpolation hybrid (not on the GPU yet)");
    }
    if p.difs {
        return r("dIFS formulas (not on the GPU yet)");
    }
    if p.is_custom_de && ![2, 11].contains(&p.de_option) {
        return r("this analytic DE option (not on the GPU yet)");
    }
    if p.cut_options != 0 {
        return r("cutting planes (not on the GPU yet)");
    }
    if p.inside_rendering || p.in_and_outside {
        return r("inside rendering (not on the GPU yet)");
    }
    if p.color_on_it != 0 {
        return r("colour on iteration (not on the GPU yet)");
    }
    if p.vol.is_some() {
        return r("volumetric light (not on the GPU yet)");
    }
    if precision(p) < MIN_PRECISION {
        return r("the zoom needs double precision");
    }
    scene_shader(p).err()
}

/// The parts of the shader and its buffers for one scene.
struct SceneShader {
    /// the complete WGSL
    code: String,
    /// the constants of all .m3f slots (u32)
    cst: Vec<u32>,
    /// the record image: for cell c the f32 (whole doubles) at 2c, or the
    /// two u32 halves at 2c, 2c + 1
    rtpl: Vec<u32>,
    /// per slot: iterations, uncounted, constants base, fHln bits, power,
    /// z multiplier bits, kind, 0
    slots: [[u32; 8]; 6],
}

/// Record cells (offset / 8 from J4) with integers the loop copies to the
/// record before a .m3f call (`Iteration::run_custom`): they must be halves.
const LOOP_HALVES: [u32; 4] = [13, 15, 16, 33];

fn scene_shader(p: &CalcParams) -> Result<SceneShader, String> {
    use crate::x86::lift_wgsl::{record_globals, CellTy, WgslFormula, PRELUDE};
    use std::fmt::Write;
    let mut slots = [[0u32; 8]; 6];
    let mut wfs: Vec<(usize, WgslFormula)> = Vec::new();
    let mut cst: Vec<u32> = Vec::new();
    if p.slots.len() > 6 {
        return Err("more than 6 formulas".into());
    }
    for (n, sl) in p.slots.iter().enumerate() {
        let row = &mut slots[n];
        row[0] = sl.iterations as u32;
        row[1] = sl.uncounted as u32;
        row[3] = p.fhln[n].to_bits();
        if sl.iterations == 0 {
            continue;
        }
        match &sl.formula {
            Formula::IntPow { power, z_mul } => {
                row[4] = *power as u32;
                row[5] = (*z_mul as f32).to_bits();
                row[6] = 1;
            }
            Formula::AmazingBox { scale, min_r, fold } => {
                row[2] = sl.ade as u32;
                row[4] = (*scale as f32).to_bits();
                row[5] = (*min_r as f32).to_bits();
                row[6] = 3;
                row[7] = (*fold as f32).to_bits();
            }
            Formula::Custom(c) if c.def.jit.is_none() => {
                let m = p.machine.as_ref().ok_or("no formula machine")?;
                let prog = m.prog(crate::custom::code_addr(n)).ok_or_else(|| format!("{}: not compiled", c.name()))?;
                let wf = prog.emit_wgsl(&format!("fm{n}"), false).ok_or_else(|| format!("{} is not on the GPU yet", c.name()))?;
                if wf.writes_vars {
                    return Err(format!("{} keeps state in its variables", c.name()));
                }
                let vb = (crate::custom::var_buf_addr(n) - crate::x86::BASE) as usize;
                row[2] = cst.len() as u32;
                cst.extend(wf.constants_from(&m.mem[vb..vb + 0x400]));
                row[6] = 2;
                wfs.push((n, wf));
            }
            f => return Err(format!("the built-in formula {} is not on the GPU yet", f.name())),
        }
    }
    // one cell type per record cell for the whole shader: the bits form
    // (halves) wins; formulas that typed a cell differently are made again
    let mut unified: std::collections::BTreeMap<u32, CellTy> = Default::default();
    for (_, w) in &wfs {
        for (c, t) in &w.record {
            let e = unified.entry(*c).or_insert(*t);
            if *t == CellTy::Halves {
                *e = CellTy::Halves;
            }
        }
    }
    for (n, w) in wfs.iter_mut() {
        if w.record.iter().any(|(c, t)| unified[c] != *t) {
            let prog = p.machine.as_ref().and_then(|m| m.prog(crate::custom::code_addr(*n))).ok_or("not compiled")?;
            *w = prog.emit_wgsl_with(&format!("fm{n}"), false, &unified).ok_or("formula typing")?;
        }
    }
    let refs: Vec<&WgslFormula> = wfs.iter().map(|(_, w)| w).collect();
    let globals = record_globals(&refs).ok_or("formulas use a record field in different ways")?;
    // the cell types of the shader
    let mut ty: std::collections::BTreeMap<u32, CellTy> = Default::default();
    for w in &refs {
        for (c, t) in &w.record {
            ty.insert(*c, *t);
        }
    }
    for (c, t) in &ty {
        if LOOP_HALVES.contains(c) && *t != CellTy::Halves {
            return Err("a formula uses an integer of the record as a double".into());
        }
    }
    // the record image from the CPU's prepared iteration (static parts)
    let it = p.new_iteration();
    let mut rtpl = vec![0u32; 128];
    if let Some(m) = it.emu.as_deref() {
        let b = (crate::custom::IT_BASE - crate::x86::BASE) as usize;
        for (c, t) in &ty {
            let o = b + 8 * *c as usize;
            let lo = u32::from_le_bytes(m.mem[o..o + 4].try_into().unwrap());
            let hi = u32::from_le_bytes(m.mem[o + 4..o + 8].try_into().unwrap());
            match t {
                CellTy::Whole => rtpl[2 * *c as usize] = (f64::from_bits(lo as u64 | (hi as u64) << 32) as f32).to_bits(),
                CellTy::Halves => {
                    rtpl[2 * *c as usize] = lo;
                    rtpl[2 * *c as usize + 1] = hi;
                }
            }
        }
    }
    let mut g = String::new();
    g.push_str(PRELUDE);
    g.push_str(&globals);
    // init_record, marshal_in, marshal_out
    let has = |c: u32| ty.contains_key(&c);
    let _ = writeln!(g, "fn init_record() {{");
    for (c, t) in &ty {
        match t {
            CellTy::Whole => {
                let _ = writeln!(g, "    r{c}f = bitcast<f32>(rtpl[{}u]);", 2 * c);
            }
            CellTy::Halves => {
                let _ = writeln!(g, "    r{c}l = rtpl[{}u]; r{c}h = rtpl[{}u];", 2 * c, 2 * c + 1);
            }
        }
    }
    let _ = writeln!(g, "}}");
    let ins: [(u32, &str); 20] = [
        (3, "r3f = v.x;"), (4, "r4f = v.y;"), (5, "r5f = v.z;"), (6, "r6f = w;"),
        (7, "r7f = c.x;"), (8, "r8f = c.y;"), (9, "r9f = c.z;"),
        (10, "r10f = j.x;"), (11, "r11f = j.y;"), (12, "r12f = j.z;"), (0, "r0f = j4;"),
        (1, "r1f = rold;"), (14, "r14f = rout;"), (31, "r31f = otrap;"), (32, "r32f = vary_scale;"),
        (34, "r34f = dfree.x;"), (35, "r35f = dfree.y;"), (36, "r36f = deriv.x;"), (37, "r37f = deriv.y;"), (38, "r38f = deriv.z;"),
    ];
    // a double cell in the bits form gets / gives the bits of the value
    let bits_in = |l: &str| -> String {
        let (lhs, rhs) = l.trim_end_matches(';').split_once(" = ").unwrap();
        let cell = &lhs[..lhs.len() - 1];
        format!("{cell}l = f64_lo({rhs}); {cell}h = f64_hi({rhs});")
    };
    let bits_out = |l: &str| -> String {
        let (lhs, rhs) = l.trim_end_matches(';').split_once(" = ").unwrap();
        let cell = &rhs[..rhs.len() - 1];
        format!("{lhs} = from_f64_bits({cell}h, {cell}l);")
    };
    let _ = writeln!(g, "fn marshal_in() {{");
    for (c, l) in ins {
        match ty.get(&c) {
            Some(CellTy::Whole) => {
                let _ = writeln!(g, "    {l}");
            }
            Some(CellTy::Halves) => {
                let _ = writeln!(g, "    {}", bits_in(l));
            }
            None => {}
        }
    }
    if has(15) {
        let _ = writeln!(g, "    r15l = bitcast<u32>(it_result); r15h = bitcast<u32>(max_it);");
    }
    if has(16) {
        let _ = writeln!(g, "    r16l = bitcast<u32>(rstop);");
    }
    if has(33) {
        let _ = writeln!(g, "    r33l = bitcast<u32>(first_it);");
    }
    let _ = writeln!(g, "}}");
    let outs: [(u32, &str); 16] = [
        (3, "v.x = r3f;"), (4, "v.y = r4f;"), (5, "v.z = r5f;"), (6, "w = r6f;"),
        (10, "j.x = r10f;"), (11, "j.y = r11f;"), (12, "j.z = r12f;"), (0, "j4 = r0f;"),
        (14, "rout = r14f;"), (31, "otrap = r31f;"), (32, "vary_scale = r32f;"),
        (34, "dfree.x = r34f;"), (35, "dfree.y = r35f;"), (36, "deriv.x = r36f;"), (37, "deriv.y = r37f;"), (38, "deriv.z = r38f;"),
    ];
    let _ = writeln!(g, "fn marshal_out() {{");
    for (c, l) in outs {
        match ty.get(&c) {
            Some(CellTy::Whole) => {
                let _ = writeln!(g, "    {l}");
            }
            Some(CellTy::Halves) => {
                let _ = writeln!(g, "    {}", bits_out(l));
            }
            None => {}
        }
    }
    if has(33) {
        let _ = writeln!(g, "    first_it = bitcast<i32>(r33l);");
    }
    let _ = writeln!(g, "}}");
    for (_, w) in &wfs {
        g.push_str(&w.code);
    }
    // call_slot
    let x = crate::custom::IT_C1 - 32;
    let regs = format!(
        "rg = array<u32, 8>({:#x}u, {:#x}u, {:#x}u, 0u, {:#x}u, 0u, 0u, 0u);",
        x,
        x + 16,
        x + 8,
        crate::custom::STACK_TOP - 12
    );
    let _ = writeln!(g, "fn call_slot(n: u32) {{");
    let _ = writeln!(g, "    switch (n) {{");
    for (n, row) in slots.iter().enumerate() {
        match row[6] {
            1 => {
                let _ = writeln!(g, "        case {n}u: {{ v = int_pow(v, j, bitcast<i32>(slot({n}u, 4u)), bitcast<f32>(slot({n}u, 5u))); }}");
            }
            2 => {
                let _ = writeln!(g, "        case {n}u: {{ marshal_in(); {regs} fsw = 0u; fm{n}(slot({n}u, 2u)); marshal_out(); }}");
            }
            3 => {
                let _ = writeln!(g, "        case {n}u: {{ amazing_box({n}u); }}");
            }
            _ => {}
        }
    }
    let _ = writeln!(g, "        default: {{}}");
    let _ = writeln!(g, "    }}");
    let _ = writeln!(g, "}}");
    let code = include_str!("gpu_march.wgsl").replace("//@FORMULAS@", &g);
    // MB3D_GPU_DUMP=file: write the scene shader (for debugging)
    if let Some(f) = std::env::var_os("MB3D_GPU_DUMP") {
        let _ = std::fs::write(f, &code);
    }
    Ok(SceneShader { code, cst, rtpl, slots })
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
    /// compiled scene shaders by a hash of their code
    pipelines: Mutex<std::collections::HashMap<u64, std::sync::Arc<wgpu::ComputePipeline>>>,
    name: String,
}

/// The pipeline of the shader `code` (compiled once, then cached).
fn pipeline(g: &Gpu, code: &str) -> Result<std::sync::Arc<wgpu::ComputePipeline>, String> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    code.hash(&mut h);
    let key = h.finish();
    if let Some(pl) = g.pipelines.lock().map_err(|e| e.to_string())?.get(&key) {
        return Ok(pl.clone());
    }
    let scope = g.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module = g.device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("march"), source: wgpu::ShaderSource::Wgsl(code.into()) });
    let pl = g.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("march"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    if let Some(e) = pollster::block_on(scope.pop()) {
        return Err(format!("the shader does not compile: {e}"));
    }
    let pl = std::sync::Arc::new(pl);
    g.pipelines.lock().map_err(|e| e.to_string())?.insert(key, pl.clone());
    Ok(pl)
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
    Ok(Gpu { device, queue, pipelines: Mutex::new(Default::default()), name: format!("{} ({:?})", info.name, info.backend) })
}

/// The parameters in the order of the indices in `gpu_march.wgsl`.
/// The words of the parameter buffer (`P` in the shader) for a band of
/// rows; `mode` 0 is the march, 1 the shadow pass and 2 the ambient
/// occlusion of `post`.
fn params(p: &CalcParams, row0: usize, rows: usize, slots: &[[u32; 8]; 6], mode: u32, post: &PostJob) -> Vec<u32> {
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
    // the post passes use their own binary search depth
    i(&mut v, [p.de_add_steps, 8, 5][mode as usize]);
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
    i(&mut v, p.is_custom_de as i32);
    i(&mut v, p.de_option);
    i(&mut v, p.end_to as i32);
    i(&mut v, p.repeat_from as i32);
    f(&mut v, p.ju[3]);
    while v.len() < 64 {
        v.push(0);
    }
    for row in slots {
        v.extend_from_slice(row);
    }
    debug_assert_eq!(v.len(), 112);
    i(&mut v, mode as i32);
    let sh = post.shadows.as_ref();
    i(&mut v, sh.map(|s| s.lights.len()).unwrap_or(0) as i32);
    f(&mut v, sh.map(|s| s.max_len_mul).unwrap_or(1.0) as f64);
    f(&mut v, sh.map(|s| s.soft_radius).unwrap_or(0.0) as f64);
    let ao = post.deao.as_ref();
    i(&mut v, ao.map(|d| d.quality).unwrap_or(0) as i32);
    i(&mut v, ao.map(|d| d.dither).unwrap_or(0) as i32);
    f(&mut v, ao.map(|d| d.max_len).unwrap_or(1.0) as f64);
    i(&mut v, ao.map(|d| d.first_step_random).unwrap_or(false) as i32);
    i(&mut v, p.width);
    while v.len() < 124 {
        v.push(0);
    }
    for k in 0..6 {
        match sh.and_then(|s| s.lights.get(k)) {
            Some(l) => {
                i(&mut v, l.idx as i32);
                for &x in &l.vec {
                    f(&mut v, x);
                }
            }
            None => v.extend_from_slice(&[0; 4]),
        }
    }
    debug_assert_eq!(v.len(), PARAM_WORDS);
    v
}

/// The size of the parameter buffer.
const PARAM_WORDS: usize = 148;

fn bytes(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// Calculates the G-buffer of `p.rect` on the graphics card, with the
/// post calculations of `post` (shadows, ambient occlusion).  Returns
/// `None` when the scene or the computer cannot use it (the reason is in
/// [`last_status`]); the caller then calculates on the CPU.
pub fn march(
    p: &CalcParams,
    post: &PostJob,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    mut sink: impl FnMut(usize, &[SiLight]),
) -> Option<Vec<SiLight>> {
    if let Some(r) = unsupported(p) {
        set_status(format!("CPU: {r}"));
        return None;
    }
    let g = match gpu() {
        Ok(g) => g,
        Err(e) => {
            set_status(format!("CPU: {e}"));
            return None;
        }
    };
    let sh = match scene_shader(p) {
        Ok(s) => s,
        Err(e) => {
            set_status(format!("CPU: {e}"));
            return None;
        }
    };
    match run(g, p, &sh, post, progress, cancel, &mut sink) {
        Ok(v) => {
            let mut s = format!("GPU {}", g.name);
            if post.shadows.is_some() {
                s.push_str(" +shadows");
            }
            if post.deao.is_some() {
                s.push_str(" +AO");
            }
            set_status(s);
            Some(v)
        }
        Err(e) => {
            set_status(format!("CPU: GPU error: {e}"));
            None
        }
    }
}

/// The passes of a band: the march and the post passes of `post`.
fn modes(post: &PostJob) -> Vec<u32> {
    let mut m = vec![0];
    if post.shadows.is_some() {
        m.push(1);
    }
    if post.deao.is_some() {
        m.push(2);
    }
    m
}

#[allow(clippy::too_many_arguments)]
fn run(
    g: &Gpu,
    p: &CalcParams,
    sh: &SceneShader,
    post: &PostJob,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    sink: &mut dyn FnMut(usize, &[SiLight]),
) -> Result<Vec<SiLight>, String> {
    let (w, h) = (p.rect[2].max(0) as usize, p.rect[3].max(0) as usize);
    // the post passes march several rays per pixel: smaller bands
    let cost = 1
        + post.shadows.as_ref().map(|s| s.lights.len()).unwrap_or(0)
        + post.deao.map(|d| [3, 7, 17, 33][d.quality.min(3) as usize] / 2).unwrap_or(0);
    let band = (BAND_PIXELS / cost / w.max(1)).clamp(8, h.max(8));
    let out_size = (w * band * 20) as u64;
    let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
    let out = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("gbuffer"), size: out_size.max(16), usage, mapped_at_creation: false });
    let read = g.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("read"),
        size: out_size.max(16),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let pipeline = pipeline(g, &sh.code)?;
    // one parameter buffer per pass (a buffer write before a submission
    // takes effect before all of its passes)
    let modes = modes(post);
    let pars: Vec<wgpu::Buffer> = modes
        .iter()
        .map(|_| {
            g.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("params"),
                size: (PARAM_WORDS * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        })
        .collect();
    let words = |label: &'static str, d: &[u32]| {
        let d = if d.is_empty() { vec![0u32] } else { d.to_vec() };
        let b = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (d.len() * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        g.queue.write_buffer(&b, 0, &bytes(&d));
        b
    };
    let cst = words("constants", &sh.cst);
    let rtpl = words("record", &sh.rtpl);
    let binds: Vec<wgpu::BindGroup> = pars
        .iter()
        .map(|par| {
            g.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: par.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: out.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: cst.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: rtpl.as_entire_binding() },
                ],
            })
        })
        .collect();
    let mut gbuf = Vec::with_capacity(w * h);
    let mut row0 = 0;
    while row0 < h {
        if cancel() {
            return Err("cancelled".into());
        }
        let rows = band.min(h - row0);
        for (k, &mode) in modes.iter().enumerate() {
            g.queue.write_buffer(&pars[k], 0, &bytes(&params(p, row0, rows, &sh.slots, mode, post)));
        }
        let mut enc = g.device.create_command_encoder(&Default::default());
        for bind in &binds {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, bind, &[]);
            pass.dispatch_workgroups(w.div_ceil(8) as u32, rows.div_ceil(8) as u32, 1);
        }
        let n = (w * rows * 20) as u64;
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
            for (r, row) in data.chunks_exact(w * 20).enumerate() {
                let start = gbuf.len();
                for px in row.chunks_exact(20) {
                    let u = |k: usize| u32::from_le_bytes(px[k * 4..k * 4 + 4].try_into().unwrap());
                    let (a, b, z, s, amb) = (u(0), u(1), u(2), u(3), u(4));
                    gbuf.push(SiLight {
                        normal: [a as u16 as i16, (a >> 16) as u16 as i16, b as u16 as i16],
                        zpos_fine: z,
                        shadow: (b >> 16) as u16,
                        amb_shadow: amb as u16,
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

/// Runs `wgsl` (entry point `main`) once with the storage buffers
/// `inputs` (bindings 0.., read only) and an output buffer of `out_len`
/// words (the next binding); `groups` workgroups.  For tests of generated
/// shader code.
pub fn run_compute(wgsl: &str, inputs: &[&[u32]], out_len: usize, groups: u32) -> Result<Vec<u32>, String> {
    let g = gpu()?;
    let scope = g.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module = g.device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("test"), source: wgpu::ShaderSource::Wgsl(wgsl.into()) });
    let pipeline = g.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("test"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    if let Some(e) = pollster::block_on(scope.pop()) {
        return Err(e.to_string());
    }
    let mut bufs = Vec::new();
    for d in inputs {
        let words = if d.is_empty() { vec![0u32] } else { d.to_vec() };
        let b = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (words.len() * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        g.queue.write_buffer(&b, 0, &bytes(&words));
        bufs.push(b);
    }
    let size = (out_len.max(1) * 4) as u64;
    let out = g.device.create_buffer(&wgpu::BufferDescriptor { label: None, size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
    let read = g.device.create_buffer(&wgpu::BufferDescriptor { label: None, size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let mut entries: Vec<wgpu::BindGroupEntry> = bufs.iter().enumerate().map(|(i, b)| wgpu::BindGroupEntry { binding: i as u32, resource: b.as_entire_binding() }).collect();
    entries.push(wgpu::BindGroupEntry { binding: bufs.len() as u32, resource: out.as_entire_binding() });
    let bind = g.device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &pipeline.get_bind_group_layout(0), entries: &entries });
    let mut enc = g.device.create_command_encoder(&Default::default());
    {
        let mut pass = enc.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(groups, 1, 1);
    }
    enc.copy_buffer_to_buffer(&out, 0, &read, 0, size);
    g.queue.submit([enc.finish()]);
    let slice = read.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    g.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    if let Some(e) = take_error() {
        return Err(e);
    }
    let data = slice.get_mapped_range().map_err(|e| e.to_string())?;
    let v = data.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    drop(data);
    read.unmap();
    Ok(v)
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
        let g = march(&p, &PostJob::default(), &|_, _| {}, &|| false, |_, _| {}).expect(&last_status());
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
