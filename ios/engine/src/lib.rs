//! mb3d-rs as a C library (`libmb3d`): the functions of `include/mb3d.h`
//! over `mb3d::api`.
//!
//! The rules of every function here: pointers come from this library or
//! are null where the header allows it; texts are UTF-8 and end with a zero
//! byte; a failed call returns null (or -1) and `mb3d_last_error` tells why,
//! on the same thread.  A panic in the engine is caught and reported the
//! same way.

use engine::api::{self, Film, Flight, Fractal, Mesh3D, MonteCarlo, Picture, RenderOptions};
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

thread_local! {
    static ERROR: RefCell<CString> = RefCell::new(CString::default());
}

fn set_error(e: &str) {
    let text = CString::new(e.replace('\0', " ")).unwrap_or_default();
    ERROR.with(|c| *c.borrow_mut() = text);
}

/// Runs `f`; an error or a panic becomes the thread's last error.
fn guard<T>(f: impl FnOnce() -> Result<T, String>) -> Option<T> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(v)) => Some(v),
        Ok(Err(e)) => {
            set_error(&e);
            None
        }
        Err(_) => {
            set_error("the engine failed unexpectedly (a bug in mb3d)");
            None
        }
    }
}

fn boxed<T>(v: Option<T>) -> *mut T {
    v.map_or(std::ptr::null_mut(), |v| Box::into_raw(Box::new(v)))
}

/// # Safety
/// `p` is null or a zero-terminated text.
unsafe fn text<'a>(p: *const c_char, what: &str) -> Result<&'a str, String> {
    if p.is_null() {
        return Err(format!("{what} is null"));
    }
    CStr::from_ptr(p).to_str().map_err(|_| format!("{what} is not UTF-8"))
}

fn scene_ref<'a>(p: *const Fractal) -> Result<&'a Fractal, String> {
    // SAFETY: a non-null scene pointer comes from this library (boxed) and
    // was not freed yet: the header's contract
    unsafe { p.as_ref() }.ok_or_else(|| "the scene is null".to_string())
}

/// This library's version, e.g. "0.1.0".  Not to be freed.
#[no_mangle]
pub extern "C" fn mb3d_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

/// The engine's own version (the mb3d-rs release this library was built from).
#[no_mangle]
pub extern "C" fn mb3d_engine_version() -> *const c_char {
    static V: std::sync::OnceLock<CString> = std::sync::OnceLock::new();
    V.get_or_init(|| CString::new(api::version()).unwrap_or_default()).as_ptr()
}

/// Why the last call on this thread failed.  Valid until the next failed
/// call on this thread; not to be freed.
#[no_mangle]
pub extern "C" fn mb3d_last_error() -> *const c_char {
    ERROR.with(|c| c.borrow().as_ptr())
}

/// Calculate on the graphics card (1) or the processor only (0, the start).
#[no_mangle]
pub extern "C" fn mb3d_use_gpu(on: c_int) {
    let _ = guard(|| {
        api::use_gpu(on != 0);
        Ok(())
    });
}

/// To call before the program ends when the graphics card was used: waits
/// until no formula is being compiled for the card any more.
#[no_mangle]
pub extern "C" fn mb3d_finish() {
    let _ = guard(|| {
        api::finish();
        Ok(())
    });
}

/// # Safety
/// `dir` is a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_add_formula_folder(dir: *const c_char) -> c_int {
    guard(|| {
        api::add_formula_folder(Path::new(text(dir, "the folder")?));
        Ok(())
    })
    .map_or(-1, |_| 0)
}

/// A further folder with map pictures.
///
/// # Safety
/// `dir` is a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_add_map_folder(dir: *const c_char) -> c_int {
    guard(|| {
        api::add_map_folder(Path::new(text(dir, "the folder")?));
        Ok(())
    })
    .map_or(-1, |_| 0)
}

/// # Safety
/// `path` is a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_open(path: *const c_char) -> *mut Fractal {
    boxed(guard(|| Fractal::open(Path::new(text(path, "the path")?))))
}

/// # Safety
/// `scene_text` is a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_open_text(scene_text: *const c_char) -> *mut Fractal {
    boxed(guard(|| Fractal::from_text(text(scene_text, "the text")?)))
}

/// # Safety
/// `name` is a zero-terminated text; `data` points to `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn mb3d_open_bytes(name: *const c_char, data: *const u8, len: usize) -> *mut Fractal {
    boxed(guard(|| {
        if data.is_null() {
            return Err("the data is null".into());
        }
        Fractal::from_bytes(text(name, "the name")?, std::slice::from_raw_parts(data, len).to_vec())
    }))
}

/// # Safety
/// `formula` is a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_preset(formula: *const c_char) -> *mut Fractal {
    boxed(guard(|| Fractal::preset(text(formula, "the formula")?)))
}

/// # Safety
/// `scene` is a scene of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_scene_copy(scene: *const Fractal) -> *mut Fractal {
    boxed(guard(|| Ok(scene_ref(scene)?.clone())))
}

/// # Safety
/// `scene` is null or a scene of this library, not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn mb3d_scene_free(scene: *mut Fractal) {
    if !scene.is_null() {
        drop(Box::from_raw(scene));
    }
}

/// # Safety
/// `scene` is a scene of this library, `changes` a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_scene_set(scene: *mut Fractal, changes: *const c_char) -> c_int {
    guard(|| scene.as_mut().ok_or("the scene is null")?.set(text(changes, "the changes")?)).map_or(-1, |_| 0)
}

/// Changes one of a formula's own values by name; `slot` is the formula's
/// place in the scene (0 for the first).
///
/// # Safety
/// `scene` is a scene of this library, `key` a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_scene_set_formula(scene: *mut Fractal, slot: u32, key: *const c_char, value: f64) -> c_int {
    guard(|| scene.as_mut().ok_or("the scene is null")?.set_formula(slot as usize, &[(text(key, "the key")?, value)], None)).map_or(-1, |_| 0)
}

/// The scene as `key = value` lines; free it with `mb3d_string_free`.
///
/// # Safety
/// `scene` is a scene of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_scene_text(scene: *const Fractal) -> *mut c_char {
    guard(|| CString::new(scene_ref(scene)?.text()).map_err(|e| e.to_string())).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// # Safety
/// `s` is null or a text this library returned for freeing.
#[no_mangle]
pub unsafe extern "C" fn mb3d_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// # Safety
/// `scene` is a scene of this library; `width` and `height` are null or
/// point to a number each.
#[no_mangle]
pub unsafe extern "C" fn mb3d_scene_size(scene: *const Fractal, width: *mut u32, height: *mut u32) -> c_int {
    guard(|| {
        let (w, h) = scene_ref(scene)?.size();
        if let Some(p) = width.as_mut() {
            *p = w as u32;
        }
        if let Some(p) = height.as_mut() {
            *p = h as u32;
        }
        Ok(())
    })
    .map_or(-1, |_| 0)
}

/// # Safety
/// `scene` is a scene of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_scene_resize(scene: *mut Fractal, width: u32, height: u32) -> c_int {
    guard(|| {
        scene.as_mut().ok_or("the scene is null")?.resize(width as usize, height as usize);
        Ok(())
    })
    .map_or(-1, |_| 0)
}

/// The view as the three angles of the `rotation` key (degrees) into
/// `angles[3]`; -1 for a view they cannot give back.
///
/// # Safety
/// `scene` is a scene of this library; `angles` points to three doubles.
#[no_mangle]
pub unsafe extern "C" fn mb3d_scene_rotation(scene: *const Fractal, angles: *mut f64) -> c_int {
    guard(|| {
        let r = scene_ref(scene)?.rotation().ok_or("the view has no such angles")?;
        if angles.is_null() {
            return Err("angles is null".into());
        }
        std::slice::from_raw_parts_mut(angles, 3).copy_from_slice(&r);
        Ok(())
    })
    .map_or(-1, |_| 0)
}

/// Told the stage ("" during the march, then "hard shadows", ...) and its
/// work done and to do; returns non-zero to stop the calculation.  Called
/// from the engine's threads, one call at a time.
pub type Mb3dProgress = Option<unsafe extern "C" fn(user: *mut c_void, stage: *const c_char, done: usize, total: usize) -> c_int>;

/// The caller's pointer, handed back to its own function.
struct User(*mut c_void);
// SAFETY: the pointer is only passed back to the caller's progress
// function, under a lock (one call at a time): the header says it is called
// from other threads
unsafe impl Send for User {}

/// Calculates the picture; null when it failed or was stopped
/// (`mb3d_last_error`: "cancelled").
///
/// # Safety
/// `scene` is a scene of this library; `progress` is null or a function
/// that may be called from other threads with `user`.
#[no_mangle]
pub unsafe extern "C" fn mb3d_render(scene: *const Fractal, progress: Mb3dProgress, user: *mut c_void) -> *mut Picture {
    boxed(guard(|| {
        let f = scene_ref(scene)?;
        let stop = AtomicBool::new(false);
        let user = Mutex::new(User(user));
        let report = |stage: &str, done: usize, total: usize| {
            let (Some(call), Ok(u), Ok(stage)) = (progress, user.lock(), CString::new(stage)) else { return };
            // SAFETY: the caller's function with the caller's pointer
            if unsafe { call(u.0, stage.as_ptr(), done, total) } != 0 {
                stop.store(true, Ordering::SeqCst);
            }
        };
        let cancel = || stop.load(Ordering::SeqCst);
        f.render(&RenderOptions { progress: Some(&report), cancel: Some(&cancel) })
    }))
}

fn picture_ref<'a>(p: *const Picture) -> Option<&'a Picture> {
    // SAFETY: a non-null picture pointer comes from mb3d_render and was not
    // freed yet: the header's contract
    unsafe { p.as_ref() }
}

/// # Safety
/// `picture` is null or a picture of this library (so for all `mb3d_picture_*`).
#[no_mangle]
pub unsafe extern "C" fn mb3d_picture_width(picture: *const Picture) -> u32 {
    picture_ref(picture).map_or(0, |p| p.width as u32)
}

/// # Safety
/// See `mb3d_picture_width`.
#[no_mangle]
pub unsafe extern "C" fn mb3d_picture_height(picture: *const Picture) -> u32 {
    picture_ref(picture).map_or(0, |p| p.height as u32)
}

/// width * height * 3 bytes: red, green, blue, rows from the top.  Valid
/// until the picture is freed.
///
/// # Safety
/// See `mb3d_picture_width`.
#[no_mangle]
pub unsafe extern "C" fn mb3d_picture_rgb(picture: *const Picture) -> *const u8 {
    picture_ref(picture).map_or(std::ptr::null(), |p| p.rgb.as_ptr())
}

/// width * height numbers: 65535 nearest, 0 the background.
///
/// # Safety
/// See `mb3d_picture_width`.
#[no_mangle]
pub unsafe extern "C" fn mb3d_picture_depth(picture: *const Picture) -> *const u16 {
    picture_ref(picture).map_or(std::ptr::null(), |p| p.depth.as_ptr())
}

/// # Safety
/// See `mb3d_picture_width`.
#[no_mangle]
pub unsafe extern "C" fn mb3d_picture_seconds(picture: *const Picture) -> f64 {
    picture_ref(picture).map_or(0.0, |p| p.seconds)
}

/// Writes the picture as a PNG file.
///
/// # Safety
/// See `mb3d_picture_width`; `path` is a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_picture_save_png(picture: *const Picture, path: *const c_char) -> c_int {
    guard(|| {
        let p = picture_ref(picture).ok_or("the picture is null")?;
        let path = text(path, "the path")?;
        std::fs::write(path, p.png()).map_err(|e| format!("{path}: {e}"))
    })
    .map_or(-1, |_| 0)
}

/// # Safety
/// `picture` is null or a picture of this library, not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn mb3d_picture_free(picture: *mut Picture) {
    if !picture.is_null() {
        drop(Box::from_raw(picture));
    }
}

/// The requests of `mb3d serve` (mb3d-rs: docs/API.md) without the program:
/// a session keeps the scenes its requests open.
pub struct Session {
    server: Mutex<engine::serve::Server>,
    /// counts the calls of `mb3d_session_cancel`
    cancels: AtomicU64,
}

/// Told every line that comes before a request's answer (progress), as JSON.
/// Called from the engine's threads, one call at a time.
pub type Mb3dEvent = Option<unsafe extern "C" fn(user: *mut c_void, json: *const c_char)>;

/// A new session; free it with `mb3d_session_free`.
#[no_mangle]
pub extern "C" fn mb3d_session_new() -> *mut Session {
    boxed(guard(|| Ok(Session { server: Mutex::default(), cancels: AtomicU64::new(0) })))
}

/// One request of `mb3d serve` as JSON (`{"op":"open","preset":"Amazing Box"}`);
/// the answer as JSON, to free with `mb3d_string_free`.  A request that
/// failed answers `{"ok":false,"error":...}`; null only for a request that
/// is not JSON (`mb3d_last_error`).  One request at a time: a second thread
/// waits.
///
/// # Safety
/// `session` is a session of this library, `request` a zero-terminated
/// text; `event` is null or a function that may be called from other
/// threads with `user`.
#[no_mangle]
pub unsafe extern "C" fn mb3d_session_request(session: *const Session, request: *const c_char, event: Mb3dEvent, user: *mut c_void) -> *mut c_char {
    guard(|| {
        let session = session.as_ref().ok_or("the session is null")?;
        let request = engine::json::parse(text(request, "the request")?)?;
        let user = Mutex::new(User(user));
        let emit = |line: engine::json::Value| {
            let (Some(call), Ok(u), Ok(line)) = (event, user.lock(), CString::new(line.to_json())) else { return };
            // SAFETY: the caller's function with the caller's pointer
            unsafe { call(u.0, line.as_ptr()) }
        };
        let before = session.cancels.load(Ordering::SeqCst);
        let cancelled = || session.cancels.load(Ordering::SeqCst) != before;
        // (a request that panicked leaves the scenes as they were: go on)
        let mut server = session.server.lock().unwrap_or_else(|e| e.into_inner());
        CString::new(server.handle(&request, &emit, &cancelled).to_json()).map_err(|e| e.to_string())
    })
    .map_or(std::ptr::null_mut(), CString::into_raw)
}

/// Stops the session's `render` that is running (from another thread): it
/// answers "cancelled".
///
/// # Safety
/// `session` is null or a session of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_session_cancel(session: *const Session) {
    if let Some(s) = session.as_ref() {
        s.cancels.fetch_add(1, Ordering::SeqCst);
    }
}

/// # Safety
/// `session` is null or a session of this library, not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn mb3d_session_free(session: *mut Session) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}

// ---------------------------------------------------------------------------
// the navigator (iPad app): a camera flying through a scene

fn flight_mut<'a>(p: *mut Flight) -> Option<&'a mut Flight> {
    // SAFETY: a non-null navigator pointer comes from this library (boxed)
    // and was not freed yet: the header's contract
    unsafe { p.as_mut() }
}

/// A camera starting at the scene's view; free it with mb3d_navigator_free.
///
/// # Safety
/// `scene` is null or a scene of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_navigator_new(scene: *const Fractal) -> *mut Flight {
    boxed(guard(|| Ok(scene_ref(scene)?.flight())))
}

/// # Safety
/// `nav` is null or a navigator of this library, not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn mb3d_navigator_free(nav: *mut Flight) {
    if !nav.is_null() {
        drop(Box::from_raw(nav));
    }
}

/// Turns the camera where it stands, in degrees.
///
/// # Safety
/// `nav` is a navigator of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_navigator_turn(nav: *mut Flight, pitch: f64, yaw: f64, roll: f64) {
    if let Some(f) = flight_mut(nav) {
        let _ = guard(|| Ok(f.turn(pitch, yaw, roll)));
    }
}

/// Slides sideways by a drag of (dx, dy) picture heights.
///
/// # Safety
/// `nav` is a navigator of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_navigator_slide(nav: *mut Flight, dx: f64, dy: f64) {
    if let Some(f) = flight_mut(nav) {
        let _ = guard(|| Ok(f.slide(dx, dy)));
    }
}

/// Flies forward (backward when negative) by that percentage of the
/// distance to the surface.
///
/// # Safety
/// `nav` is a navigator of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_navigator_fly(nav: *mut Flight, percent: f64) {
    if let Some(f) = flight_mut(nav) {
        let _ = guard(|| Ok(f.fly(percent)));
    }
}

/// The field of view in degrees.
///
/// # Safety
/// `nav` is a navigator of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_navigator_fov(nav: *const Flight) -> f64 {
    // SAFETY: as flight_mut
    unsafe { nav.as_ref() }.map_or(0.0, |f| f.fov())
}

/// The scene's directions of the picture's right, down and depth (each of
/// length 1) into out[9], row after row: a compass.
///
/// # Safety
/// `nav` is a navigator of this library; `out` points to 9 numbers.
#[no_mangle]
pub unsafe extern "C" fn mb3d_navigator_axes(nav: *const Flight, out: *mut f64) {
    // SAFETY: as flight_mut
    let Some(f) = (unsafe { nav.as_ref() }) else { return };
    if out.is_null() {
        return;
    }
    for (i, v) in f.axes().iter().flatten().enumerate() {
        // SAFETY: `out` holds 9 numbers (the header's contract)
        unsafe { *out.add(i) = *v };
    }
}

/// A new scene of the view: quick != 0 the quick one while moving, else
/// the full one; free it with mb3d_scene_free.
///
/// # Safety
/// `nav` is a navigator of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_navigator_scene(nav: *const Flight, quick: c_int, width: u32, height: u32) -> *mut Fractal {
    boxed(guard(|| {
        // SAFETY: as flight_mut
        let f = unsafe { nav.as_ref() }.ok_or_else(|| "the navigator is null".to_string())?;
        let (w, h) = (width as usize, height as usize);
        Ok(if quick != 0 { f.preview(w, h) } else { f.view(w, h) })
    }))
}

// formulas and files (iPad app): the formula picker, saving a scene

fn owned_text(s: String) -> *mut c_char {
    CString::new(s.replace('\0', " ")).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// The names of all formulas, one per line (the built-in ones, then those
/// of the formula folders); free it with mb3d_string_free.
#[no_mangle]
pub extern "C" fn mb3d_formula_names() -> *mut c_char {
    guard(|| Ok(api::formula_names().join("\n"))).map_or(std::ptr::null_mut(), owned_text)
}

/// A formula's settings with their default values, as `key = value` lines
/// of a `[formula]` section; free it with mb3d_string_free.
///
/// # Safety
/// `name` is a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_formula_settings(name: *const c_char) -> *mut c_char {
    guard(|| {
        let v = api::formula_settings(text(name, "the name")?)?;
        Ok(v.iter().map(|(k, x)| format!("{k} = {x}\n")).collect::<String>())
    })
    .map_or(std::ptr::null_mut(), owned_text)
}

/// Writes the scene as a Mandelbulb3D parameter file (.m3p) at `path`.
///
/// # Safety
/// `scene` is a scene of this library; `path` a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_scene_save_m3p(scene: *const Fractal, path: *const c_char) -> c_int {
    guard(|| {
        let s = scene_ref(scene)?;
        let path = text(path, "the path")?;
        std::fs::write(path, s.m3p()).map_err(|e| format!("{path}: {e}"))
    })
    .map_or(-1, |_| 0)
}

/// The parameters as Mandelbulb3D text (`Mandelbulb3Dv18{...}`) for the
/// clipboard; free it with mb3d_string_free.
///
/// # Safety
/// `scene` is a scene of this library; `title` a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_scene_mb3d_text(scene: *const Fractal, title: *const c_char) -> *mut c_char {
    guard(|| Ok(scene_ref(scene)?.mb3d_text(text(title, "the title")?))).map_or(std::ptr::null_mut(), owned_text)
}

/// The depth at pixel (x, y) of the scene's picture as `dof_focus` takes
/// it, into `focus`: 0, or -1 on the background (or a failure).
///
/// # Safety
/// `scene` is a scene of this library; `focus` points to a number.
#[no_mangle]
pub unsafe extern "C" fn mb3d_scene_pick(scene: *const Fractal, x: u32, y: u32, focus: *mut f64) -> c_int {
    guard(|| scene_ref(scene)?.pick(x as usize, y as usize).ok_or_else(|| "the background".to_string()))
        .map_or(-1, |(_, z)| {
            // SAFETY: the header's contract
            if let Some(f) = unsafe { focus.as_mut() } {
                *f = z;
            }
            0
        })
}

/// MutaGen: 15 variations of the scene into `out` (15 scene pointers, each
/// freed with mb3d_scene_free; the first is the scene itself); progress as
/// mb3d_render's (non-zero from it stops). 0, or -1 (nothing in `out`).
///
/// # Safety
/// `scene` is a scene of this library; `out` points to 15 pointers.
#[no_mangle]
pub unsafe extern "C" fn mb3d_variations(
    scene: *const Fractal,
    seed: u64,
    out: *mut *mut Fractal,
    progress: Mb3dProgress,
    user: *mut c_void,
) -> c_int {
    // SAFETY: the same contract
    unsafe { mb3d_variations_strength(scene, seed, 1.0, out, progress, user) }
}

/// mb3d_variations with the changes `strength` times MutaGen's own (0.1
/// to 3).
///
/// # Safety
/// `scene` is a scene of this library; `out` points to 15 pointers.
#[no_mangle]
pub unsafe extern "C" fn mb3d_variations_strength(
    scene: *const Fractal,
    seed: u64,
    strength: f64,
    out: *mut *mut Fractal,
    progress: Mb3dProgress,
    user: *mut c_void,
) -> c_int {
    guard(|| {
        let f = scene_ref(scene)?;
        if out.is_null() {
            return Err("out is null".into());
        }
        let stop = AtomicBool::new(false);
        let user = Mutex::new(User(user));
        let report = |stage: &str, done: usize, total: usize| {
            let (Some(call), Ok(u), Ok(stage)) = (progress, user.lock(), CString::new(stage)) else { return };
            // SAFETY: the caller's function with the caller's pointer
            if unsafe { call(u.0, stage.as_ptr(), done, total) } != 0 {
                stop.store(true, Ordering::SeqCst);
            }
        };
        let cancel = || stop.load(Ordering::SeqCst);
        let v = f.variations_with(seed, strength, &RenderOptions { progress: Some(&report), cancel: Some(&cancel) })?;
        for (i, (_, g)) in v.into_iter().take(15).enumerate() {
            // SAFETY: `out` holds 15 pointers (the header's contract)
            unsafe { *out.add(i) = Box::into_raw(Box::new(g)) };
        }
        Ok(())
    })
    .map_or(-1, |_| 0)
}

// Monte Carlo (iPad app): pass after pass, the picture between them

/// A Monte Carlo render of the scene (no pass yet); free it with
/// mb3d_montecarlo_free.
///
/// # Safety
/// `scene` is a scene of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_montecarlo_new(scene: *const Fractal) -> *mut MonteCarlo {
    boxed(guard(|| Ok(scene_ref(scene)?.monte_carlo())))
}

/// # Safety
/// `mc` is null or a Monte Carlo render of this library, not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn mb3d_montecarlo_free(mc: *mut MonteCarlo) {
    if !mc.is_null() {
        // SAFETY: from mb3d_montecarlo_new (boxed), freed once
        drop(unsafe { Box::from_raw(mc) });
    }
}

/// One more pass; progress as mb3d_render's (non-zero from it stops). 0, or
/// -1 (stopped or failed).
///
/// # Safety
/// `mc` is a Monte Carlo render of this library, used by one thread at a time.
#[no_mangle]
pub unsafe extern "C" fn mb3d_montecarlo_pass(mc: *mut MonteCarlo, progress: Mb3dProgress, user: *mut c_void) -> c_int {
    guard(|| {
        // SAFETY: the header's contract
        let m = unsafe { mc.as_mut() }.ok_or_else(|| "the Monte Carlo render is null".to_string())?;
        let stop = AtomicBool::new(false);
        let user = Mutex::new(User(user));
        let report = |stage: &str, done: usize, total: usize| {
            let (Some(call), Ok(u), Ok(stage)) = (progress, user.lock(), CString::new(stage)) else { return };
            // SAFETY: the caller's function with the caller's pointer
            if unsafe { call(u.0, stage.as_ptr(), done, total) } != 0 {
                stop.store(true, Ordering::SeqCst);
            }
        };
        let cancel = || stop.load(Ordering::SeqCst);
        m.pass(&RenderOptions { progress: Some(&report), cancel: Some(&cancel) })
    })
    .map_or(-1, |_| 0)
}

/// The passes done.
///
/// # Safety
/// `mc` is a Monte Carlo render of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_montecarlo_passes(mc: *const MonteCarlo) -> u32 {
    // SAFETY: the header's contract
    unsafe { mc.as_ref() }.map_or(0, |m| m.passes())
}

/// The picture so far; free it with mb3d_picture_free.
///
/// # Safety
/// `mc` is a Monte Carlo render of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_montecarlo_picture(mc: *const MonteCarlo) -> *mut Picture {
    // SAFETY: the header's contract
    boxed(guard(|| unsafe { mc.as_ref() }.map(|m| m.picture()).ok_or_else(|| "the Monte Carlo render is null".to_string())))
}

// bytes (iPad app): the 360 photo

fn owned_bytes(v: Vec<u8>, len: *mut usize) -> *mut u8 {
    let b = v.into_boxed_slice();
    // SAFETY: the header's contract (`len` points to a number)
    if let Some(l) = unsafe { len.as_mut() } {
        *l = b.len();
    }
    Box::into_raw(b).cast()
}

/// A JPEG of a `w` × `h` equirectangular picture with the tags of a 360
/// photo (Photo Sphere XMP, the 360 camera's EXIF), as the Share window
/// writes it; its length into `out_len`; free with mb3d_bytes_free.
///
/// # Safety
/// `jpeg` points to `len` bytes; `out_len` to a number.
#[no_mangle]
pub unsafe extern "C" fn mb3d_jpeg_360(jpeg: *const u8, len: usize, w: u32, h: u32, out_len: *mut usize) -> *mut u8 {
    guard(|| {
        if jpeg.is_null() {
            return Err("the JPEG is null".into());
        }
        // SAFETY: the header's contract
        let data = unsafe { std::slice::from_raw_parts(jpeg, len) };
        engine::vr::jpeg_with_pano(data, w as usize, h as usize)
    })
    .map_or(std::ptr::null_mut(), |v| owned_bytes(v, out_len))
}

/// Frees bytes this library returned (with their length).
///
/// # Safety
/// `p` is null or bytes of this library of `len` bytes, not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn mb3d_bytes_free(p: *mut u8, len: usize) {
    if !p.is_null() {
        // SAFETY: from owned_bytes with this length, freed once
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(p, len)) });
    }
}

// animation (iPad app): keyframes and the frames between them

fn film_ref<'a>(p: *const Film) -> Result<&'a Film, String> {
    // SAFETY: a non-null film pointer comes from this library (boxed) and
    // was not freed yet: the header's contract
    unsafe { p.as_ref() }.ok_or_else(|| "the film is null".to_string())
}

/// An animation without keyframes: `looped` != 0 goes back to the first,
/// `interpolation` 0 linear, 1 Bezier, 2 Catmull-Rom.
#[no_mangle]
pub extern "C" fn mb3d_film_new(looped: c_int, interpolation: c_int) -> *mut Film {
    boxed(guard(|| Ok(Film::new(looped != 0, interpolation))))
}

/// A turntable around the scene's object: `steps` keyframes, `frames`
/// frames from each to the next.
///
/// # Safety
/// `scene` is a scene of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_film_orbit(scene: *const Fractal, steps: u32, frames: u32) -> *mut Film {
    boxed(guard(|| Ok(Film::orbit(scene_ref(scene)?, steps as usize, frames))))
}

/// # Safety
/// `film` is null or a film of this library, not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn mb3d_film_free(film: *mut Film) {
    if !film.is_null() {
        // SAFETY: from this library (boxed), freed once
        drop(unsafe { Box::from_raw(film) });
    }
}

/// Adds a keyframe (a copy of `scene`) with `frames` frames to the next.
///
/// # Safety
/// `film` is a film and `scene` a scene of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_film_add(film: *mut Film, scene: *const Fractal, frames: u32) -> c_int {
    guard(|| {
        // SAFETY: as film_ref, and only this thread uses it
        let f = unsafe { film.as_mut() }.ok_or_else(|| "the film is null".to_string())?;
        f.add(scene_ref(scene)?, frames);
        Ok(())
    })
    .map_or(-1, |_| 0)
}

/// # Safety
/// `film` is a film of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_film_keyframes(film: *const Film) -> u32 {
    film_ref(film).map_or(0, |f| f.keyframes() as u32)
}

/// A copy of keyframe `i`; free it with mb3d_scene_free.
///
/// # Safety
/// `film` is a film of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_film_keyframe(film: *const Film, i: u32) -> *mut Fractal {
    boxed(guard(|| film_ref(film)?.keyframe(i as usize)))
}

/// # Safety
/// `film` is a film of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_film_frames(film: *const Film) -> u32 {
    film_ref(film).map_or(0, |f| f.frames() as u32)
}

/// The scene of frame `i` (from 0); free it with mb3d_scene_free.
///
/// # Safety
/// `film` is a film of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_film_frame(film: *const Film, i: u32) -> *mut Fractal {
    boxed(guard(|| film_ref(film)?.frame(i as usize)))
}

/// The scene `offset` (0..1) of the way from frame `i` to the next (the
/// pictures of a motion blur); free it with mb3d_scene_free.
///
/// # Safety
/// `film` is a film of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_film_frame_at(film: *const Film, i: u32, offset: f64) -> *mut Fractal {
    boxed(guard(|| film_ref(film)?.frame_at(i as usize, offset)))
}

/// The short names of the values the curves show, one per line (an index
/// into them is a curve's `param`); free with mb3d_string_free.
#[no_mangle]
pub extern "C" fn mb3d_curve_names() -> *mut c_char {
    guard(|| Ok(Film::curve_names().join("\n"))).map_or(std::ptr::null_mut(), owned_text)
}

/// Curve `param`: its value at every frame into `values` (room for
/// mb3d_film_frames; NaN where a frame has none), at every keyframe into
/// `key_values` and the keyframe's frame into `key_frames` (room for
/// mb3d_film_keyframes each). 0, or -1.
///
/// # Safety
/// `film` is a film of this library; the arrays have the room above.
#[no_mangle]
pub unsafe extern "C" fn mb3d_film_curve(
    film: *const Film,
    param: u32,
    values: *mut f64,
    key_frames: *mut u32,
    key_values: *mut f64,
) -> c_int {
    guard(|| {
        let f = film_ref(film)?;
        if values.is_null() || key_frames.is_null() || key_values.is_null() {
            return Err("an array is null".into());
        }
        let (v, keys) = f.curve(param as usize).ok_or_else(|| format!("there is no curve {param}"))?;
        for (i, x) in v.into_iter().take(f.frames()).enumerate() {
            // SAFETY: `values` has room for every frame (the header's contract)
            unsafe { *values.add(i) = x };
        }
        for (i, (frame, x)) in keys.into_iter().take(f.keyframes()).enumerate() {
            // SAFETY: the key arrays have room for every keyframe
            unsafe {
                *key_frames.add(i) = frame as u32;
                *key_values.add(i) = x;
            }
        }
        Ok(())
    })
    .map_or(-1, |_| 0)
}

/// Sets value `param` of keyframe `key` (see mb3d_film_keyframe).
///
/// # Safety
/// `film` is a film of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_film_set_curve_key(film: *mut Film, param: u32, key: u32, value: f64) -> c_int {
    guard(|| {
        // SAFETY: as film_ref, and only this thread uses it
        let f = unsafe { film.as_mut() }.ok_or_else(|| "the film is null".to_string())?;
        f.set_curve_key(param as usize, key as usize, value);
        Ok(())
    })
    .map_or(-1, |_| 0)
}

// 3D model (iPad app): the BulbTracer's mesh

fn mesh_ref<'a>(p: *const Mesh3D) -> Option<&'a Mesh3D> {
    // SAFETY: a non-null mesh pointer comes from mb3d_mesh_trace and was
    // not freed yet: the header's contract
    unsafe { p.as_ref() }
}

/// The object as a triangle mesh: a `resolution`³ grid, colours from the
/// palette when `colors` != 0, `smooth` smoothing passes, the largest side
/// `size` long and centred; progress as mb3d_render's.
///
/// # Safety
/// `scene` is a scene of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_mesh_trace(
    scene: *const Fractal,
    resolution: u32,
    colors: c_int,
    smooth: u32,
    size: f64,
    progress: Mb3dProgress,
    user: *mut c_void,
) -> *mut Mesh3D {
    boxed(guard(|| {
        let f = scene_ref(scene)?;
        let stop = AtomicBool::new(false);
        let user = Mutex::new(User(user));
        let report = |stage: &str, done: usize, total: usize| {
            let (Some(call), Ok(u), Ok(stage)) = (progress, user.lock(), CString::new(stage)) else { return };
            // SAFETY: the caller's function with the caller's pointer
            if unsafe { call(u.0, stage.as_ptr(), done, total) } != 0 {
                stop.store(true, Ordering::SeqCst);
            }
        };
        let cancel = || stop.load(Ordering::SeqCst);
        f.mesh(resolution as usize, colors != 0, smooth, size, &RenderOptions { progress: Some(&report), cancel: Some(&cancel) })
    }))
}

/// # Safety
/// `mesh` is null or a mesh of this library, not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn mb3d_mesh_free(mesh: *mut Mesh3D) {
    if !mesh.is_null() {
        // SAFETY: from mb3d_mesh_trace (boxed), freed once
        drop(unsafe { Box::from_raw(mesh) });
    }
}

/// # Safety
/// `mesh` is a mesh of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_mesh_vertex_count(mesh: *const Mesh3D) -> usize {
    mesh_ref(mesh).map_or(0, |m| m.vertices().len())
}

/// # Safety
/// `mesh` is a mesh of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_mesh_face_count(mesh: *const Mesh3D) -> usize {
    mesh_ref(mesh).map_or(0, |m| m.faces().len())
}

/// x, y, z of each vertex; valid while the mesh lives.
///
/// # Safety
/// `mesh` is a mesh of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_mesh_vertices(mesh: *const Mesh3D) -> *const f32 {
    mesh_ref(mesh).map_or(std::ptr::null(), |m| m.vertices().as_ptr().cast())
}

/// The normal of each vertex (x, y, z); valid while the mesh lives.
///
/// # Safety
/// `mesh` is a mesh of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_mesh_normals(mesh: *const Mesh3D) -> *const f32 {
    mesh_ref(mesh).map_or(std::ptr::null(), |m| m.normals().as_ptr().cast())
}

/// Red, green, blue (0..1) and the palette position of each vertex, or
/// null without colours; valid while the mesh lives.
///
/// # Safety
/// `mesh` is a mesh of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_mesh_colors(mesh: *const Mesh3D) -> *const f32 {
    mesh_ref(mesh).and_then(|m| m.colors()).map_or(std::ptr::null(), |c| c.as_ptr().cast())
}

/// Three vertex numbers (from 0) per triangle; valid while the mesh lives.
///
/// # Safety
/// `mesh` is a mesh of this library.
#[no_mangle]
pub unsafe extern "C" fn mb3d_mesh_faces(mesh: *const Mesh3D) -> *const u32 {
    mesh_ref(mesh).map_or(std::ptr::null(), |m| m.faces().as_ptr().cast())
}

/// Writes the mesh as OBJ, PLY or STL (by the path's extension).
///
/// # Safety
/// `mesh` is a mesh of this library; `path` a zero-terminated text.
#[no_mangle]
pub unsafe extern "C" fn mb3d_mesh_save(mesh: *const Mesh3D, path: *const c_char) -> c_int {
    guard(|| mesh_ref(mesh).ok_or("the mesh is null")?.save(Path::new(text(path, "the path")?)))
        .map_or(-1, |_| 0)
}
