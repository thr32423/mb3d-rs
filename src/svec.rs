//! Small vector helpers in single precision (MB3D's `TSVec`), shared by
//! the Monte Carlo renderer and the reflections.

use crate::math::Vec3;

pub(crate) type S3 = [f32; 3];

#[inline]
pub(crate) fn s3(v: &Vec3) -> S3 {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}
#[inline]
pub(crate) fn d3(v: S3) -> Vec3 {
    [v[0] as f64, v[1] as f64, v[2] as f64]
}
#[inline]
pub(crate) fn add3(a: S3, b: S3) -> S3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline]
pub(crate) fn sub3(a: S3, b: S3) -> S3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline]
pub(crate) fn mul3(a: S3, b: S3) -> S3 {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}
#[inline]
pub(crate) fn sc3(a: S3, s: f32) -> S3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
#[inline]
pub(crate) fn dot3(a: S3, b: S3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
/// `YofSVec` (negative results are 0)
#[inline]
pub(crate) fn luma(v: S3) -> f32 {
    (v[0] * 0.3 + v[1] * 0.59 + v[2] * 0.11).max(0.0)
}
/// `SVecPow`
#[inline]
pub(crate) fn spow(v: S3, e: f32) -> S3 {
    [v[0].powf(e), v[1].powf(e), v[2].powf(e)]
}
/// `LinInterpolate2SVecs(sv1, sv2, w1)` = sv1 * w1 + sv2 * (1 - w1)
#[inline]
pub(crate) fn lin2(sv1: S3, sv2: S3, w1: f32) -> S3 {
    [sv2[0] + w1 * (sv1[0] - sv2[0]), sv2[1] + w1 * (sv1[1] - sv2[1]), sv2[2] + w1 * (sv1[2] - sv2[2])]
}
/// `NormaliseVectorTo`
#[inline]
pub(crate) fn norm_to(n: f64, v: Vec3) -> Vec3 {
    let d = n / (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + 1e-100).sqrt();
    [v[0] * d, v[1] * d, v[2] * d]
}
