//! The values of a scene an animation's curves show (an idea from
//! Mandelbulber, not in MB3D): each over all frames as it is interpolated,
//! with the keyframes' own values. The desktop's Curves window and the iPad
//! app draw them.

use crate::anim::Animation;
use crate::math::{build_rot_matrix, matrix_to_angles};
use crate::scene::Scene;

/// A value of a scene the curves can show: its label and how it is read
/// and written (None: the scene does not have it).
pub struct Param {
    /// a short name (for programs: the iPad app's texts)
    pub key: &'static str,
    /// the desktop's text label
    pub label: &'static str,
    pub get: fn(&Scene) -> Option<f64>,
    pub set: fn(&mut Scene, f64),
}

fn angle(s: &Scene, k: usize) -> Option<f64> {
    matrix_to_angles(&s.vgrads).map(|a| a[k].to_degrees())
}

fn set_angle(s: &mut Scene, k: usize, v: f64) {
    let mut a = matrix_to_angles(&s.vgrads).unwrap_or([0.0; 3]);
    a[k] = v.to_radians();
    s.vgrads = build_rot_matrix(a[0], a[1], a[2]);
}

pub const PARAMS: &[Param] = &[
    Param { key: "zoom", label: "@MB:Zoom2", get: |s| Some(s.zoom), set: |s, v| s.zoom = v.max(1e-30) },
    Param { key: "x", label: "@MB:PositionX", get: |s| Some(s.mid[0]), set: |s, v| s.mid[0] = v },
    Param { key: "y", label: "@MB:PositionY", get: |s| Some(s.mid[1]), set: |s, v| s.mid[1] = v },
    Param { key: "z", label: "@MB:PositionZ", get: |s| Some(s.mid[2]), set: |s, v| s.mid[2] = v },
    Param { key: "rotation_x", label: "@MB:RotationX", get: |s| angle(s, 0), set: |s, v| set_angle(s, 0, v) },
    Param { key: "rotation_y", label: "@MB:RotationY", get: |s| angle(s, 1), set: |s, v| set_angle(s, 1, v) },
    Param { key: "rotation_z", label: "@MB:RotationZ", get: |s| angle(s, 2), set: |s, v| set_angle(s, 2, v) },
    Param { key: "fov", label: "@MB:FOVy", get: |s| Some(s.fov_y), set: |s, v| s.fov_y = v.clamp(0.1, 360.0) },
    Param { key: "z_start", label: "@MB:ZStart", get: |s| Some(s.z_start), set: |s, v| s.z_start = v },
    Param {
        key: "ground", label: "@MB:GroundPosition",
        get: |s| s.ground.map(|g| g.pos),
        set: |s, v| {
            if let Some(g) = s.ground.as_mut() {
                g.pos = v
            }
        },
    },
    Param {
        key: "cloud_x", label: "@MB:CloudDriftX",
        get: |s| s.clouds.map(|c| c.offset[0]),
        set: |s, v| {
            if let Some(c) = s.clouds.as_mut() {
                c.offset[0] = v
            }
        },
    },
    Param {
        key: "cloud_y", label: "@MB:CloudDriftY",
        get: |s| s.clouds.map(|c| c.offset[1]),
        set: |s, v| {
            if let Some(c) = s.clouds.as_mut() {
                c.offset[1] = v
            }
        },
    },
    Param {
        key: "cloud_z", label: "@MB:CloudDriftZ",
        get: |s| s.clouds.map(|c| c.offset[2]),
        set: |s, v| {
            if let Some(c) = s.clouds.as_mut() {
                c.offset[2] = v
            }
        },
    },
];

/// The value `p` of every frame (NaN where a frame has none) and of every
/// keyframe, with the frame it is at.
pub fn curve(a: &Animation, p: &Param) -> (Vec<f64>, Vec<(usize, f64)>) {
    let n = a.frame_count();
    let mut values = Vec::with_capacity(n);
    for f in 0..n {
        let v = a.frame_scene(f).ok().and_then(|s| (p.get)(&s));
        values.push(v.unwrap_or(f64::NAN));
    }
    let mut keys = Vec::new();
    let mut f = 0usize;
    for k in &a.keyframes {
        keys.push((f, (p.get)(&k.scene).unwrap_or(f64::NAN)));
        f += k.frames as usize;
    }
    (values, keys)
}

/// Sets the value `p` of keyframe `k` (a keyframe without it, such as a
/// ground position without a ground, stays as it is).
pub fn set_key(a: &mut Animation, p: &Param, k: usize, v: f64) {
    if let Some(kf) = a.keyframes.get_mut(k) {
        if (p.get)(&kf.scene).is_some() {
            (p.set)(&mut kf.scene, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_value_reads_back_what_was_set() {
        let mut s = Scene::preset("Integer Power").unwrap();
        s.ground = Some(Default::default());
        s.clouds = Some(Default::default());
        for (i, p) in PARAMS.iter().enumerate() {
            let v = 12.5 + i as f64;
            (p.set)(&mut s, v);
            let got = (p.get)(&s).unwrap();
            assert!((got - v).abs() < 1e-6, "{}: {got} for {v}", p.label);
        }
        // a scene without a ground or clouds has none of their values
        let plain = Scene::preset("Integer Power").unwrap();
        assert!(PARAMS.iter().filter(|p| p.label.contains("Ground") || p.label.contains("Cloud")).all(|p| (p.get)(&plain).is_none()));
    }

    #[test]
    fn keys_are_unique() {
        let mut k: Vec<_> = PARAMS.iter().map(|p| p.key).collect();
        k.sort();
        k.dedup();
        assert_eq!(k.len(), PARAMS.len());
    }
}
