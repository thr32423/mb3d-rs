// The ray marcher of calc.rs (`Marcher::march_pixel`, MB3D's
// TMandCalcThread.Execute) for the graphics card, in single precision.
// The alternating 3D hybrid of 'Integer Power' and .m3f formulas (lifted
// to WGSL, x86/lift_wgsl.rs; gpu.rs inserts them at the FORMULAS marker
// marker), numerical or analytic DE; no cutting planes, inside rendering
// or volumetric light (gpu.rs checks).
// The structure follows calc.rs closely so the two can be compared line by
// line; the iteration state is kept in private variables like the CPU's
// `Marcher::it`.

@group(0) @binding(0) var<storage, read> P: array<u32>;
@group(0) @binding(1) var<storage, read_write> OUT: array<u32>;
// the constants of the .m3f formulas (all slots) and the CPU's image of
// the iteration record (u32 words from J4)
@group(0) @binding(2) var<storage, read> cst: array<u32>;
@group(0) @binding(3) var<storage, read> rtpl: array<u32>;

// parameter indices (gpu.rs writes them in this order)
const RECT_X0: u32 = 0u;
const RECT_Y0: u32 = 1u;
const RECT_W: u32 = 2u;
const ROW0: u32 = 3u;
const ROWS: u32 = 4u;
const HEIGHT: u32 = 5u;
const OPTIC: u32 = 6u;
const FOV_Y: u32 = 7u;
const FOVX_OFF: u32 = 8u;
const FOVX_MUL: u32 = 9u;
const PL_OPTIC_Z: u32 = 10u;
const VG: u32 = 11u; // 9 values, vgrads[0..3][0..3]
const YSTART: u32 = 20u; // 3 values
const MAX_IT: u32 = 23u;
const MIN_IT: u32 = 24u;
const D_RSTOP: u32 = 25u;
const RSTOP3D: u32 = 26u;
const DO_JULIA: u32 = 27u;
const JU: u32 = 28u; // 3 values
const POWER: u32 = 31u;
const Z_MUL: u32 = 32u;
const DE_STOP: u32 = 33u;
const DE_STOP_FACTOR: u32 = 34u;
const Z_STEP_DIV: u32 = 35u;
const MS_DE_SUB: u32 = 36u;
const MH04ZSD: u32 = 37u;
const DFOG_ON_IT: u32 = 38u;
const FIRST_STEP_RANDOM: u32 = 39u;
const DE_ADD_STEPS: u32 = 40u;
const NORMALS_ON_DE: u32 = 41u;
const SM_NORMALS: u32 = 42u;
const DE_OFFSET: u32 = 43u;
const DE_OFFSET006: u32 = 44u;
const D_DE_SCALE: u32 = 45u;
const ZCORR: u32 = 46u;
const ZC_MUL: u32 = 47u;
const ZEND: u32 = 48u;
const COL_VAR_DE_STOP_MUL: u32 = 49u;
const D_COL_PLUS: u32 = 50u;
const MCTS_M: u32 = 51u;
const STEP_WIDTH: u32 = 52u;
const COLOR_OPTION: u32 = 53u;
const MCT_COLOR_MUL: u32 = 54u;
const LN_RSTOP: u32 = 55u;
const FHLN0: u32 = 56u;
const IS_CUSTOM_DE: u32 = 57u;
const DE_OPTION: u32 = 58u;
const END_TO: u32 = 59u;
const REPEAT_FROM: u32 = 60u;
const J4: u32 = 61u;
// 6 slots of 8 words: iterations, uncounted, constants base, fHln,
// Integer Power power, z multiplier, kind (0 none, 1 Integer Power, 2 .m3f,
// 3 Amazing Box: see amazing_box)
const SLOTS: u32 = 64u;
fn slot(n: u32, k: u32) -> u32 { return P[SLOTS + n * 8u + k]; }

const PI: f32 = 3.14159265358979;

fn pf(i: u32) -> f32 { return bitcast<f32>(P[i]); }
fn pi(i: u32) -> i32 { return bitcast<i32>(P[i]); }
fn pv(i: u32) -> vec3<f32> { return vec3<f32>(pf(i), pf(i + 1u), pf(i + 2u)); }

// iteration state (Marcher::it)
var<private> c: vec3<f32>;
var<private> v: vec3<f32>;
var<private> rout: f32;
var<private> rold: f32;
var<private> otrap: f32;
var<private> it_result: i32;
var<private> max_it: i32;
var<private> rstop: f32;
var<private> calc_sit: bool;
var<private> smooth_it: f32;
// the rest of the iteration record the formulas see (Iteration)
var<private> w: f32;
var<private> j: vec3<f32>;
var<private> j4: f32;
var<private> vary_scale: f32;
var<private> first_it: i32;
var<private> deriv: vec3<f32>;
var<private> dfree: vec2<f32>;
// marcher state
var<private> ms_de_stop: f32;
var<private> mzz: f32;
var<private> vfov: vec3<f32>;
var<private> base: vec3<f32>;
var<private> max_its_result: i32;
var<private> s_roughness: f32;
var<private> seed: i32;
// the result (SiLight)
var<private> si_normal: vec3<i32>;
var<private> si_zpos: u32;
var<private> si_shadow: u32;
var<private> si_grad: u32;
var<private> si_otrap: u32;

fn is_nan(x: f32) -> bool {
    let b = bitcast<u32>(x) & 0x7FFFFFFFu;
    return b > 0x7F800000u;
}

// min_max_clip_15bit
fn clip15(s: f32) -> u32 {
    if (is_nan(s) || s < 0.0) { return 0u; }
    if (s > 32767.0) { return 32767u; }
    return u32(round(s));
}

// the position on the ray (CPU: c, moved with add_weight)
fn setc() {
    c = base + vfov * mzz;
}

// fHIntFunctions[power] (formulas.rs int_pow)
fn int_pow(p: vec3<f32>, j: vec3<f32>, power: i32, zmul: f32) -> vec3<f32> {
    let x = p.x;
    let y = p.y;
    let z = p.z;
    let EPS = 1e-30;
    var o: vec3<f32>;
    if (power <= 2) {
        let yy = y * y;
        let xx = x * x;
        let r = xx + yy;
        o.z = (sqrt(r) * zmul * z) * 2.0 + j.z;
        let a = (r - z * z) / r;
        o.x = (xx - yy) * a + j.x;
        o.y = (x * y * a) * 2.0 + j.y;
    } else if (power == 3) {
        let sy = y * y;
        let sx = x * x;
        let r = sx + sy;
        let sz = z * z;
        let a = 1.0 - 3.0 * sz / (r + EPS);
        o.x = (sx - 3.0 * sy) * a * x + j.x;
        o.z = j.z - (sz - 3.0 * r) * z * zmul;
        o.y = a * (3.0 * sx - sy) * y + j.y;
    } else if (power == 4) {
        let sy = y * y;
        let sx = x * x;
        let r = sx + sy;
        let sz = z * z;
        let a = 1.0 + sz * (sz - 6.0 * r) / (r * r + EPS);
        o.x = (sx * (sx - 6.0 * sy) + sy * sy) * a + j.x;
        o.z = sqrt(r) * (r - sz) * z * 4.0 * zmul + j.z;
        o.y = a * x * (sx - sy) * 4.0 * y + j.y;
    } else if (power == 5) {
        let sy = y * y;
        let sx = x * x;
        let r = sx + sy;
        let sz = z * z;
        let a = 1.0 + (sz * sz - sz * r * 2.0) * 5.0 / (r * r + EPS);
        o.y = (5.0 * sx * sx - sy * (10.0 * sx - sy)) * a * y + j.y;
        o.z = (sz * (sz - 10.0 * r) + 5.0 * r * r) * z * zmul + j.z;
        o.x = a * x * (sx * (sx - 10.0 * sy) + 5.0 * sy * sy) + j.x;
    } else if (power == 6) {
        let sy = y * y;
        let sx = x * x;
        let r = sx + sy;
        let sz = z * z;
        let rr = r * r;
        let a = 1.0 - sz * (15.0 * rr + sz * (sz - r * 15.0)) / (rr * r + EPS);
        o.y = (3.0 * sy * sy + sx * (3.0 * sx - 10.0 * sy)) * a * y * x * 2.0 + j.y;
        o.z = (sz * (3.0 * sz - 10.0 * r) + 3.0 * rr) * sqrt(r) * z * zmul * 2.0 + j.z;
        o.x = (sy * sy * (15.0 * sx - sy) + sx * sx * (sx - 15.0 * sy)) * a + j.x;
    } else if (power == 7) {
        let sy = y * y;
        let sx = x * x;
        let r = sx + sy;
        let sz = z * z;
        let rr = r * r;
        let a = 1.0 - 7.0 * (sz * (3.0 * rr + sz * (sz - 5.0 * r))) / (rr * r + EPS);
        o.y = (sx * (21.0 * sy * sy + sx * (7.0 * sx - 35.0 * sy)) - sy * sy * sy) * a * y + j.y;
        o.z = j.z - (sz * sz * sz - 7.0 * r * (rr + sz * (3.0 * sz - 5.0 * r))) * z * zmul;
        o.x = a * x * (sx * (35.0 * sy * sy + sx * (sx - 21.0 * sy)) - 7.0 * sy * sy * sy) + j.x;
    } else {
        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let r = xx + yy;
        let rr = r * r;
        let zzzz = zz * zz;
        let t = (zzzz - 6.0 * r * zz + rr) * (zz - r) * sqrt(r);
        o.z = -(t * z * 8.0 * zmul) + j.z;
        let num = (rr * 70.0 + zzzz) * zzzz - 28.0 * zz * r * (zzzz + rr);
        let a = num / (rr * rr + EPS) + 1.0;
        let xxxx = xx * xx;
        let yyyy = yy * yy;
        o.y = (yyyy * (7.0 * xx - yy) + xxxx * (xx - 7.0 * yy)) * 8.0 * x * y * a + j.y;
        o.x = a * (xxxx * xxxx + yyyy * (70.0 * xxxx + yyyy) - 28.0 * (xx * yy * (yyyy + xxxx))) + j.x;
    }
    return o;
}

// HybridCube / HybridCubeDE (formulas.rs amazing_box); slot words: 4 scale,
// 5 min R, 7 fold, 2 analytic DE (the running derivative in w)
fn box_fold(x: f32, fold: f32) -> f32 {
    return abs(x + fold) - (abs(x - fold) + x);
}

fn amazing_box(n: u32) {
    let scale = bitcast<f32>(slot(n, 4u));
    let min_r = bitcast<f32>(slot(n, 5u));
    let fold = bitcast<f32>(slot(n, 7u));
    let x = box_fold(v.x, fold);
    let y = box_fold(v.y, fold);
    let z = box_fold(v.z, fold);
    let r = z * z + y * y + x * x;
    let sqr_min_r = min_r * min_r;
    var mul: f32;
    if (r < sqr_min_r) {
        mul = scale / sqr_min_r;
    } else if (1.0 < r) {
        mul = scale;
    } else {
        mul = scale / r;
    }
    if (slot(n, 2u) != 0u) {
        w *= mul;
    }
    v = vec3<f32>(x * mul + j.x, y * mul + j.y, z * mul + j.z);
}

// CalcSmoothIterations (fHln of the last formula n)
fn calc_smooth_iterations(n: u32) {
    if (rout <= 1.0) {
        smooth_it = f32(it_result);
    } else if (rold < 1.0) {
        let d = log(0.5 * log(rout)) * bitcast<f32>(slot(n, 3u));
        smooth_it = f32(it_result) + pf(LN_RSTOP) - d;
    } else {
        let d = log(0.5 * log(rout));
        let e = d - log(0.5 * log(rold));
        smooth_it = (pf(LN_RSTOP) - d) / (e + 1e-30) + f32(it_result);
    }
}

//@FORMULAS@

// doHybridPas / doHybridPasDE (Iteration::run, hybrid_3d, hybrid_3d_de):
// the alternating hybrid; `de`: with the analytic DE (returned)
fn hybrid(de: bool) -> f32 {
    if (pi(DO_JULIA) != 0) {
        j = pv(JU);
    } else {
        j = c;
    }
    v = c;
    w = 0.0;
    rout = dot(c, c);
    otrap = rout;
    let deo = pi(DE_OPTION);
    if (de) {
        if ((deo & 0x38) == 16) {
            w = rout;
        } else if ((deo & 0x38) == 32) {
            deriv = vec3<f32>(1.0, 0.0, 0.0);
        } else {
            w = 1.0;
        }
    }
    var n = 0u;
    var btmp = max(pi(SLOTS), 0);
    it_result = 0;
    first_it = 0;
    var uncounted_guard = 0u;
    loop {
        rold = rout;
        var guard = 0;
        while (btmp <= 0) {
            n += 1u;
            if (n > P[END_TO]) {
                n = P[REPEAT_FROM];
            }
            btmp = max(bitcast<i32>(slot(n, 0u)), 0);
            guard += 1;
            if (guard > 64) {
                break;
            }
        }
        if (btmp <= 0) {
            break;
        }
        call_slot(n);
        btmp -= 1;
        if (slot(n, 1u) != 0u) {
            uncounted_guard += 1u;
            if (uncounted_guard > 100000u) {
                break;
            }
            continue;
        }
        it_result += 1;
        rout = v.x * v.x + v.y * v.y + v.z * v.z;
        if (rout < otrap) {
            otrap = rout;
        }
        if (it_result >= max_it || rout > rstop) {
            break;
        }
    }
    var result = 0.0;
    if (de) {
        if ((deo & 0x38) == 32) {
            result = sqrt(rout) * 0.5 * log(rout) / deriv.x;
        } else if ((deo & 7) == 4) {
            result = abs(v.y) * log(abs(v.y)) / w;
        } else {
            result = sqrt(rout) / abs(w);
        }
    }
    if (calc_sit) {
        calc_smooth_iterations(n);
    }
    return result;
}

// The one place that iterates the formulas (the driver inlines functions,
// and the formula code must not be copied for every caller): the DE
// (`on_de`: CalcDEanalytic, or CalcDEnoADE with its numerical gradient)
// or the smoothed iteration count (mMandFunction with calc_sit).
fn evaluate(on_de: bool) -> f32 {
    let custom = on_de && pi(IS_CUSTOM_DE) != 0;
    let numeric = on_de && !custom;
    let bc = c;
    var buf_rout = 0.0;
    var buf_sit = calc_sit;
    var buf_max_it = max_it;
    var g = 0.0;
    var hres = 0.0;
    var zero = false;
    var nk = 1u;
    if (numeric) {
        nk = 4u;
    }
    for (var k = 0u; k < nk; k++) {
        if (k > 0u) {
            var o = vec3<f32>(0.0);
            o[k - 1u] = pf(DE_OFFSET);
            c = bc + o;
        }
        hres = hybrid(custom);
        if (numeric) {
            if (k == 0u) {
                if (rout <= 0.0) {
                    zero = true;
                    break;
                }
                buf_rout = rout;
                buf_sit = calc_sit;
                buf_max_it = max_it;
                calc_sit = false;
                max_it = it_result;
                rstop = pf(RSTOP3D);
            } else {
                g += (buf_rout - rout) * (buf_rout - rout);
            }
        }
    }
    if (!on_de) {
        return smooth_it;
    }
    var result: f32;
    if (custom) {
        result = hres * pf(D_DE_SCALE);
    } else if (zero) {
        result = 0.0;
    } else {
        c = bc;
        result = buf_rout * log(buf_rout) * pf(D_DE_SCALE) / (sqrt(g) + pf(DE_OFFSET006));
        rout = buf_rout;
        it_result = max_it;
        max_it = buf_max_it;
        calc_sit = buf_sit;
        rstop = pf(D_RSTOP);
    }
    max_its_result = max_it;
    let mn = ms_de_stop * 0.25;
    if (result < mn || is_nan(result)) {
        result = mn;
    }
    return result;
}

fn vg(i: u32) -> vec3<f32> {
    return pv(VG + i * 3u);
}

// rotate_vector_reverse(v, vgrads)
fn rot_rev(a: vec3<f32>) -> vec3<f32> {
    return a.x * vg(0u) + a.y * vg(1u) + a.z * vg(2u);
}

fn update_de_stop() {
    ms_de_stop = pf(DE_STOP) * (1.0 + mzz * pf(DE_STOP_FACTOR));
}

// MakeWNormalsFromDVec
fn store_normal(n: vec3<f32>) {
    let d = 32767.0 / sqrt(dot(n, n) + 1e-30);
    var o: vec3<i32>;
    for (var k = 0; k < 3; k++) {
        let x = round(n[k] * d);
        if (is_nan(x)) {
            o[k] = 0;
        } else {
            o[k] = i32(clamp(x, -32767.0, 32767.0));
        }
    }
    if (o.x == 0 && o.y == 0) {
        o.z = select(-32767, 32767, o.z > 0);
    }
    si_normal = o;
}

// RMCalcRoughness
fn calc_roughness(n: vec3<f32>, dt2: f32, dsg: f32) -> f32 {
    let a = (dsg * 7.0 * dt2 * dt2 + 1e-30) / (dot(n, n) + 1e-30);
    return clamp(sqrt(max(a, 0.0)) - 0.05, 0.0, 1.0);
}

// create_xy_vecs_from_normals
fn xy_vecs(n: vec3<f32>, vx: ptr<function, vec3<f32>>, vy: ptr<function, vec3<f32>>) {
    let d = n.y * n.y + n.x * n.x;
    if (d < 1e-30) {
        *vx = vec3<f32>(1.0, 0.0, 0.0);
        *vy = vec3<f32>(0.0, 1.0, 0.0);
        return;
    }
    let a = acos(clamp(-n.z / sqrt(d + n.z * n.z + 1e-30), -1.0, 1.0)) * 0.5;
    let sin_a = sin(a) / sqrt(d);
    let cos_a = cos(a);
    let dd = -n.y * sin_a;
    let d1 = n.x * sin_a;
    let x = vec3<f32>(1.0 - 2.0 * d1 * d1, 2.0 * dd * d1, 2.0 * d1 * cos_a);
    *vx = x;
    *vy = vec3<f32>(x.y, 1.0 - 2.0 * dd * dd, -2.0 * dd * cos_a);
}

// RMdoColor
fn do_color() {
    let x = v.x;
    let y = v.y;
    let z = v.z;
    var s: f32;
    switch (pi(COLOR_OPTION)) {
        case 1: {
            s = log(rout / (rold + 1.0)) * pf(MCT_COLOR_MUL);
        }
        case 2: {
            s = (atan2(y - c.y, x - c.x) + PI) * 5200.0;
        }
        case 3: {
            s = (atan2(z - c.z, x - c.x) + PI) * 5200.0;
        }
        case 4: {
            s = (atan2(z - c.z, y - c.y) + PI) * 5200.0;
        }
        case 5: {
            s = (atan2(x, y) + PI) * 5215.0;
            let r = sqrt(x * x + y * y + 1e-30 + z * z);
            let s2 = (PI + asin(clamp(z / r, -1.0, 1.0)) * 2.0) * 5215.0;
            si_grad = clip15(s2);
        }
        default: {
            s = otrap * 4096.0;
        }
    }
    si_otrap = clip15(s);
}

fn calc_zpos_and_rough() {
    let zz = max(mzz, 0.0);
    // zc_mul * (sqrt(zz * zcorr + 1) - 1) without the cancellation
    let a = zz * pf(ZCORR);
    let vv = round(pf(ZC_MUL) * a / (sqrt(a + 1.0) + 1.0));
    let itmp = u32(clamp(8388352.0 - vv, 0.0, 8388352.0));
    var r = itmp << 8u;
    if (pi(SM_NORMALS) > 0) {
        r |= u32(round(s_roughness * 255.0)) & 0xFFu;
    }
    si_zpos = r;
}

// states of march_pixel; a state may request an evaluation (`need`, at
// `c`, DE or iteration count by `want_de`), whose result the next state gets
const P_START: u32 = 0u;     // result: the DE at the start
const P_LOOP: u32 = 1u;      // the top of the march loop
const P_HALF: u32 = 2u;      // result: the DE after the half step back
const P_DECIDE: u32 = 3u;    // step or surface
const P_STEP: u32 = 4u;      // result: the DE after a step
const P_BS: u32 = 5u;        // RMdoBinSearch: the loop
const P_BS_R: u32 = 6u;      // result of a binary search step
const P_BSI: u32 = 7u;       // RMdoBinSearchIt: the loop
const P_BSI_1: u32 = 8u;     // result: the first DE of a step
const P_BSI_2: u32 = 9u;     // result: the DE 0.001 back
const P_BSI_END: u32 = 10u;
const P_N_INIT: u32 = 11u;   // normals: the centre sample
const P_N_0: u32 = 12u;      // result: the centre sample
const P_N8: u32 = 13u;       // 5x5x5 samples
const P_N8_R: u32 = 14u;
const P_NA: u32 = 15u;       // the 6 axis samples
const P_NA_R: u32 = 16u;
const P_NSM: u32 = 17u;      // smoothing: the 4 samples around
const P_NSM_R: u32 = 18u;
const P_SW: u32 = 19u;       // smoothing: the sweeps
const P_SW_R: u32 = 20u;
const P_NEND: u32 = 21u;

fn march_pixel(x: i32, y: i32) {
    si_normal = vec3<i32>(0);
    si_zpos = 32768u << 16u;
    si_shadow = 0u;
    si_grad = 0u;
    si_otrap = 0u;
    s_roughness = 0.0;
    max_it = pi(MAX_IT);
    rstop = pf(D_RSTOP);
    max_its_result = max_it;

    // RMCalculateVgradsFOV
    let cafy = (f32(y) / f32(pi(HEIGHT)) - 0.5) * pf(FOV_Y);
    let cafx = (pf(FOVX_OFF) - f32(x + 1)) * pf(FOVX_MUL);
    var dv: vec3<f32>;
    let optic = pi(OPTIC);
    if (optic == 1) {
        dv = normalize(vec3<f32>(-cafx, cafy, pf(PL_OPTIC_Z)));
    } else if (optic == 2) {
        dv = vec3<f32>(-sin(cafx) * cos(cafy), sin(cafy), cos(cafy) * cos(cafx));
    } else {
        dv = normalize(vec3<f32>(-sin(cafx), sin(cafy), cos(cafy) * cos(cafx)));
    }
    vfov = rot_rev(dv);
    // RMCalculateStartPos
    if (optic == 2) {
        base = pv(YSTART);
    } else {
        base = pv(YSTART) + vg(0u) * f32(x) + vg(1u) * f32(y);
    }
    calc_sit = false;
    var step_count = 0.0;
    mzz = 0.0;
    setc();
    ms_de_stop = pf(DE_STOP);
    var first_step = pi(FIRST_STEP_RANDOM) != 0;
    let dfog_on_it = pi(DFOG_ON_IT);
    let on_de = pi(NORMALS_ON_DE) != 0;
    let sm = pi(SM_NORMALS);

    // the state
    var pc = P_START;
    var need = true;
    var want_de = true;
    var r = 0.0;
    var done = false;
    // march
    var dtmp = 0.0;
    var rsf_mul = 1.0;
    var last_step = 0.0;
    var last_de = 0.0;
    var dt1 = 0.0;
    var de_limited = false;
    var itmp = 0;
    // RMdoBinSearchIt
    var yp = 0.0;
    var saved_max = 0;
    var dmul = 1.0;
    var first = true;
    var last_si = 0.0;
    var last_dif = 0.0;
    // normals
    var noffset = 0.0;
    var ct1 = vec3<f32>(0.0);
    var dnn = 0.0;
    var nn = 0.0;
    var n = vec3<f32>(0.0);
    var cnt = 0;
    var na = 0.0;
    var acc = 0.0;
    var ssn = 0.0;
    var dm = 0.0;
    var vx = vec3<f32>(0.0);
    var vy = vec3<f32>(0.0);
    var s1 = vec2<f32>(0.0);
    var ds = vec2<f32>(0.0);
    var axis = 0;
    var kk = 0;

    for (var guard = 0; guard < 2000000; guard++) {
        if (need) {
            r = evaluate(want_de);
            need = false;
        }
        switch (pc) {
            case P_START: {
                dtmp = r;
                if (it_result >= max_its_result || dtmp < ms_de_stop) {
                    // inside the set at the start plane
                    si_zpos = 0x7FFF0000u;
                    si_normal = vec3<i32>(0, 0, -32767);
                    do_color();
                    if (pi(COLOR_OPTION) > 4) {
                        si_grad |= 32768u;
                    } else {
                        let t = clamp(rout / pf(D_RSTOP), 0.0, 1.0);
                        si_grad = 32768u + u32(round(32767.0 * t));
                    }
                    return;
                }
                rsf_mul = 1.0;
                last_step = dtmp * pf(Z_STEP_DIV);
                pc = P_LOOP;
            }
            case P_LOOP: {
                if (it_result >= max_its_result) {
                    dt1 = -0.5 * last_step;
                    mzz += dt1;
                    setc();
                    update_de_stop();
                    need = true;
                    want_de = true;
                    pc = P_HALF;
                } else {
                    pc = P_DECIDE;
                }
            }
            case P_HALF: {
                dtmp = r;
                last_step = -dt1;
                pc = P_DECIDE;
            }
            case P_DECIDE: {
                if (it_result < pi(MIN_IT) || (it_result < max_its_result && dtmp >= ms_de_stop)) {
                    // next step
                    last_de = dtmp;
                    var d = max(0.11, (dtmp - pf(MS_DE_SUB) * ms_de_stop) * pf(Z_STEP_DIV) * rsf_mul);
                    let st1 = max(ms_de_stop, 0.4) * pf(MH04ZSD);
                    let count = dfog_on_it == 0 || it_result == dfog_on_it;
                    if (st1 < d) {
                        if (count) {
                            step_count += st1 / d;
                        }
                        d = st1;
                    } else if (count) {
                        step_count += 1.0;
                    }
                    if (first_step) {
                        first_step = false;
                        seed = seed * 214013 + 2531011;
                        d *= f32(u32(seed) & 0x7FFFFFFFu) * (1.0 / 2147483647.0);
                    }
                    mzz += d;
                    if (mzz > pf(ZEND)) {
                        // background (`break` here would only leave the switch)
                        done = true;
                    } else {
                        last_step = d;
                        setc();
                        update_de_stop();
                        need = true;
                        want_de = true;
                        pc = P_STEP;
                    }
                } else {
                    // surface found
                    de_limited = it_result < max_its_result || dtmp < ms_de_stop;
                    if (pi(DE_ADD_STEPS) != 0) {
                        itmp = pi(DE_ADD_STEPS);
                        if (de_limited) {
                            dt1 = last_step * -0.5;
                            pc = P_BS;
                        } else {
                            yp = f32(max_it) - 0.99;
                            saved_max = max_it;
                            max_it += 1;
                            calc_sit = true;
                            dt1 = 0.0;
                            dmul = 1.0;
                            first = true;
                            last_si = 0.0;
                            last_dif = 0.0;
                            pc = P_BSI;
                        }
                    } else {
                        pc = P_N_INIT;
                    }
                }
            }
            case P_STEP: {
                dtmp = r;
                if (dtmp > last_de + last_step) {
                    dtmp = last_de + last_step;
                }
                rsf_mul = 1.0;
                if (last_de > dtmp + 1e-30) {
                    let t = last_step / (last_de - dtmp);
                    if (t < 1.0) {
                        rsf_mul = max(0.5, t);
                    }
                }
                pc = P_LOOP;
            }
            // RMdoBinSearch
            case P_BS: {
                if (abs(dtmp - ms_de_stop) <= 0.001) {
                    pc = P_N_INIT;
                } else {
                    mzz += dt1;
                    setc();
                    update_de_stop();
                    itmp -= 1;
                    if (itmp <= 0) {
                        pc = P_N_INIT;
                    } else {
                        need = true;
                        want_de = true;
                        pc = P_BS_R;
                    }
                }
            }
            case P_BS_R: {
                dtmp = r;
                if (it_result >= max_its_result) {
                    dt1 = -abs(dt1);
                } else if (dtmp < ms_de_stop) {
                    dt1 = abs(dt1) * -0.55;
                } else {
                    dt1 = abs(dt1) * 0.55;
                }
                pc = P_BS;
            }
            // RMdoBinSearchIt
            case P_BSI: {
                mzz += dt1;
                setc();
                need = true;
                want_de = true;
                pc = P_BSI_1;
            }
            case P_BSI_1: {
                if (!first && last_dif < abs(yp - smooth_it)) {
                    mzz -= dt1;
                    setc();
                    smooth_it = last_si;
                    if (dt1 > 0.0) {
                        dmul *= 0.5;
                    } else {
                        dmul *= 0.7;
                    }
                }
                last_dif = abs(yp - smooth_it);
                last_si = smooth_it;
                if (smooth_it > f32(max_it) - 0.1) {
                    dt1 = -3.0;
                    pc = P_BSI_END;
                } else {
                    mzz -= 0.001;
                    setc();
                    need = true;
                    want_de = true;
                    pc = P_BSI_2;
                }
            }
            case P_BSI_2: {
                let rr = last_si - smooth_it;
                if (abs(rr) < 1e-30) {
                    dt1 = select(0.0, 1.0, last_si < smooth_it) - 0.5;
                } else if (rr < 0.0) {
                    dt1 = (yp - smooth_it) / (rr * 500.0);
                } else {
                    dt1 = (yp - smooth_it) / (rr * 1000.0);
                }
                if (dt1 > 4.0) {
                    dt1 = sqrt(dt1) * 2.0;
                } else if (dt1 < -9.0) {
                    dt1 = sqrt(-dt1) * -3.0;
                }
                dt1 = dt1 * dmul + 0.0005;
                pc = P_BSI_END;
            }
            case P_BSI_END: {
                first = false;
                itmp -= 1;
                if (itmp < 0) {
                    max_it = saved_max;
                    pc = P_N_INIT;
                } else {
                    pc = P_BSI;
                }
            }
            // RMCalculateNormals (on_de) / RMCalculateNormalsOnSmoothIt
            case P_N_INIT: {
                noffset = min(pf(DE_STOP), 1.0) * (1.0 + mzz * pf(DE_STOP_FACTOR)) * 0.15;
                calc_sit = true;
                ct1 = c;
                need = true;
                want_de = on_de;
                pc = P_N_0;
            }
            case P_N_0: {
                dnn = r;
                nn = smooth_it;
                if (on_de) {
                    calc_sit = false;
                }
                n = vec3<f32>(0.0);
                cnt = 0;
                if (sm == 8) {
                    pc = P_N8;
                } else {
                    pc = P_NA;
                }
            }
            case P_N8: {
                // the next of the 124 points of the 5x5x5 cube (not the centre)
                if (cnt >= 125) {
                    n *= 0.0075;
                    pc = P_NSM;
                } else {
                    let a = cnt / 25 - 2;
                    let b = (cnt / 5) % 5 - 2;
                    let cc = cnt % 5 - 2;
                    if ((a | b | cc) == 0) {
                        cnt += 1;
                    } else {
                        let sn = noffset * 1.3333;
                        c = ct1 + vg(2u) * (f32(a) * sn) + vg(1u) * (f32(b) * sn) + vg(0u) * (f32(cc) * sn);
                        need = true;
                        want_de = on_de;
                        pc = P_N8_R;
                    }
                }
            }
            case P_N8_R: {
                let a = cnt / 25 - 2;
                let b = (cnt / 5) % 5 - 2;
                let cc = cnt % 5 - 2;
                let d = r * select(-1.0, 1.0, on_de);
                if (a != 0) { n.z += d / f32(a); }
                if (b != 0) { n.y += d / f32(b); }
                if (cc != 0) { n.x += d / f32(cc); }
                cnt += 1;
                pc = P_N8;
            }
            case P_NA: {
                // z, x, y as on the CPU; + then - the offset
                if (cnt >= 6) {
                    pc = P_NSM;
                } else {
                    let ax = (u32(cnt / 2) + 2u) % 3u;
                    if (cnt % 2 == 0) {
                        c = ct1 + vg(ax) * noffset;
                    } else {
                        c = ct1 - vg(ax) * noffset;
                    }
                    need = true;
                    want_de = on_de;
                    pc = P_NA_R;
                }
            }
            case P_NA_R: {
                let ax = (u32(cnt / 2) + 2u) % 3u;
                if (cnt % 2 == 0) {
                    na = r;
                } else {
                    n[ax] = select(r - na, na - r, on_de) * 0.5;
                }
                cnt += 1;
                pc = P_NA;
            }
            case P_NSM: {
                if (sm <= 0) {
                    pc = P_NEND;
                } else {
                    if (cnt < 1000) {
                        // first visit: start the 4 samples around
                        noffset *= 2.0;
                        cnt = 1000;
                        acc = nn;
                    }
                    let i = cnt - 1000;
                    if ((on_de && sm >= 8) || i >= 4) {
                        if (on_de) {
                            if (sm < 8) {
                                dnn = dnn * 0.2;
                            }
                        } else {
                            nn = acc * 0.2;
                            dnn = nn;
                        }
                        ssn = noffset * 3.0 / (f32(sm) + 0.5);
                        dm = f32(sm) * 2.0;
                        xy_vecs(n, &vx, &vy);
                        vx = rot_rev(vx);
                        vy = rot_rev(vy);
                        s1 = vec2<f32>(0.0);
                        ds = vec2<f32>(0.0);
                        axis = 0;
                        kk = -sm;
                        pc = P_SW;
                    } else {
                        let ax = u32(i / 2);
                        if (i % 2 == 0) {
                            c = ct1 - vg(ax) * noffset;
                        } else {
                            c = ct1 + vg(ax) * noffset;
                        }
                        need = true;
                        want_de = on_de;
                        pc = P_NSM_R;
                    }
                }
            }
            case P_NSM_R: {
                if (on_de) {
                    dnn += r;
                } else {
                    acc += r;
                }
                cnt += 1;
                pc = P_NSM;
            }
            case P_SW: {
                if (axis >= 2) {
                    let dsg = ds.x * dm - s1.x * s1.x + ds.y * dm - s1.y * s1.y;
                    let dt2 = noffset * 0.5 / (dm * ssn);
                    s_roughness = calc_roughness(n, dt2, dsg);
                    if (sm < 8) {
                        n.x += s1.x * dt2;
                        n.y += s1.y * dt2;
                    }
                    pc = P_NEND;
                } else if (kk > sm) {
                    axis += 1;
                    kk = -sm;
                } else if (kk == 0) {
                    kk += 1;
                } else {
                    let dir = select(vy, vx, axis == 0);
                    c = ct1 + dir * (f32(kk) * ssn);
                    need = true;
                    want_de = on_de;
                    pc = P_SW_R;
                }
            }
            case P_SW_R: {
                var d: f32;
                if (on_de) {
                    d = (r - dnn) / f32(kk);
                } else {
                    d = (dnn - r) / f32(kk);
                }
                s1[axis] += d;
                ds[axis] += d * d;
                kk += 1;
                pc = P_SW;
            }
            case P_NEND: {
                c = ct1;
                store_normal(n);
                var g: f32;
                if (de_limited) {
                    g = 32767.0 - (nn + pf(D_COL_PLUS) + pf(COL_VAR_DE_STOP_MUL)
                        * log(max(pf(DE_STOP), ms_de_stop) * pf(STEP_WIDTH))) * pf(MCTS_M);
                } else {
                    g = 32767.0 - nn * pf(MCTS_M);
                }
                si_grad = clip15(g);
                do_color();
                calc_zpos_and_rough();
                done = true;
            }
            default: {
                done = true;
            }
        }
        if (done) {
            break;
        }
    }
    si_shadow = u32(round(clamp(step_count, 0.0, 1023.0)));
}


@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let lx = i32(gid.x);
    let ly = i32(gid.y);
    let w = pi(RECT_W);
    if (lx >= w || ly >= pi(ROWS)) {
        return;
    }
    let x = pi(RECT_X0) + lx;
    let y = pi(RECT_Y0) + pi(ROW0) + ly;
    // MB3D seeds per row; the CPU advances the seed along the row, here the
    // seed is advanced by the column (the same random sequence, but not
    // exactly the same numbers where pixels have no first step)
    seed = bitcast<i32>(0x24563487u + u32(y + 1) * 0x324594A1u);
    for (var k = 0; k < lx; k++) {
        seed = seed * 214013 + 2531011;
    }
    _ = cst[0];
    _ = rtpl[0];
    init_record();
    j4 = pf(J4);
    vary_scale = 1.0;
    deriv = vec3<f32>(1.0, 0.0, 0.0);
    dfree = vec2<f32>(0.0, 0.0);
    march_pixel(x, y);
    let o = u32(ly * w + lx) * 4u;
    OUT[o] = (u32(si_normal.x) & 0xFFFFu) | (u32(si_normal.y) << 16u);
    OUT[o + 1u] = (u32(si_normal.z) & 0xFFFFu) | (si_shadow << 16u);
    OUT[o + 2u] = si_zpos;
    OUT[o + 3u] = si_grad | (si_otrap << 16u);
}
