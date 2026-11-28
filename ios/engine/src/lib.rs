//! The engine's C functions (capi/, `capi/include/mb3d.h`) as the iPad
//! app's static library: nothing of its own, so the app and mb3d-bindings'
//! libmb3d cannot drift apart.

pub use mb3d_capi::*;
