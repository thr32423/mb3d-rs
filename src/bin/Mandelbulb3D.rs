//! The Mandelbulb3D desktop application (MB3D's windows).
#![cfg_attr(windows, windows_subsystem = "windows")]
#![allow(non_snake_case)]

/// How long the closed program waits for the graphics card at most: a
/// driver that hangs must not leave the process behind for ever.
#[cfg(feature = "gpu")]
const GPU_EXIT_WAIT: std::time::Duration = std::time::Duration::from_secs(300);

fn main() {
    #[cfg(feature = "gpu")]
    mb3d::gpu::use_own_driver_cache();
    // a panic or an error goes to mb3d_crash.txt as well (see crash.rs)
    mb3d::crash::install("Mandelbulb3D");
    // the log of the session (logs/ in the data folder; the Log window)
    mb3d::log::init("gui", true);
    mb3d::log::session_start("Mandelbulb3D");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = mb3d::app::run(&args);
    match &result {
        Ok(()) => mb3d::log::end("the session ended"),
        Err(e) => {
            eprintln!("Mandelbulb3D: {e}");
            mb3d::log::error("session", e);
            mb3d::log::end("the session ended with an error");
            mb3d::crash::note("Mandelbulb3D", e);
        }
    }
    // The windows are gone. A process that exits while a thread is inside the
    // driver can crash there, and a shader cut off never reaches the driver's
    // cache, so a long compile would start over next time. (After the log's
    // end: logging out during this wait is no unfinished session.)
    #[cfg(feature = "gpu")]
    mb3d::gpu::wait_idle(mb3d::gpu::compile_wait_limit().unwrap_or(GPU_EXIT_WAIT));
    if result.is_err() {
        std::process::exit(1);
    }
}

