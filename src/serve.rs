//! `mb3d serve`: the engine for programs in other languages.  A program
//! starts `mb3d serve` and sends it requests, a line of JSON each, on the
//! standard input; the answers come as lines of JSON on the standard
//! output.  The process stays, with its scenes, the graphics card and the
//! compiled formulas, so a second picture starts at once.  No network:
//! nothing listens anywhere.  docs/API.md describes the requests.
//!
//! EXPERIMENTAL like [`crate::api`], which it is built on: `hello` tells
//! the protocol number, raised when a request changes its meaning.

use crate::api::{self, Fractal, RenderOptions};
use crate::json::{self, obj, s, Value};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// Raised when a request or an answer changes its meaning.
pub const PROTOCOL: u32 = 1;

/// The scenes a program has open.
#[derive(Default)]
pub struct Server {
    scenes: HashMap<u64, Fractal>,
    next: u64,
}

fn num(v: f64) -> Value {
    Value::Num(v)
}

fn arg_str<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str)
}

fn arg_bool(a: &Value, k: &str) -> bool {
    a.get(k).and_then(Value::as_bool).unwrap_or(false)
}

fn arg_size(a: &Value, k: &str) -> Result<Option<usize>, String> {
    match a.get(k).and_then(Value::as_f64) {
        None => Ok(None),
        Some(v) if (16.0..=32768.0).contains(&v) => Ok(Some(v.round() as usize)),
        Some(v) => Err(format!("{k} = {v}: a picture is 16 to 32768 pixels wide and high")),
    }
}

fn write_file(path: &str, data: &[u8]) -> Result<(), String> {
    std::fs::write(path, data).map_err(|e| format!("{path}: {e}"))
}

fn u16_bytes(v: &[u16]) -> Vec<u8> {
    v.iter().flat_map(|d| d.to_le_bytes()).collect()
}

impl Server {
    fn scene(&self, a: &Value) -> Result<(u64, &Fractal), String> {
        let id = a.get("scene").and_then(Value::as_f64).ok_or("scene is missing (the number `open` gave)")? as u64;
        self.scenes.get(&id).map(|f| (id, f)).ok_or_else(|| format!("no scene {id} (closed, or never opened)"))
    }

    fn about(id: u64, f: &Fractal) -> Vec<(String, Value)> {
        let (w, h) = f.size();
        let formulas = f.formulas().into_iter().map(|(n, i)| obj([("name", s(n)), ("iterations", num(i as f64))])).collect();
        let rotation = f.rotation().map_or(Value::Null, |r| Value::Arr(r.iter().map(|a| num((a * 1e6).round() / 1e6)).collect()));
        vec![
            ("scene".into(), num(id as f64)),
            ("width".into(), num(w as f64)),
            ("height".into(), num(h as f64)),
            ("formulas".into(), Value::Arr(formulas)),
            ("rotation".into(), rotation),
            ("notes".into(), Value::Arr(f.notes().iter().map(s).collect())),
        ]
    }

    fn add(&mut self, f: Fractal) -> Vec<(String, Value)> {
        self.next += 1;
        let r = Server::about(self.next, &f);
        self.scenes.insert(self.next, f);
        r
    }

    /// The answer's members to a request (without `id` and `ok`).  `emit`
    /// sends a line before the answer (progress); `cancelled` says whether
    /// the program asked to stop.
    pub fn call(&mut self, a: &Value, emit: &(dyn Fn(Value) + Sync), cancelled: &(dyn Fn() -> bool + Sync)) -> Result<Vec<(String, Value)>, String> {
        let op = arg_str(a, "op").ok_or("op is missing")?;
        match op {
            "hello" => Ok(vec![
                ("name".into(), s("mb3d")),
                ("version".into(), s(api::version())),
                ("protocol".into(), num(PROTOCOL as f64)),
                ("gpu".into(), Value::Bool(cfg!(feature = "gpu"))),
            ]),
            "gpu" => {
                api::use_gpu(a.get("on").and_then(Value::as_bool).ok_or("on is missing (true or false)")?);
                Ok(Vec::new())
            }
            "open" => {
                let f = if let Some(file) = arg_str(a, "file") {
                    if !Path::new(file).is_file() {
                        return Err(format!("{file}: no such file"));
                    }
                    Fractal::open(Path::new(file))?
                } else if let Some(text) = arg_str(a, "text") {
                    Fractal::from_text(text)?
                } else if let Some(data) = arg_str(a, "data") {
                    Fractal::from_bytes(arg_str(a, "name").unwrap_or("scene.m3p"), json::base64_decode(data)?)?
                } else if let Some(name) = arg_str(a, "preset") {
                    Fractal::preset(name)?
                } else {
                    return Err("give file, text, data (Base64, with name) or preset".into());
                };
                Ok(self.add(f))
            }
            "copy" => {
                let f = self.scene(a)?.1.clone();
                Ok(self.add(f))
            }
            "close" => {
                let id = self.scene(a)?.0;
                self.scenes.remove(&id);
                Ok(Vec::new())
            }
            "text" => Ok(vec![("text".into(), s(self.scene(a)?.1.text()))]),
            "set" => {
                let id = self.scene(a)?.0;
                let changes = arg_str(a, "changes").ok_or("changes is missing (`key = value` lines)")?;
                let f = self.scenes.get_mut(&id).ok_or("no such scene")?;
                f.set(changes)?;
                Ok(Server::about(id, f))
            }
            "resize" => {
                let id = self.scene(a)?.0;
                let f = self.scenes.get_mut(&id).ok_or("no such scene")?;
                let (w, h) = sized(f.size(), arg_size(a, "width")?, arg_size(a, "height")?);
                f.resize(w, h);
                Ok(Server::about(id, f))
            }
            "save" => {
                let f = self.scene(a)?.1;
                let file = arg_str(a, "file").ok_or("file is missing")?;
                let lower = file.to_ascii_lowercase();
                if lower.ends_with(".m3s") || lower.ends_with(".txt") {
                    write_file(file, f.text().as_bytes())?;
                } else {
                    write_file(file, &f.m3p())?;
                }
                Ok(vec![("file".into(), s(file))])
            }
            "formulas" => match arg_str(a, "name") {
                None => Ok(vec![("names".into(), Value::Arr(api::formula_names().into_iter().map(s).collect()))]),
                Some(n) => Ok(vec![("settings".into(), Value::Obj(api::formula_settings(n)?.into_iter().map(|(k, v)| (k, num(v))).collect()))]),
            },
            "guide" => Ok(vec![("text".into(), s(crate::mcp::GUIDE))]),
            "render" => self.render(a, emit, cancelled),
            _ => Err(format!("unknown op {op}")),
        }
    }

    fn render(&self, a: &Value, emit: &(dyn Fn(Value) + Sync), cancelled: &(dyn Fn() -> bool + Sync)) -> Result<Vec<(String, Value)>, String> {
        if cancelled() {
            return Err("cancelled".into());
        }
        let mut f = self.scene(a)?.1.clone();
        let (w, h) = sized(f.size(), arg_size(a, "width")?, arg_size(a, "height")?);
        if (w, h) != f.size() {
            f.resize(w, h);
        }
        let file = arg_str(a, "file");
        let format = arg_str(a, "format").unwrap_or(if file.is_some() { "none" } else { "png" });
        if !matches!(format, "png" | "rgb" | "none") {
            return Err(format!("format {format}: png, rgb or none"));
        }
        let id = a.get("id").cloned().unwrap_or(Value::Null);
        let last = Mutex::new(Instant::now());
        let progress = |stage: &str, done: usize, total: usize| {
            let Ok(mut t) = last.lock() else { return };
            if t.elapsed() >= Duration::from_millis(100) {
                *t = Instant::now();
                emit(obj([("id", id.clone()), ("event", s("progress")), ("stage", s(stage)), ("done", num(done as f64)), ("total", num(total as f64))]));
            }
        };
        let opt = RenderOptions { progress: if arg_bool(a, "progress") { Some(&progress) } else { None }, cancel: Some(cancelled) };
        let pic = f.render(&opt)?;
        let mut r = vec![
            ("width".to_string(), num(pic.width as f64)),
            ("height".to_string(), num(pic.height as f64)),
            ("seconds".to_string(), num((pic.seconds * 1000.0).round() / 1000.0)),
            ("device".to_string(), s(pic.device.clone())),
        ];
        if let Some(file) = file {
            write_file(file, &pic.png())?;
            r.push(("file".into(), s(file)));
        }
        if let Some(file) = arg_str(a, "depth_file") {
            write_file(file, &pic.depth_png())?;
            r.push(("depth_file".into(), s(file)));
        }
        let depth = arg_bool(a, "depth");
        match format {
            "png" => {
                r.push(("png".into(), s(json::base64(&pic.png()))));
                if depth {
                    r.push(("depth_png".into(), s(json::base64(&pic.depth_png()))));
                }
            }
            "rgb" => {
                r.push(("rgb".into(), s(json::base64(&pic.rgb))));
                if depth {
                    r.push(("depth".into(), s(json::base64(&u16_bytes(&pic.depth)))));
                }
            }
            _ => {}
        }
        Ok(r)
    }

    /// The whole answer line to a request.
    pub fn handle(&mut self, req: &Value, emit: &(dyn Fn(Value) + Sync), cancelled: &(dyn Fn() -> bool + Sync)) -> Value {
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let mut line = vec![("id".to_string(), id)];
        match self.call(req, emit, cancelled) {
            Ok(mut r) => {
                line.push(("ok".into(), Value::Bool(true)));
                line.append(&mut r);
            }
            Err(e) => {
                line.push(("ok".into(), Value::Bool(false)));
                line.push(("error".into(), s(e)));
            }
        }
        Value::Obj(line)
    }
}

/// The size asked for: both given, or one with the other following the
/// picture's proportions, or the scene's own.
fn sized(own: (usize, usize), w: Option<usize>, h: Option<usize>) -> (usize, usize) {
    let (w0, h0) = (own.0 as f64, own.1 as f64);
    match (w, h) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, ((h0 * w as f64 / w0).round() as usize).clamp(16, 32768)),
        (None, Some(h)) => (((w0 * h as f64 / h0).round() as usize).clamp(16, 32768), h),
        (None, None) => own,
    }
}

fn send(v: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", v.to_json());
    let _ = out.flush();
}

/// Serves requests until the input ends.
pub fn serve() -> Result<(), String> {
    crate::log::info("serve", "serve mode started");
    // the lines are read on a thread of their own, so that `cancel` is
    // seen while a picture is calculated: it stops every `render` sent
    // before it (the one running and those waiting); other requests are
    // carried out, so a change of a scene is never lost
    let cancel_upto = Arc::new(AtomicU64::new(0));
    let (tx, rx) = mpsc::channel::<(u64, Result<Value, String>)>();
    let reader_cancel = cancel_upto.clone();
    std::thread::spawn(move || {
        let mut seq = 0u64;
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            let req = json::parse(&line);
            if let Ok(v) = &req {
                if v.get("op").and_then(Value::as_str) == Some("cancel") {
                    reader_cancel.store(seq, Ordering::SeqCst);
                    send(&obj([("id", v.get("id").cloned().unwrap_or(Value::Null)), ("ok", Value::Bool(true))]));
                    continue;
                }
            }
            seq += 1;
            if tx.send((seq, req)).is_err() {
                break;
            }
        }
    });
    let mut server = Server::default();
    for (seq, req) in rx {
        let cancelled = || cancel_upto.load(Ordering::SeqCst) >= seq;
        let reply = match req {
            Ok(req) => server.handle(&req, &|v| send(&v), &cancelled),
            Err(e) => obj([("id", Value::Null), ("ok", Value::Bool(false)), ("error", s(format!("not JSON: {e}")))]),
        };
        send(&reply);
    }
    crate::log::info("serve", "serve mode ended (the program closed it)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(sv: &mut Server, line: &str) -> Value {
        sv.handle(&json::parse(line).unwrap(), &|_| {}, &|| false)
    }

    fn ok(v: &Value) -> bool {
        v.get("ok").and_then(Value::as_bool) == Some(true)
    }

    #[test]
    fn a_scene_is_opened_changed_and_rendered() {
        let mut sv = Server::default();
        let hello = ask(&mut sv, r#"{"id":1,"op":"hello"}"#);
        assert_eq!(hello.get("protocol").and_then(Value::as_f64), Some(PROTOCOL as f64));
        let open = ask(&mut sv, r#"{"id":"a","op":"open","preset":"Integer Power"}"#);
        assert!(ok(&open), "{}", open.to_json());
        assert_eq!(open.get("id").and_then(Value::as_str), Some("a"));
        assert_eq!(open.get("scene").and_then(Value::as_f64), Some(1.0));
        assert!(ok(&ask(&mut sv, r#"{"op":"set","scene":1,"changes":"iterations = 7"}"#)));
        let text = ask(&mut sv, r#"{"op":"text","scene":1}"#);
        assert!(text.get("text").and_then(Value::as_str).unwrap().contains("iterations = 7"));
        let bad = ask(&mut sv, r#"{"op":"set","scene":1,"changes":"no_such_key = 1"}"#);
        assert!(!ok(&bad) && bad.get("error").is_some());

        let events = Mutex::new(0usize);
        let req = json::parse(r#"{"id":5,"op":"render","scene":1,"width":64,"format":"rgb","depth":true,"progress":true}"#).unwrap();
        let pic = sv.handle(&req, &|v| {
            assert_eq!(v.get("event").and_then(Value::as_str), Some("progress"));
            *events.lock().unwrap() += 1;
        }, &|| false);
        assert!(ok(&pic), "{}", pic.to_json());
        let (w, h) = (pic.get("width").and_then(Value::as_f64).unwrap() as usize, pic.get("height").and_then(Value::as_f64).unwrap() as usize);
        assert_eq!(w, 64);
        assert_eq!(json::base64_decode(pic.get("rgb").and_then(Value::as_str).unwrap()).unwrap().len(), w * h * 3);
        assert_eq!(json::base64_decode(pic.get("depth").and_then(Value::as_str).unwrap()).unwrap().len(), w * h * 2);
        // the scene keeps its own size
        let again = ask(&mut sv, r#"{"op":"resize","scene":1}"#);
        assert_ne!(again.get("width").and_then(Value::as_f64), Some(64.0));

        let stopped = sv.handle(&json::parse(r#"{"op":"render","scene":1,"width":64}"#).unwrap(), &|_| {}, &|| true);
        assert_eq!(stopped.get("error").and_then(Value::as_str), Some("cancelled"));

        let copy = ask(&mut sv, r#"{"op":"copy","scene":1}"#);
        assert_eq!(copy.get("scene").and_then(Value::as_f64), Some(2.0));
        assert!(ok(&ask(&mut sv, r#"{"op":"close","scene":1}"#)));
        assert!(!ok(&ask(&mut sv, r#"{"op":"text","scene":1}"#)));
        assert!(ok(&ask(&mut sv, r#"{"op":"text","scene":2}"#)));
        assert!(!ok(&ask(&mut sv, r#"{"op":"nope"}"#)));
    }

    #[test]
    fn scenes_come_from_text_and_bytes_and_go_to_files() {
        let mut sv = Server::default();
        let base = Fractal::preset("Integer Power").unwrap();
        let from_text = sv.handle(&obj([("op", s("open")), ("text", s(base.text()))]), &|_| {}, &|| false);
        assert!(ok(&from_text), "{}", from_text.to_json());
        let from_data = sv.handle(&obj([("op", s("open")), ("name", s("x.m3p")), ("data", s(json::base64(&base.m3p())))]), &|_| {}, &|| false);
        assert!(ok(&from_data), "{}", from_data.to_json());
        let dir = std::env::temp_dir().join(format!("mb3d-serve-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.m3p");
        assert!(ok(&sv.handle(&obj([("op", s("save")), ("scene", num(1.0)), ("file", s(p.to_string_lossy()))]), &|_| {}, &|| false)));
        let back = sv.handle(&obj([("op", s("open")), ("file", s(p.to_string_lossy()))]), &|_| {}, &|| false);
        assert!(ok(&back), "{}", back.to_json());
        let names = ask(&mut sv, r#"{"op":"formulas"}"#);
        assert!(matches!(names.get("names"), Some(Value::Arr(n)) if !n.is_empty()));
        let set = ask(&mut sv, r#"{"op":"formulas","name":"Integer Power"}"#);
        assert!(matches!(set.get("settings"), Some(Value::Obj(n)) if !n.is_empty()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
