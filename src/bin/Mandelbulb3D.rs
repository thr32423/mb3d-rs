//! The Mandelbulb3D desktop application (MB3D's windows).
#![cfg_attr(windows, windows_subsystem = "windows")]
#![allow(non_snake_case)]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = mb3d::app::run(&args) {
        eprintln!("Mandelbulb3D: {e}");
        std::process::exit(1);
    }
}
