//! mb3d-rs as a C library (`libmb3d`): the functions of `include/mb3d.h`
//! over `mb3d::api`.
//!
//! The rules of every function here: pointers come from this library or
//! are null where the header allows it; texts are UTF-8 and end with a zero
//! byte; a failed call returns null (or -1) and `mb3d_last_error` tells why,
//! on the same thread.  A panic in the engine is caught and reported the
//! same way.

use engine::api::{self, Fractal, Picture, RenderOptions};
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
