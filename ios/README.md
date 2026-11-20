# Mandelbulb3D for iPad (prototype)

A native SwiftUI app for the iPad on top of the mb3d-rs engine. The engine
is linked as a static library (`engine/`, the C functions of
`engine/include/mb3d.h`, the same as mb3d-bindings' libmb3d). It is built
without JIT (iOS does not allow code made at run time) and without OIDN.

- `App/`: the SwiftUI app (examples list, picture, settings inspector)
- `engine/`: the engine as `libmb3d.a` for the iPad and the Simulator
- `project.yml`: the Xcode project, made by XcodeGen
- `build.sh DATA`: builds the engine, copies the formula, map and example
  folders from DATA (an mb3d-rs release folder) and makes the Xcode project

The workflow `.github/workflows/ios.yml` builds the app on GitHub's Mac,
runs it on a simulated iPad and keeps screenshots and its log.

The engine library also builds on Linux:
`cargo build --release --target aarch64-apple-ios-sim --manifest-path ios/engine/Cargo.toml`
