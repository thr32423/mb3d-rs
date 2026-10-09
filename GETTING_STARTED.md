# Getting started with mb3d-rs

mb3d-rs is a Rust port of [Mandelbulb3D](https://github.com/thargor6/mb3d)
(MB3D). It is a single program, `mb3d`, with a command line renderer and an
editor that runs in the browser (`mb3d gui`). It needs no installation and
no other libraries.

## 1. Get the program

Download the archive for your system from the
[releases](../../releases) page and unpack it:

| system | archive |
|---|---|
| Windows (64 bit) | `mb3d-rs-<version>-x86_64-pc-windows-msvc.zip` |
| Linux (64 bit) | `mb3d-rs-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| macOS, Apple silicon | `mb3d-rs-<version>-aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `mb3d-rs-<version>-x86_64-apple-darwin.tar.gz` |

Or build it yourself with [Rust](https://rustup.rs) (stable):

```sh
git clone <this repository> mb3d-rs
cd mb3d-rs
cargo build --release          # the program is target/release/mb3d
```

On macOS, a downloaded binary may need `xattr -d com.apple.quarantine mb3d`
before the first start.

## 2. Get MB3D's formulas and maps

The built-in formulas (Integer Power, Amazing Box, Quaternion, ...) work
without anything else. Most parameter files use MB3D's formula files
(`.m3f`) and maps (images), which come with MB3D:

```sh
git clone https://github.com/thargor6/mb3d
```

(or download the repository as a ZIP from GitHub). You need its folders

* `M3Formulas` — the formula files (`*.m3f`, about 460 of them),
* `M3Maps` — the maps for light maps, colour maps and backgrounds,
* `M3Parameter` — 80 example parameter files (`*.m3p`), a good start.

An existing MB3D installation has the same folders.

### Where mb3d looks for them

The easiest setup puts the folders next to the program:

```
mb3d-rs/
  mb3d(.exe)
  M3Formulas/
  M3Maps/
  M3Parameter/
```

`mb3d` searches, in this order:

| what | searched in |
|---|---|
| formulas | `--formulas DIR` (may be repeated), `$MB3D_FORMULAS` (a path list), `./M3Formulas`, `M3Formulas` next to the program |
| maps | `--maps DIR`, `$MB3D_MAPS`, `./M3Maps`, `M3Maps` next to the program, `M3Maps` next to every formula folder |

In the editor, *Prefs ▸ Ini Dirs* shows the folders in use and adds more.

## 3. The editor

```sh
mb3d gui                                   # then open http://127.0.0.1:8080/
mb3d gui "M3Parameter/6 AM - Torii temple.m3p"
mb3d gui --port 9000 --host 0.0.0.0        # reachable from other machines
```

The page follows MB3D's main window:

* **Top left:** *Animations*, *BTracer2* (meshes), *Navigator*, *MutaGen*,
  *ZBuf16Bit*; the pages *Open* (m3i, m3p, from the clipboard, new
  presets), *Save* (m3i, m3p, m3s, to the clipboard), *Save pic* (PNG,
  JPEG, Z-buffer), *Tools* (batch processing, voxel export, big renders,
  Monte Carlo, the parameter text) and *Prefs* (folders, map sequences,
  light/dark theme). Then the viewing scale and the image size.
* **Below the image:** the mouse modes — *walk* (click flies towards a point,
  drag turns, the wheel moves; keys W/S, A/D, R/F, arrows, Q/E), *2D zoom*,
  *X,Y* and *Z* — and the rotation buttons (right click: around the
  object's own axes).
* **Right:** position and rotation, *Calculate 3D*, the quality presets, the
  windows *Formulas*, *Lighting* and *Postprocess*, and the pages
  Calculation, Internal, Infos, Cutting, Julia, Camera, Coloring and
  Stereo, with the messages below.

The ⧉ button in a window's title bar opens it in its own browser window, so
you can arrange the windows next to the editor or on another screen (allow
pop-ups for the page if the browser asks); all windows stay in sync.

Every change shows a quick preview. *Calculate 3D* calculates the image at
full size; after that, changes in the Lighting window only repaint it and
changes of shadows or ambient occlusion only redo those, as in MB3D. *Save
pic ▸ PNG* saves it, reduced by the viewing scale (1:2 and 1:3 are
anti-aliased).

## 4. The command line

```sh
mb3d "M3Parameter/6 AM - Torii temple.m3p" -o torii.png            # full size
mb3d "M3Parameter/6 AM - Torii temple.m3p" --scale 0.25 -o small.png
mb3d examples/mandelbulb.m3s -s dof=sorted -s dof_focus=0.6 -o dof.png
mb3d batch M3Parameter --scale 0.2 -o previews                       # a folder
mb3d animate flight.m3a                                              # MB3D animation
mb3d montecarlo scene.m3p --rays 64 -o mc.png
mb3d mesh scene.m3p -o object.stl
mb3d --help                     # all options; also mb3d animate --help, ...
```

Parameters copied from a forum post (`Mandelbulb3Dv18{...}`) can be saved
into a text file and opened like a `.m3p`.

## 5. When something is missing

* *formula 'xyz' not found*: the formula folder is not found; see step 2,
  or add `--formulas path/to/M3Formulas`.
* *map N not found*: the same for `M3Maps` (`--maps`).
* A file renders, but looks different from MB3D: please open an issue with
  the parameter file (README ▸ "Known differences" lists what is not ported).
