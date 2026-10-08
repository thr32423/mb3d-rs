# mb3d-rs — Mandelbulb3D in Rust

A Rust port of [Mandelbulb3D](https://github.com/thargor6/mb3d) (MB3D, Delphi).
It renders MB3D parameter files (`.m3p`, `.m3i`, text parameters) and its own
`.m3s` scene files from the command line: built-in and custom `.m3f`
formulas (including dIFS and map formulas) in alternating hybrids or DE
combinations, inside rendering, MB3D's camera and ray marcher, colouring,
global, positional, visible and light-map lights, background pictures,
diffuse colour maps, volumetric light, ambient occlusion (15 and 24 bit SSAO,
random SSAO, DEAO), hard and soft shadows, cutting planes and depth of field.
Everything is multithreaded, big images can be rendered in tiles, and the
output is a PNG. Scenes can be written back as MB3D parameter files.

The crate has **no external dependencies** (std only, including its own PNG
encoder) and builds with any recent stable Rust:

```sh
cargo build --release
./target/release/mb3d --help
./target/release/mb3d -f "Amazing Box" -W 1024 -H 768 -o box.png
./target/release/mb3d examples/mandelbulb.m3s -o bulb.png --depth bulb_depth.png
./target/release/mb3d examples/hybrid_box_bulb.m3s -s iterations=40 --auto-color

# MB3D parameter files with custom formulas (.m3f from the MB3D repo):
./target/release/mb3d City.m3p --formulas path/to/mb3d/M3Formulas --scale 0.1 --aa 2 -o city.png
./target/release/mb3d -f MengerHyper --formulas path/to/mb3d/M3Formulas -o menger.png

# text parameters as posted in forums (Mandelbulb3Dv18{...}) work the same way:
./target/release/mb3d pasted.txt --formulas path/to/mb3d/M3Formulas --scale 0.1 -o pasted.png
# images (maps, background pictures) are searched in --maps DIR, $MB3D_MAPS,
# ./M3Maps, next to the executable and next to the formula directory:
./target/release/mb3d "6 AM - Torii temple.m3p" --formulas path/to/mb3d/M3Formulas --maps path/to/mb3d/M3Maps

# conversions between .m3p, .m3i, text parameters and .m3s (no rendering
# unless --render is given):
./target/release/mb3d City.m3p --save-text city.txt
./target/release/mb3d pasted.txt --save-m3p pasted.m3p
./target/release/mb3d City.m3p --save-scene city.m3s
./target/release/mb3d city.m3s -s iterations=80 --save-m3p city80.m3p

# big images in tiles (one tile after another, then stitched):
./target/release/mb3d City.m3p --formulas path/to/mb3d/M3Formulas --tiles 4x4 -o city.png
# or one tile per machine / process, written as MB3D tile parameter files:
./target/release/mb3d City.m3p --tiles 4x4 --tile 2,3 --save-m3p city_t23.m3p
./target/release/mb3d City.m3p --formulas path/to/mb3d/M3Formulas --tiles 4x4 --tile 2,3 -o t23.png
```

`--scale` shrinks the image for previews of large parameter files. The DE
stop value is in pixels, so it is scaled along with the image; the result is
like rendering at full size and downsampling. Without `--aa`, detail finer
than a pixel shows up as noise, which is why MB3D authors often render at
4000–6000 pixels and downsample.

## Editor in the browser

```sh
./target/release/mb3d gui --formulas path/to/mb3d/M3Formulas --maps path/to/mb3d/M3Maps [file.m3p]
# then open http://127.0.0.1:8080/  (--port N, --host 0.0.0.0 to allow other machines)
```

`mb3d gui` is a small built-in web server (std only, no dependencies) with a
single-page editor:

* **Preview** in progressive passes (⅛, ¼, ½, full view width) that restart
  on every change; the early passes skip shadows, DEAO and volumetric light.
  Like MB3D's navigator, previews keep the DE stop in preview pixels, so they
  are fast and a little coarse. *Render image…* calculates the final image
  with the scene's full settings (size, anti-aliasing, tiles) for download.
* **Navigation** with the mouse (click: fly towards the point / turn to it /
  set the DOF focus; drag: turn; wheel: forward/back), the keyboard (W/S,
  A/D, R/F move, arrow keys turn, Q/E roll, +/− zoom, Shift for small steps)
  or the buttons. Moves are a percentage of the distance estimate at the
  camera, like MB3D's navigator.
* **Editors** for formulas (hybrid slots with the options of built-in and
  `.m3f` formulas), camera, ray marching, ambient occlusion, shadows, DOF,
  colours (palette, interior colours, maps), lights and background; every
  scene key can be added, the whole scene can be edited as text; undo/redo.
* **Files:** open `.m3p`, `.m3i`, `.m3s` and pasted text parameters; save as
  `.m3s`, `.m3p` or MB3D text parameters (to paste into MB3D).

The page and the renderer talk over a few JSON/PNG endpoints (`src/gui.rs`),
so the server can also run on a bigger machine than the browser.

## What is ported

Each Delphi routine was ported from its assembler or Pascal source. Where only
x87/SSE2 assembler existed, the stack code was translated back to scalar code
and kept in the original evaluation order.

| Rust | Delphi source | Contents |
|---|---|---|
| `math.rs` | Math3D.pas | view vectors (`BuildViewVectorDFOV`, sphere pano), `RotateVectorReverse`, `NormaliseMatrixTo`, `BuildSMatrix4`/`Rotate4Dex`, `CreateXYVecsFromNormals`, `FastIntPow`, spline weights |
| `formulas.rs` | formulas.pas, CustomFormulas.pas, HeaderTrafos.pas | Integer Power 2–8 (`HybridItIntPow2..P8`), Real Power (`HybridFloatPow`), Quaternion, Tricorn, Amazing Box (`HybridCube`/`HybridCubeDE`), Bulbox (`HybridSuperCube2`), Folding Int Pow, with their default options, DE options, DE scales and bailouts |
| `iteration.rs` | formulas.pas, TypeDefinitions.pas | `TIteration3Dext`, `doHybridPas`, `doHybridPasDE`, `doHybrid4DPas`, `CalcSmoothIterations` |
| `calc.rs` | HeaderTrafos.pas, Calc.pas, CalcThread.pas | `GetMCTparasFromHeader` (DE scale, colour heuristics, camera, z buffer constants), `CheckFormulaOptions`, `CalcDEanalytic`, `CalcDEnoADE` (4-point gradient DE), `RMCalculateNormals[OnSmoothIt]` incl. smooth normals and roughness, `RMdoBinSearch[It]`, `RMdoColor`, `CalcZposAndRough`, the main ray-march loop of `TMandCalcThread.Execute` (random first step, step-width regulation, dynamic fog step counting) |
| `lighting.rs` | HeaderTrafos.pas, LightAdjust.pas, PaintThread.pas | `MakeLightValsFromHeaderLight`, `SetCosTabFunction`/`GetCosTabVal`, `CalcColors[Inside]`, `CalcPixelColor2` and `CalcPixelColorSqr` ("internal gamma 2"): global lights with diffuse + specular, light maps, background pictures, diffuse colour maps, ambient top/bottom or from the background, depth fog, dynamic fog options, both total-light modes, gamma; MB3D's start-up light preset |
| `render.rs` | Calc.pas, PaintThread.pas, DivUtils.pas | threaded calculation (rows interleaved per thread like `CalcMandT`), painting, tiled rendering (`CalcRect`, `TilingOptions`) |
| `ssao.rs` | AmbHiQ.pas | 24 bit screen space ambient occlusion (`TAmbHiQCalc`, `TAmbHiQCalcT0`, the random variants `TAmbHiQCalcR`/`RT0`, `NextATlevelHiQ`) |
| `ssao15.rs` | AmbShadowCalcThreadN.pas | 15 bit SSAO (`BuildATlevels`, `TAmbShadowCalc`, `TAmbShadowCalcT0`) |
| `iteration.rs` (dIFS) | formulas.pas | `doHybridIFS3D` and the dIFS calling convention |
| `calc.rs` (DE combination, inside) | Calc.pas, CalcThread.pas, HeaderTrafos.pas | DE combinations of two hybrid parts (min, max, inverted max, smooth, mix), inside and in-and-outside rendering, `CheckHybridOptions` |
| `image.rs` | (new) | PNG, JPEG (baseline, progressive), BMP and PGM decoders for the maps |
| `maps.rs` | ImageProcess.pas, maps/Maps.pas | `LoadLightMap`, bicubic `GetLightMapPixel[Sphere]`, `SplineIpolMap`, `MakeSmallLMimage`; map formulas get MB3D's `PMapFunc` host functions |
| `calc.rs` (shadows), `render.rs` | CalcHardShadow.pas, HeaderTrafos.pas | hard shadows for up to 6 global lights incl. the "open air" shortcut, the one-light soft shadow (`calcHSsoft`), `CalcHSVecsFromLights` |
| `lighting.rs` (positional) | PaintThread.pas, HeaderTrafos.pas | positional lights with 1/d² falloff, visible light sources (`CalcPosLightShape`, `CalcXYZposForLight`, `SortLights`) |
| `vollight.rs` | maps/Maps.pas, CalcThread.pas | volumetric light: light shadow map (cube map for positional lights) and its integration along the view rays (`DoDynFog`) |
| `deao.rs` | CalcAmbShadowDE.pas | DE ambient occlusion with 3–33 rays, dithering and ray correction |
| `dof.rs` | DOF.pas | depth of field, sorted and forward variants, 1–4 passes |
| `m3p.rs` (text) | FileHandling.pas, DivUtils.pas | text parameters (`GetHeaderFromText`, `MakeTextparas`), `.m3i` parameter loading |
| `gbuffer.rs` | TypeDefinitions.pas | `TsiLight5` |
| `gui.rs`, `gui/index.html` | Navigator.pas, Mand.pas (main window) | browser editor: HTTP server, progressive preview worker, navigation (`SpeedButton1Click`: DE-scaled moves, rotations around the camera), picking from the G-buffer |
| `scene.rs` | TMandHeader10 + GUI defaults | scene parameters, defaults, the `.m3s` text format |
| `m3p.rs` | FileHandling.pas (`LoadParameter`, `UpdateLightParasAbove3`), DivUtils.pas (tiling), TypeDefinitions.pas | binary `.m3p` files (MandId ≥ 20): `TMandHeader10`, `TLightingParas9`, `THeaderCustomAddon`; reading with the upgrades of older versions, and writing (MandId 44) |
| `m3f.rs` | CustomFormulas.pas (`LoadCustomFormula`, `FillCustomVBufWithVars`) | `.m3f` custom formula files, all 23 option types, constants |
| `x86.rs` | (new) | IA-32 interpreter for the formulas' machine code: integer, x87 and SSE/SSE2 subset; plus a compiler that resolves the x87 stack statically into micro-ops |
| `custom.rs` | formulas.pas / TypeDefinitions.pas | memory layout of `TIteration3Dext` and the calling convention used by MB3D's hybrid loop, host functions |

Tests check that the translated assembler matches the reference spherical
triplex formulas for all integer powers, that renders are deterministic and
independent of the thread count, that every preset renders, and that the DEs
are sane.

## Custom formulas (`.m3f`)

MB3D's 460 formula files store their iteration step as 32-bit x86 machine
code. This port runs that code in three tiers:

1. **Translated to Rust** (`src/native.rs`, generated): 441 formulas. Their
   machine code was translated ahead of time into ordinary Rust functions,
   which the Rust compiler turns into native code for whatever CPU the crate
   is built for (x86-64, ARM64, …); nothing in it is specific to Intel CPUs.
   At run time a formula is recognised by a hash of its code bytes.
2. **Compiled micro ops** (`x86.rs`): the same analysis, executed by a small
   loop. Used for formula files that have no translation (for example new
   files added later), or with `MB3D_NO_NATIVE=1`.
3. **Interpreter** (`x86.rs`): an IA-32 emulator (integer, x87, SSE/SSE2
   subset) for code that does not fit the static model (a handful of formula files).

All tiers emulate the flat 32-bit address space with the `TIteration3Dext`
record, the option and constant buffer and a stack, using the same memory
layout and calling convention as MB3D.

The translation and the micro ops rely on the x87 register stack depth being
known at every instruction. Where code joins with different depths (for
example paths for invalid option values, or helper routines called at
different depths), the register file is rotated exactly as a different x87
TOP pointer would, so behaviour matches the CPU on every path.

Regenerating the translation (only needed when the formula set changes):

```sh
cargo run --release --bin m3f2rs -- path/to/M3Formulas -o src/native.rs
cargo build --release
```

Speed on the 2-core test machine, 120-pixel previews of the three test scenes
(volumetric light off), compared with phase 4: City 35.2 s → 4.6 s, Eagle
18.3 s → 5.4 s, Electric 28.0 s → 5.2 s. The images are pixel-identical.
Formulas with very short code (Eagle) gain less, because the cost of passing
the iteration state in and out of the emulated memory remains.

Verification: `tools/oracle/oracle.asm` is a small native 32-bit Linux
program that runs the same machine code on the real CPU from the same memory
image. `src/bin/m3fcheck.rs` compares interpreter and CPU over random inputs
for every formula:

```sh
nasm -f elf32 tools/oracle/oracle.asm -o /tmp/oracle.o && ld -m elf_i386 /tmp/oracle.o -o /tmp/mb3d_oracle
./target/release/m3fcheck path/to/M3Formulas --oracle /tmp/mb3d_oracle --trials 12
#   460 formulas: 449 ok, 0 mismatch, ...
```

All 449 testable formulas, dIFS formulas included, agree with the CPU to
1e-9 relative, in every tier (the check runs the translated code by default,
`MB3D_NO_NATIVE=1` checks the micro ops). The only differences are in the
last bits, because the x87 computes with 80-bit precision and the port uses
f64. Not checked: 3 formulas compiled from Pascal source (`[SOURCE]`, not
supported) and the formulas that read image maps through `PMapFunc` (the
oracle has no maps; they work in the renderer).

Formulas are looked up in `--formulas DIR`, `$MB3D_FORMULAS`, `./M3Formulas`
and `M3Formulas` next to the executable.

## Scene files (`.m3s`)

The INI-style format holds global keys, then optional `[formula]` sections (up
to 6, forming an alternating hybrid) and `[light]` sections (up to 6). See
`examples/`. Any key can be overridden on the command line with
`-s key=value`. `Scene::to_text()` writes the format, and `.m3p` → `.m3s` →
`.m3p` keeps every setting the port uses (checked over the parameter
collection). In `[light]` sections, `slot = N` keeps MB3D's light numbering,
`lights = none` turns all lights off, and formula options whose names are
not valid keys are written as `optionN = value ; name`.

The light values use MB3D's slider units (for example `diffuse = 50` means
1.0, `dyn_fog = 53` means off), so the binary `.m3p` light record can be mapped
onto them directly in phase 2. `color_start`/`color_end` are the colour-range
sliders (`TBpos[9]`, `TBpos[10]`). `--auto-color` fits them to the rendered
surface; this convenience is not in MB3D.

## Ambient occlusion and shadows

Like MB3D, both run on the G-buffer after the main calculation (as with the
"calculate automatically" options), and the painter then applies them.

| key | values | MB3D |
|---|---|---|
| `ao` | `off`, `ssao24` (default), `ssao24t0`, `ssao15`, `ssao15t0`, `deao` | ambient shadow, "SSAO 24 bit" / "threshold to 0" / "15 bit" / "DEAO" |
| `ao_random` | 0 = off, 1..n passes | random 24 bit SSAO with n accumulated passes |
| `deao_quality` | 0..3 (3, 7, 17, 33 rays) | DEAO quality |
| `deao_dither` | 0, 1 (2×2), 2 (3×3) | DEAO dithering |
| `deao_max_len` | default 1 | DEAO max. ray length |
| `ao_threshold` | z/r threshold, default 2 | `sAmbShadowThreshold` |
| `ao_border` | 0..0.9 | border mirror size |
| `shadows` | `off` (default), `all`, or light numbers like `1,3` | hard shadows for the selected lights |
| `shadow_soft` | `true`/`false` | "1 soft HS" (uses the last selected light) |
| `shadow_soft_radius` | 0.01..20 | soft shadow radius |
| `shadow_max_len` | default 1 | max. shadow length multiplier |
| `shadow_set_cos` | `true`/`false` | set the diffuse function of shadowed lights to cos |

```sh
./target/release/mb3d examples/mandelbulb.m3s -s shadows=1 -s shadow_soft=true -o soft.png
```

`.m3p` files bring their own settings. Shadows are only calculated when the
file has "calculate automatically" set. Hard shadows run the DE ray march
again for every pixel and light, so they cost about as much as the image
itself per light.

## Lights, volumetric light, cutting planes, depth of field

| key | values | MB3D |
|---|---|---|
| `position` (in `[light]`) | x, y, z | positional light at this world position |
| `visible` (in `[light]`) | 0 = no, 1..4 = MB3D's visible light shapes | visible light source |
| `shadow` (in `[light]`) | `true`/`false` | light uses hard shadows |
| `vol_light` | `off` or light number 1-6 | volumetric light of that light (shown with the dynamic fog colours, `dyn_fog` sets the amount) |
| `vol_light_map_size` | -7..7 | size of the light's shadow map in 20 % steps |
| `cut_x`, `cut_y`, `cut_z` | position or `off` | cutting planes; the side away from the camera is kept |
| `dof` | `off`, `sorted`, `forward` | depth of field |
| `dof_focus`, `dof_focus2` | distance as fraction of the image width | sharp range (`--stats` prints the value for the image centre) |
| `dof_aperture` | 0.0001..2 | aperture |
| `dof_max_radius` | pixels | maximum blur radius |
| `dof_passes` | 1..4 | passes |

```sh
./target/release/mb3d examples/mandelbulb.m3s -s dof=sorted -s dof_focus=0.61 -s dof_aperture=0.3 -o dof.png
./target/release/mb3d examples/mandelbulb.m3s -s cut_y=-0.2 -o cut.png
```

The volumetric light map is calculated before the image, with a ray march
per map pixel, so it adds noticeable time.

## Hybrids, inside rendering, maps

| key | values | MB3D |
|---|---|---|
| `de_combination` | `min`, `max`, `max_inverted`, `smooth_linear`, `smooth`, `mix` | DE combination of hybrid part 1 (formulas 1..`decomb_end1`) and part 2 |
| `decomb_end1`, `decomb_start2`, `decomb_end2`, `decomb_repeat2` | formula slots 1..6 | the two parts |
| `decomb_iterations2`, `decomb_smooth`, `decomb_mix_pow`, `decomb_mix_color` | | part 2 iterations, smoothing, mix options |
| `inside` | `outside`, `inside`, `both` | inside rendering, in-and-outside |
| `background_image` | file name | background picture (sphere or `background_direct = true` image coordinates) |
| `background_rotation`, `background_brightness`, `background_add_light`, `background_ambient` | | picture rotation (bytes, 128 = half turn), brightness 1.04^(v-40), add instead of fog, small blurred copy as ambient light |
| `map`, `map_rotation` (in `[light]`) | map number, 3 bytes | light map: an image lighting from all directions |
| `diffuse_map` | map number | diffuse colour map |
| `diffuse_map_mode` | `iterations_otrap`, `normals`, `wrap_sine`, `wrap` | how the map is placed |
| `diffuse_map_offset`, `diffuse_map_rotation`, `diffuse_map_scale`, `diffuse_map_brightness_only` | | placement and "Y + colour" combination |
| `internal_gamma2` | `true`/`false` | light calculation in squared colour space |
| `ambient_relative_to_object`, `dynfog_options`, `light_mode`, `color_interpolation` | | further light options of MB3D's lighting editor |
| `tiles`, `tile`, `tile_downscale` | `4x3`, `col, row` (1-based), 1..3 | tiled rendering (also `--tiles`, `--tile`) |

Map numbers refer to the files of MB3D's `M3Maps` folder (`12.png`,
`129 name.jpg`, ...). Map formulas (height maps, map transforms) find their
images the same way. A missing image is reported and the feature is skipped.

Tiled rendering: with all tiles, the tiles are calculated one after another
into one G-buffer and SSAO, painting and DOF run on the whole image, so the
result equals an untiled render. A single tile is calculated with a border of
24 pixels for the image space effects, like MB3D's big-render tiles, and the
tile files that MB3D writes for big renders are rendered as that tile.

## Known differences and gaps

* **Not yet ported:** interpolation hybrids, stereo, 2D slices, the internal
  formula Aexion C and formulas compiled from Pascal source (`[SOURCE]`). The
  loader reports which of these a parameter file uses. The browser editor
  covers the main window, navigator and editors; MB3D's animation, voxel
  export, Monte Carlo rendering and mutagen are not ported.
* **Speed:** custom formulas run as translated native code, but still read
  and write their values through the emulated 32-bit memory, so they are
  slower than MB3D's hand-written assembler (not benchmarked against MB3D
  itself). Use `--scale` for previews; speed scales with the number of cores.
* **Pixel exactness:** the x87 code computed in 80-bit precision; this port
  uses f64, and f32 wherever MB3D stored values as `Single`. Images match in
  structure but are not bit-identical. The per-thread random seed of the
  "first step random" option is deterministic per row instead of using
  Delphi's `Random`. Verifying against MB3D needs reference renders made with
  the original program.
* Points exactly on the z axis are degenerate in MB3D's triplex formulas
  (division by `x² + y²`). This is kept as in the original.

## Roadmap

1. ✅ Core renderer CLI
2. ✅ `.m3p` loading and custom `.m3f` formulas via the x86 interpreter
3. ✅ Ambient occlusion (24 bit SSAO), hard and soft shadows
4. ✅ Rendering features: positional, visible and volumetric lights, DEAO, cutting planes, depth of field, text parameters, `.m3i` parameters, 15 bit and random SSAO, inside rendering, background pictures, light maps, diffuse colour maps, the remaining light options, `.m3p` writing
5. ✅ Formulas translated to Rust (3–8× faster), dIFS formulas, DE combinations, image maps and map formulas, tiled rendering
6. ✅ Editor in the browser (`mb3d gui`): progressive preview, navigator, formula, camera, render, colour and light editors, file handling, final rendering
7. Animation, batch rendering, voxel and mesh export, mutagen

## License

MB3D is licensed under the LGPL 2.1. This port is a derivative work and uses
the same license.
