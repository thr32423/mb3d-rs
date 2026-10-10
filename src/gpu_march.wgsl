// The ray marcher of calc.rs (`Marcher::march_pixel`, MB3D's
// TMandCalcThread.Execute) for the graphics card, in single precision.
// Prototype: one 'Integer Power' formula (the Mandelbulbs), numerical DE,
// no cutting planes, inside rendering or volumetric light (gpu.rs checks).
// The structure follows calc.rs closely so the two can be compared line by
// line; the iteration state is kept in private variables like the CPU's
// `Marcher::it`.

@group(0) @binding(0) var<storage, read> P: array<u32>;
@group(0) @binding(1) var<storage, read_write> OUT: array<u32>;

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
fn int_pow(p: vec3<f32>, j: vec3<f32>) -> vec3<f32> {
    let x = p.x;
    let y = p.y;
    let z = p.z;
    let zmul = pf(Z_MUL);
    let power = pi(POWER);
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

// CalcSmoothIterations
fn calc_smooth_iterations() {
    if (rout <= 1.0) {
        smooth_it = f32(it_result);
    } else if (rold < 1.0) {
        let d = log(0.5 * log(rout)) * pf(FHLN0);
        smooth_it = f32(it_result) + pf(LN_RSTOP) - d;
    } else {
        let d = log(0.5 * log(rout));
        let e = d - log(0.5 * log(rold));
        smooth_it = (pf(LN_RSTOP) - d) / (e + 1e-30) + f32(it_result);
    }
}

// doHybridPas with one formula (Iteration::hybrid_3d)
fn mand_function() {
    var j = c;
    if (pi(DO_JULIA) != 0) {
        j = pv(JU);
    }
    v = c;
    rout = dot(c, c);
    otrap = rout;
    it_result = 0;
    loop {
        rold = rout;
        v = int_pow(v, j);
        it_result += 1;
        rout = v.x * v.x + v.y * v.y + v.z * v.z;
        if (rout < otrap) {
            otrap = rout;
        }
        if (it_result >= max_it || rout > rstop) {
            break;
        }
    }
    if (calc_sit) {
        calc_smooth_iterations();
    }
}

// CalcDEnoADE (calc_de_part, numerical gradient)
fn calc_de() -> f32 {
    mand_function();
    var result: f32;
    if (rout <= 0.0) {
        result = 0.0;
    } else {
        let buf_sit = calc_sit;
        let buf_max_it = max_it;
        let buf_rout = rout;
        calc_sit = false;
        max_it = it_result;
        rstop = pf(RSTOP3D);
        let off = pf(DE_OFFSET);
        let bc = c;
        c = bc + vec3<f32>(off, 0.0, 0.0);
        mand_function();
        let g0 = (buf_rout - rout) * (buf_rout - rout);
        c = bc + vec3<f32>(0.0, off, 0.0);
        mand_function();
        let g1 = (buf_rout - rout) * (buf_rout - rout);
        c = bc + vec3<f32>(0.0, 0.0, off);
        mand_function();
        let g2 = (buf_rout - rout) * (buf_rout - rout);
        c = bc;
        result = buf_rout * log(buf_rout) * pf(D_DE_SCALE) / (sqrt(g0 + g1 + g2) + pf(DE_OFFSET006));
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

// RMdoBinSearch
fn bin_search(de0: f32, last_step_width: f32) -> f32 {
    var de = de0;
    var itmp = pi(DE_ADD_STEPS);
    var dt1 = last_step_width * -0.5;
    loop {
        if (abs(de - ms_de_stop) <= 0.001) {
            break;
        }
        mzz += dt1;
        setc();
        update_de_stop();
        itmp -= 1;
        if (itmp <= 0) {
            break;
        }
        de = calc_de();
        if (it_result >= max_its_result) {
            dt1 = -abs(dt1);
        } else if (de < ms_de_stop) {
            dt1 = abs(dt1) * -0.55;
        } else {
            dt1 = abs(dt1) * 0.55;
        }
    }
    return de;
}

// RMdoBinSearchIt
fn bin_search_it() {
    let yp = f32(max_it) - 0.99;
    let saved_max = max_it;
    max_it += 1;
    var itmp = pi(DE_ADD_STEPS);
    calc_sit = true;
    var dt1 = 0.0;
    var dmul = 1.0;
    var first = true;
    var last_si = 0.0;
    var last_dif = 0.0;
    loop {
        mzz += dt1;
        setc();
        _ = calc_de();
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
        } else {
            mzz -= 0.001;
            setc();
            _ = calc_de();
            let r = last_si - smooth_it;
            if (abs(r) < 1e-30) {
                dt1 = select(0.0, 1.0, last_si < smooth_it) - 0.5;
            } else if (r < 0.0) {
                dt1 = (yp - smooth_it) / (r * 500.0);
            } else {
                dt1 = (yp - smooth_it) / (r * 1000.0);
            }
            if (dt1 > 4.0) {
                dt1 = sqrt(dt1) * 2.0;
            } else if (dt1 < -9.0) {
                dt1 = sqrt(-dt1) * -3.0;
            }
            dt1 = dt1 * dmul + 0.0005;
        }
        first = false;
        itmp -= 1;
        if (itmp < 0) {
            break;
        }
    }
    max_it = saved_max;
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

// one sample of the normal calculation: the DE (on_de) or the smoothed
// iteration count at c
fn sample(on_de: bool) -> f32 {
    if (on_de) {
        return calc_de();
    }
    mand_function();
    return smooth_it;
}

// RMCalculateNormals (on_de) / RMCalculateNormalsOnSmoothIt; returns NN
fn calculate_normals(on_de: bool) -> f32 {
    let sm = pi(SM_NORMALS);
    var noffset = min(pf(DE_STOP), 1.0) * (1.0 + mzz * pf(DE_STOP_FACTOR)) * 0.15;
    calc_sit = true;
    let ct1 = c;
    var dnn = sample(on_de);
    var nn = smooth_it;
    if (on_de) {
        calc_sit = false;
    }
    // the DE falls towards the surface, the iteration count rises
    let sgn = select(-1.0, 1.0, on_de);
    var n = vec3<f32>(0.0);
    if (sm == 8) {
        let ssn = noffset * 1.3333;
        for (var a = -2; a <= 2; a++) {
            for (var b = -2; b <= 2; b++) {
                for (var cc = -2; cc <= 2; cc++) {
                    if ((a | b | cc) == 0) {
                        continue;
                    }
                    c = ct1 + vg(2u) * (f32(a) * ssn) + vg(1u) * (f32(b) * ssn) + vg(0u) * (f32(cc) * ssn);
                    let d = sample(on_de) * sgn;
                    if (a != 0) { n.z += d / f32(a); }
                    if (b != 0) { n.y += d / f32(b); }
                    if (cc != 0) { n.x += d / f32(cc); }
                }
            }
        }
        n *= 0.0075;
    } else {
        for (var k = 0u; k < 3u; k++) {
            let axis = (k + 2u) % 3u; // z, x, y as on the CPU
            c = ct1 + vg(axis) * noffset;
            let a = sample(on_de);
            c = ct1 - vg(axis) * noffset;
            let b = sample(on_de);
            n[axis] = select(b - a, a - b, on_de) * 0.5;
        }
    }
    if (sm > 0) {
        noffset *= 2.0;
        if (on_de) {
            if (sm < 8) {
                c = ct1 - vg(0u) * noffset;
                dnn += calc_de();
                c = ct1 + vg(0u) * noffset;
                dnn += calc_de();
                c = ct1 - vg(1u) * noffset;
                dnn += calc_de();
                c = ct1 + vg(1u) * noffset;
                dnn = (dnn + calc_de()) * 0.2;
            }
        } else {
            var acc = nn;
            c = ct1 - vg(0u) * noffset;
            mand_function();
            acc += smooth_it;
            c = ct1 + vg(0u) * noffset;
            mand_function();
            acc += smooth_it;
            c = ct1 - vg(1u) * noffset;
            mand_function();
            acc += smooth_it;
            c = ct1 + vg(1u) * noffset;
            mand_function();
            acc += smooth_it;
            nn = acc * 0.2;
            dnn = nn;
        }
        let ssn = noffset * 3.0 / (f32(sm) + 0.5);
        let dm = f32(sm) * 2.0;
        var vx: vec3<f32>;
        var vy: vec3<f32>;
        xy_vecs(n, &vx, &vy);
        vx = rot_rev(vx);
        vy = rot_rev(vy);
        var s1 = vec2<f32>(0.0);
        var ds = vec2<f32>(0.0);
        for (var axis = 0; axis < 2; axis++) {
            let dir = select(vy, vx, axis == 0);
            for (var k = -sm; k <= sm; k++) {
                if (k != 0) {
                    c = ct1 + dir * (f32(k) * ssn);
                    var d: f32;
                    if (on_de) {
                        d = (calc_de() - dnn) / f32(k);
                    } else {
                        mand_function();
                        d = (dnn - smooth_it) / f32(k);
                    }
                    s1[axis] += d;
                    ds[axis] += d * d;
                }
            }
        }
        let dsg = ds.x * dm - s1.x * s1.x + ds.y * dm - s1.y * s1.y;
        let dt2 = noffset * 0.5 / (dm * ssn);
        s_roughness = calc_roughness(n, dt2, dsg);
        if (sm < 8) {
            n.x += s1.x * dt2;
            n.y += s1.y * dt2;
        }
    }
    c = ct1;
    store_normal(n);
    return nn;
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

    var dtmp = calc_de();
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
    var rsf_mul = 1.0;
    var last_step = dtmp * pf(Z_STEP_DIV);
    var last_de: f32;
    let dfog_on_it = pi(DFOG_ON_IT);
    for (var guard = 0; guard < 1000000; guard++) {
        if (it_result >= max_its_result) {
            let dt1 = -0.5 * last_step;
            mzz += dt1;
            setc();
            update_de_stop();
            dtmp = calc_de();
            last_step = -dt1;
        }
        if (it_result < pi(MIN_IT) || (it_result < max_its_result && dtmp >= ms_de_stop)) {
            // next step
            last_de = dtmp;
            var d = max(0.11, (dtmp - pf(MS_DE_SUB) * ms_de_stop) * pf(Z_STEP_DIV) * rsf_mul);
            let s1 = max(ms_de_stop, 0.4) * pf(MH04ZSD);
            let count = dfog_on_it == 0 || it_result == dfog_on_it;
            if (s1 < d) {
                if (count) {
                    step_count += s1 / d;
                }
                d = s1;
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
                break;
            }
            last_step = d;
            setc();
            update_de_stop();
            dtmp = calc_de();
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
        } else {
            // surface found
            let de_limited = it_result < max_its_result || dtmp < ms_de_stop;
            if (pi(DE_ADD_STEPS) != 0) {
                if (de_limited) {
                    dtmp = bin_search(dtmp, last_step);
                } else {
                    bin_search_it();
                }
            }
            let nn = calculate_normals(pi(NORMALS_ON_DE) != 0);
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
    march_pixel(x, y);
    let o = u32(ly * w + lx) * 4u;
    OUT[o] = (u32(si_normal.x) & 0xFFFFu) | (u32(si_normal.y) << 16u);
    OUT[o + 1u] = (u32(si_normal.z) & 0xFFFFu) | (si_shadow << 16u);
    OUT[o + 2u] = si_zpos;
    OUT[o + 3u] = si_grad | (si_otrap << 16u);
}
