//! Vector / matrix helpers, ported from `Math3D.pas`.
//!
//! Conventions follow the Delphi code: a `Mat3` is stored row-major and the
//! rows of the camera matrix (`VGrads`) are the world-space directions of the
//! screen X axis, screen Y axis and view (Z) axis.

pub type Vec3 = [f64; 3];
pub type Vec4 = [f64; 4];
pub type Mat3 = [[f64; 3]; 3];
pub type Mat4 = [[f64; 4]; 4];

#[inline]
pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
pub fn scale(a: Vec3, s: f64) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// `mAddVecWeight(V1, V2, w)`:  V1 := V1 + V2 * w
#[inline]
pub fn add_weight(a: &mut Vec3, b: &Vec3, w: f64) {
    a[0] += b[0] * w;
    a[1] += b[1] * w;
    a[2] += b[2] * w;
}

#[inline]
pub fn dot(a: &Vec3, b: &Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
pub fn sqr_len(a: &Vec3) -> f64 {
    dot(a, a)
}

#[inline]
pub fn len(a: &Vec3) -> f64 {
    sqr_len(a).sqrt()
}

pub fn normalize(a: Vec3) -> Vec3 {
    let l = len(&a);
    if l > 0.0 {
        scale(a, 1.0 / l)
    } else {
        a
    }
}

pub const IDENTITY3: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// `RotateVectorReverse`: V' = V * M  (i.e. sum_i V[i] * M[i]).
#[inline]
pub fn rotate_vector_reverse(v: &Vec3, m: &Mat3) -> Vec3 {
    [
        v[0] * m[0][0] + v[1] * m[1][0] + v[2] * m[2][0],
        v[0] * m[0][1] + v[1] * m[1][1] + v[2] * m[2][1],
        v[0] * m[0][2] + v[1] * m[1][2] + v[2] * m[2][2],
    ]
}

/// `RotateVector`: V' = M * V
#[inline]
pub fn rotate_vector(v: &Vec3, m: &Mat3) -> Vec3 {
    [dot(&m[0], v), dot(&m[1], v), dot(&m[2], v)]
}

/// `NormaliseMatrixTo`: scale every row to length `n`.
pub fn normalise_matrix_to(n: f64, m: &Mat3) -> Mat3 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        let d = n / sqr_len(&m[i]).sqrt();
        for j in 0..3 {
            r[i][j] = m[i][j] * d;
        }
    }
    r
}

/// `Multiply2Matrix(M1, M2)`: returns M1 * M2
pub fn mat_mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    r
}

/// Builds the view matrix (`hVGrads`) from three rotation angles in radians,
/// as done by `CalcVGradsFromHeader8rots` (rotate around X, then Y, then Z).
pub fn vgrads_from_angles(ax: f64, ay: f64, az: f64) -> Mat3 {
    let (s1, c1) = ax.sin_cos();
    let (s2, c2) = ay.sin_cos();
    let (s3, c3) = az.sin_cos();
    let mut m = [[0.0; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        let (mut x1, mut y1, mut z1) = (0.0, 0.0, 0.0);
        match i {
            0 => x1 = 1.0,
            1 => y1 = 1.0,
            _ => z1 = 1.0,
        }
        let x2 = x1;
        let y2 = c1 * y1 + s1 * z1;
        let z2 = c1 * z1 - s1 * y1;
        x1 = c2 * x2 + s2 * z2;
        y1 = y2;
        z1 = c2 * z2 - s2 * x2;
        *row = [c3 * x1 + s3 * y1, c3 * y1 - s3 * x1, z1];
    }
    m
}

/// `BuildViewVectorDFOV(xa, ya, v)`: v = normalize(-sin ya, sin xa, cos xa * cos ya)
#[inline]
pub fn build_view_vector_dfov(xa: f64, ya: f64) -> Vec3 {
    let (sx, cx) = xa.sin_cos();
    let (sy, cy) = ya.sin_cos();
    let v = [-sy, sx, cx * cy];
    let n = 1.0 / sqr_len(&v).sqrt();
    scale(v, n)
}

/// `BuildViewVectorDSphereFOV(xa, ya, v)`: panorama camera.
#[inline]
pub fn build_view_vector_dsphere_fov(xa: f64, ya: f64) -> Vec3 {
    let (sx, cx) = xa.sin_cos();
    let (sy, cy) = ya.sin_cos();
    [-sy * cx, sx, cx * cy]
}

pub fn ini_mat4() -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    m
}

/// `Multiply2SMatrix4(M1, M2)`: returns M1 * M2
pub fn mat4_mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut r = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            r[i][j] = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    r
}

/// `BuildSMatrix4`: 4D rotation from the XW, YW and ZW plane angles.
/// (Stored as single precision in MB3D; we keep the f32 rounding.)
pub fn build_smatrix4(xw: f64, yw: f64, zw: f64) -> Mat4 {
    let (sx, cx) = xw.sin_cos();
    let (sy, cy) = yw.sin_cos();
    let (sz, cz) = zw.sin_cos();
    let mut m = ini_mat4();
    m[0][0] = cx;
    m[3][3] = cx;
    m[0][3] = -sx;
    m[3][0] = sx;
    let mut m2 = ini_mat4();
    m2[1][1] = cy;
    m2[3][3] = cy;
    m2[1][3] = -sy;
    m2[3][1] = sy;
    m2 = mat4_mul(&m2, &m);
    let mut m3 = ini_mat4();
    m3[2][2] = cz;
    m3[3][3] = cz;
    m3[2][3] = -sz;
    m3[3][2] = sz;
    let mut r = mat4_mul(&m3, &m2);
    for row in r.iter_mut() {
        for v in row.iter_mut() {
            *v = *v as f32 as f64;
        }
    }
    r
}

/// `Rotate4Dex`: rotate the 3D vector (with w = 0) into 4D.
#[inline]
pub fn rotate_4dex(v: &Vec3, m: &Mat4) -> Vec4 {
    let mut r = [0.0; 4];
    for i in 0..4 {
        r[i] = v[0] * m[i][0] + v[1] * m[i][1] + v[2] * m[i][2];
    }
    r
}

/// `CreateXYVecsFromNormals`: two vectors orthogonal to the normal N.
pub fn create_xy_vecs_from_normals(n: &Vec3) -> (Vec3, Vec3) {
    let d = n[1] * n[1] + n[0] * n[0];
    if d < 1e-50 {
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])
    } else {
        let a = arccos_safe(-n[2] / (d + n[2] * n[2] + 1e-100).sqrt()) * 0.5;
        let (mut sin_a, cos_a) = a.sin_cos();
        sin_a /= d.sqrt();
        let dd = -n[1] * sin_a;
        let d1 = n[0] * sin_a;
        let vx = [1.0 - 2.0 * d1 * d1, 2.0 * dd * d1, 2.0 * d1 * cos_a];
        let vy = [vx[1], 1.0 - 2.0 * dd * dd, -2.0 * dd * cos_a];
        (vx, vy)
    }
}

#[inline]
pub fn arccos_safe(x: f64) -> f64 {
    x.clamp(-1.0, 1.0).acos()
}

#[inline]
pub fn arcsin_safe(x: f64) -> f64 {
    x.clamp(-1.0, 1.0).asin()
}

/// `MinMaxClip15bit`: clamp a value to 0..32767 and round to a word.
#[inline]
pub fn min_max_clip_15bit(s: f32) -> u16 {
    if s.is_nan() || s < 0.0 {
        0
    } else if s > 32767.0 {
        32767
    } else {
        s.round_ties_even() as u16
    }
}

/// Delphi's `Round` uses banker's rounding (round half to even).
#[inline]
pub fn delphi_round(x: f64) -> i64 {
    x.round_ties_even() as i64
}

/// `MakeSplineCoeff`: cubic B-spline weights.
pub fn make_spline_coeff(xs: f64) -> [f32; 4] {
    let r3 = xs * xs * xs / 6.0;
    let r0 = 1.0 / 6.0 + 0.5 * xs * (xs - 1.0) - r3;
    let r2 = xs + r0 - 2.0 * r3;
    let r1 = 1.0 - r0 - r2 - r3;
    [r0 as f32, r1 as f32, r2 as f32, r3 as f32]
}

/// `FastIntPow`: base^expo for expo a power of two.
#[inline]
pub fn fast_int_pow(base: f32, expo: i32) -> f32 {
    if base < 0.0 {
        return 0.0;
    }
    let mut r = base;
    let mut i = expo >> 1;
    while i > 0 {
        r *= r;
        i >>= 1;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_rotation() {
        let m = vgrads_from_angles(0.0, 0.0, 0.0);
        assert_eq!(m, IDENTITY3);
    }

    #[test]
    fn view_vector_center_is_forward() {
        let v = build_view_vector_dfov(0.0, 0.0);
        assert!((v[2] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn xy_vecs_orthogonal() {
        let n = normalize([0.3, -0.5, 0.8]);
        let (vx, vy) = create_xy_vecs_from_normals(&n);
        assert!(dot(&vx, &n).abs() < 1e-9);
        assert!(dot(&vy, &n).abs() < 1e-9);
        assert!(dot(&vx, &vy).abs() < 1e-9);
    }

    #[test]
    fn fast_pow() {
        assert!((fast_int_pow(0.9, 64) - 0.9f32.powi(64)).abs() < 1e-6);
    }
}
