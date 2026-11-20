#!/bin/bash
# Builds the iPad app on a Mac: the engine for the device and the Simulator,
# the data folders, the Xcode project. Needs Xcode, Rust and XcodeGen
# (brew install xcodegen).
#
#   ios/build.sh DATA        DATA: a folder with M3Formulas, EM_JIT_M3Formulas,
#                            M3Maps and M3Parameter (an mb3d-rs release)
#   then: open ios/MB3D.xcodeproj (or xcodebuild, see the workflow)
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
data=${1:?usage: ios/build.sh DATA-FOLDER}

rustup target add aarch64-apple-ios aarch64-apple-ios-sim
for t in aarch64-apple-ios aarch64-apple-ios-sim; do
  CARGO_TARGET_DIR="$here/engine/target" cargo build --release --manifest-path "$here/engine/Cargo.toml" --target $t
done

rm -rf "$here/Data"
mkdir -p "$here/Data"
for d in M3Formulas EM_JIT_M3Formulas M3Maps M3Parameter; do
  cp -R "$data/$d" "$here/Data/$d"
done

cd "$here" && xcodegen
