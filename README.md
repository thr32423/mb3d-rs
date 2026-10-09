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
Animations are made from keyframes with MB3D's interpolation (and MB3D's
`.m3a` animation files can be opened and saved), and lists of parameter
files are rendered in batches.

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

## Animation

```sh
# keyframes from parameter files, 60 sub-frames from each to the next:
./target/release/mb3d animate start.m3p middle.m3p end.m3p --frames 60 --save flight.m3k
./target/release/mb3d animate flight.m3k --list            # frame schedule
./target/release/mb3d animate flight.m3k --scale 0.25 --every 4 -o preview   # quick look
./target/release/mb3d animate flight.m3k --aa 2 -o frames --name flight      # frames/flight000001.png ...
# MB3D animation projects work too, and can be written for MB3D:
./target/release/mb3d animate project.m3a --formulas path/to/mb3d/M3Formulas -o frames
./target/release/mb3d animate flight.m3k --save flight.m3a
```

Like MB3D's animation maker, a keyframe is a complete parameter set plus the
number of sub-frames to the next keyframe. A frame takes everything from
its keyframe and puts the interpolated values on top
(`Interpolate2frames`, `Interpolate3framesBezier`):

* **interpolated:** camera (start/end plane and middle linearly, zoom and
  bailout logarithmically, the view matrix as quaternions with slerp, field
  of view), iterations, DE stop, raystep, julia values, 4D rotation (the short
  way round), cutting planes, DOF, shadow and AO lengths, the options and
  iteration counts of hybrid slots that hold the same formula in the
  keyframes (angle options the short way), and the **light values**: MB3D
  blends the derived `TLightVals`, not the sliders, and so does the port
  (colours, amounts, fog, gamma, palette, light colours, global light
  directions on the sphere, positional lights by position, picture and map
  rotations). Lights that are on in only one keyframe fade in or out.
* **interpolation:** linear, or MB3D's "quadratic bezier" (the default): a
  quadratic B-spline through the middle points between the keyframes, with
  the sub-frame position corrected for different frame counts so the speed
  stays continuous. The curve is pulled towards the keyframes but does not
  pass exactly through them. **Loop** animations continue from the last
  keyframe to the first.
* **frames:** `<folder>/<name><6 digit index>.png` like MB3D (start index and
  step, `--from`/`--to`/`--frame` select file indices). `--aa N` is MB3D's
  image scale (frames calculated N times larger and reduced). `--format bmp`,
  or `--format m3p` for one parameter file per frame (to render them
  elsewhere, e.g. with `mb3d batch`; their light settings are the blend
  interpolated on the sliders, a close approximation). `--depth` writes
  `ZBuf <name><index>.png` too.
* **continuing and sharing:** each output file is created and locked while
  its frame is calculated (MB3D's `OccupyDFile`). With `--skip-existing`,
  finished frames are kept, so an interrupted render continues where it
  stopped, and several processes (or machines on a shared folder) render
  one animation together.
* `-s key=value` changes a key in every keyframe (what MB3D's
  "process keyframes" window does).

Animation files:

* `.m3a`: MB3D's binary animation project (version 5, written by MB3D 1.7 and
  later): settings, keyframes and their preview images. Older versions
  (`TMandHeader9` keyframes) are not supported.
* `.m3k`: the text format of this port. Keyframes refer to parameter files
  (with optional changes) or hold MB3D text parameters inline:

```text
# mb3d animation
width = 1280
height = 720
scale = 2                # anti-aliasing (MB3D's image scale)
interpolation = bezier   # or linear
loop = false
output = frames          # relative to this file
name = flight
format = png             # png, bmp, m3p
start_index = 1
index_step = 1
overwrite = true
save_depth = false

[keyframe]
frames = 60
file = start.m3p
set = iterations = 40    # changes on top (repeatable)

[keyframe]
frames = 60
params = Mandelbulb3Dv18{
...
}
```

In the editor (`mb3d gui`), the **Animation** tab is the animation maker:
navigate, add the view as a keyframe (or insert it after one, replace one,
reorder, delete, set frame counts), click a keyframe image to load it back.
**Preview frames** renders all or every n-th frame small (fast mode without
shadows, volumetric light and DEAO, like MB3D's) into a flipbook with a
player; any frame can be opened in the editor, e.g. to turn it into a new
keyframe. **Render frames** writes the frames into the output folder in the
background. Animations are opened and saved as `.m3k` or `.m3a`.

## Batch rendering

```sh
./target/release/mb3d batch path/to/mb3d/M3Parameter -o renders --formulas path/to/mb3d/M3Formulas
./target/release/mb3d batch a.m3p b.m3s pasted.txt --aa 2 --skip-existing
./target/release/mb3d batch --list jobs.txt --scale 0.25 -o previews --dry-run
```

Parameter files (`.m3p`, `.m3i`, `.m3s`, text parameters; a directory means
all of them in it, and list files with one file per line, optionally followed
by changes: `6 AM - Torii temple.m3p | width = 1920 | height = 1080`) are
rendered one after another to `<name>.png` next to them or in `--output`,
like MB3D's batch window. Files that cannot be rendered are reported and the
batch goes on (the exit code tells whether all succeeded). As with
animations, outputs are locked while they are calculated: `--skip-existing`
continues an interrupted batch and lets several processes share a list.
`--format m3p` converts the files to MB3D parameter files instead.
All 80 of MB3D's example parameter files render.

## Voxel export

```sh
./target/release/mb3d voxel scene.m3p -o voxels --slices 256
./target/release/mb3d voxel scene.m3p --slices 400 --scale 1,1,0.5 --axes --iterations
./target/release/mb3d voxel scene.m3p --preview vox.png        # quick look first
./target/release/mb3d voxel project.m3v                        # MB3D voxel projects
./target/release/mb3d voxel scene.m3p --save-m3v project.m3v   # ... and back to MB3D
```

Like MB3D's voxel export, the object is cut into a stack of 1 bit PNG slices
(white = object) for voxel editors and 3D printing. The stack is a box of
2.2 / zoom scene units around the scene's middle, oriented like the view
(`--axes`: like the formula's axes), with `--slices` images of as many
pixels. A voxel is solid where the distance estimate is below the threshold
(by default from the DE stop) or, with `--iterations`, where the iteration
count reaches the maximum. In-and-outside rendering limits and MB3D's `.m3v`
projects (versions 1–4, read and written) are supported.

## Mesh export

```sh
./target/release/mb3d mesh scene.m3p -o bulb.stl
./target/release/mb3d mesh scene.m3p -o bulb.obj --resolution 300 --sharpness 2 --colors --smooth 5
./target/release/mb3d mesh scene.m3p -o part.ply --bounds 0,50,0,100,0,100 --close
```

A port of MB3D's BulbTracer2 mesh export: the distance estimate is sampled on
an N × N × N grid in a cube around the scene's middle (scale 0.5 = twice the
visible 2.2 / zoom), marching cubes with BulbTracer2's tables (including the
ambiguous cases) builds the surface where the DE is 1 / sharpness grid steps,
equal vertices are merged and the mesh is centred and scaled to size 1. OBJ
and PLY are written like MB3D's writers (with normals and optional vertex
colours from the scene's palette); binary STL, closing at the bounds and
Taubin smoothing are additions. Two planes of samples are calculated at a
time on all cores, so memory stays small at high resolutions.

## MutaGen

```sh
./target/release/mb3d mutagen scene.m3p -o muta --formulas path/to/mb3d/M3Formulas
./target/release/mb3d mutagen muta/1.2.1.m3p -o muta2 --params-strength 0.5 --seed 7
```

MB3D's MutaGen makes a family of 15 random variations: the parent's mutation
1, its children 1.1 and 1.2, four grandchildren and eight great-grandchildren.
Each mutation adds, replaces or removes a formula of the hybrid (chosen from
formulas of the same kind: the DE option decides between 3D, 4D, dIFS and
ADE formulas), changes formula options, julia mode and constant, or iteration
counts, with the weights and strengths of MB3D's MutaGen window. With probing
(the default) each child is the best of up to 9 candidates rendered at
40 × 32: candidates that differ too little from their parent are dropped and
the one with the most structure (edge coverage) is kept. The output folder
gets `<label>.m3p`, a preview per member, `sheet.png` (the family in MB3D's
tree layout) and `members.txt`; the next generation starts from a chosen
member.

In the editor, the **MutaGen** tab shows the tree as it grows; a click
selects a mutation, a double-click (or "Open in editor") loads it, "Breed
from this" makes the next generation from it, and the arrows go back and
forth between generations. The **Export** tab writes voxel slices (with a
preview of the stack) and builds meshes for download.

## Monte Carlo rendering

```sh
./target/release/mb3d montecarlo "Mengerplus for MC.m3p" --rays 64 -o menger.png
./target/release/mb3d montecarlo scene.m3p --time 600 --m3c scene.m3c   # stop after 10 minutes
./target/release/mb3d montecarlo scene.m3c --rays 256                   # continue later
./target/release/mb3d montecarlo bulb.m3s -s mc_reflections=true -s mc_depth=4 --exposure 150
```

A port of MB3D's Monte Carlo renderer: path tracing with the scene's lights,
palette and background. Ambient light comes from bounces between the
surfaces (`mc_depth`), lights have a size and cast soft shadows
(`mc_soft_shadow_radius`), and with `mc_reflections` the specular colours of
the palette reflect (sharp, or rough with `mc_diffuse_reflects`) up to
`mc_reflection_depth` times. With `mc_transparency` the alpha of the
specular colours (`palette_alpha`) makes surfaces transparent: refraction
(`mc_refraction_index`), Fresnel reflection, absorption coloured by the
diffuse colour (`mc_absorption`) and light scattering inside the material
(`mc_scattering`); `mc_only_difs` limits it to dIFS formulas. Depth of field
(`dof`, `dof_aperture`, `dof_focus`) uses the bokeh shapes 1..6
(`mc_bokeh`). Volumetric light, depth and dynamic fog and visible lights
work as in normal renders.

The image is refined in passes, like in MB3D's Monte Carlo window: the first
pass shoots 4 rays per pixel (Halton sampled), every further pass adds rays
where the noise estimate and the contrast to the neighbours ask for them.
`--rays`, `--passes` and `--time` say when to stop; the image (and with
`--m3c` the state, as MB3D's `.m3c` file) is written after every pass.
Exposure (`mc_exposure`, `--exposure`), colour saturation (`mc_saturation`),
HDR soft clipping (`mc_soft_clip`) and the gamma slider only change the
painting. MB3D's own `.m3c` files can be continued and the port's opened
in MB3D.

In the editor, the **Monte Carlo** tab has these settings, renders the
editor's scene in the background (Continue adds rays to the image) and
saves PNG or `.m3c`.

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
| `anim.rs` | Animation.pas, Interpolation.pas, Math3D.pas | keyframes, sub-frame schedule (`TotalBMPsToRender`, `Timer2Timer`), `Interpolate2frames`, `Interpolate3framesBezier` (header and formula values), `bInterpolateFormula`, quaternions (`MatrixToQuat`, `SlerpQuat`, `SlerpSVec`, `BezierIpol3SVecs`) |
| `lighting.rs` (`blend`) | Interpolation.pas | the light value part of both interpolations (`TLightVals`, `Slerp2SMatrices`, `Slerp3SMatrices`) |
| `animfile.rs` | Animation.pas (`LoadAni`, `SpeedButton9Click`) | `.m3a` animation files incl. preview images; the `.m3k` text format |
| `frames.rs` | Animation.pas, Mand.pas (`DoSaveAniImage`, `AniFileAlreadyExists`, `OccupyDFile`) | frame files, claiming/locking, image scale, BMP output, keyframe previews (`RenderPrevBMP`) |
| `batch.rs` | BatchForm.pas | batch lists |
| `gui/anim.rs` | Animation.pas, AniPreviewWindow.pas | the editor's animation maker and flipbook preview |
| `voxel.rs` | VoxelExport.pas | voxel slice stacks (`TVoxelExportCalcThread`, object test, in-and-outside limits), `.m3v` projects incl. older versions, stack preview |
| `mesh.rs`, `mclut.rs` | BulbTracer2.pas, ObjectScanner2.pas, VertexList.pas, MeshWriter.pas | the DE grid (`TObjectScanner2`), marching cubes with BulbTracer2's tables, vertex merging, centring, OBJ and PLY writers; STL and Taubin smoothing (new) |
| `formulas.rs` (Aexion C) | formulas.pas | the internal formula Aexion C (`HybridAexionC`, translated from x87 assembler) with all its modes |
| `mutagen.rs` | mutagen/MutaGen.pas, MutaGenGUI.pas, PreviewRenderer.pas, FormulaNames.pas | mutation operators, probing (Sobel and difference coverage), the 15-member tree and its layout, formula categories |
| `gui/tools.rs` | MutaGenGUI.pas, VoxelExport.pas, BulbTracer2UI.pas, MonteCarloForm.pas | the editor's Monte Carlo, MutaGen and Export tabs |
| `jit.rs` | paxCompiler (JIT formulas), formulas/JIT*.m3f | a compiler for the Delphi subset of `[SOURCE]` formulas: preprocessor (options and constants), Delphi typing, `Math`/`System` functions, compiled to closures over the iteration state |
| `iteration.rs` (interpolation) | formulas.pas | `doInterpolHybridPas`, `doInterpolHybridPasDE` and the 4D variants; `doHybrid4DDEPas` |
| `calc.rs` (2D, colour on iteration) | CalcThread2D.pas, Calc.pas | `T2DcalcThread` (plane at Z start, middle or end), `doColorOnIt` |
| `scene.rs`, `render.rs` (stereo) | HeaderTrafos.pas (`StereoChange`, `CalcXoff`) | the stereo eyes: shifted middle and off-axis view centre; left and right eye images combined (an addition) |
| `reflect.rs` | CalcSR.pas, ImageProcess.pas (`NormalsOnZbuf`), Calc.pas (`CalcHS`, `CalcHSsoft`), CalcAmbShadowDE.pas (`CalcAmbShadowDEfor1pos`) | reflections and transparency of the normal renderer, normals on the z-buffer |
| `lighting.rs` (`shade`) | PaintThread.pas | `CalcPixelColorSvec` and `CalcPixelColorSvecTrans` (the painter for any view ray) |
| `mc.rs` | CalcMonteCarlo.pas, MonteCarloForm.pas, PaintThread.pas (`PaintMC`) | the Monte Carlo renderer: `CalcRay`, `CalcHSMC`, `CalcPhongLight`, `CalcVisLights`, `CalcBGLight`, `CalcN`, `DoDOF`, `CalcBokeh`, Halton sequences; `CalcAvrgNoise`, `.m3c` files; painting in Lab space |

Tests check that the translated assembler matches the reference spherical
triplex formulas for all integer powers, that renders are deterministic and
independent of the thread count, that every preset renders, and that the DEs
are sane. Animation tests check the frame schedule, the interpolation (first
and last frames equal their keyframes, logarithmic zoom, slerped rotations
and light directions, fading lights, angles the short way), the file
formats and the output claiming. Phase 8 tests cut voxel slices through the
bulb, round-trip `.m3v` files, check the marching-cubes tables, that an
analytic sphere becomes a closed, round mesh with outward normals, that a
bulb mesh is found, that Aexion C matches MB3D's Pascal reference, and that
mutations, the coverage measures and a whole generation work.

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
f64. Not checked by the oracle: the formulas compiled from Pascal source
(`[SOURCE]`, no machine code; see below) and the formulas that read image
maps through `PMapFunc` (the oracle has no maps; they work in the renderer).

Formulas written in Pascal (`[SOURCE]`, compiled by paxCompiler in MB3D) are
compiled by `jit.rs` from the Delphi subset they use; all 40 of the MB3D
repository compile, and the tests compare two of them with equivalent
formulas.

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

## Reflections and transparency, normals on the z-buffer

| key | values | MB3D |
|---|---|---|
| `mc_reflections` | `true`/`false` | "Calculate reflections" (post processing, and in the Monte Carlo renderer) |
| `mc_reflection_depth` | 0.. | reflections of reflections |
| `mc_reflection_amount` | 0..100 (0..1 realistic) | amount of reflected light |
| `mc_transparency` | `true`/`false` | transparency: the alpha of the palette's specular colours (`palette_alpha`) |
| `mc_only_difs` | `true`/`false` | only dIFS formulas are transparent |
| `mc_refraction_index`, `mc_absorption`, `mc_scattering` | | refraction, absorption and light scattering inside transparent material |
| `normals_on_zbuf` | `true`/`false` | "Normals on Z-buffer": normals from the positions of the neighbour pixels |

```sh
./target/release/mb3d "material colors.m3p" --formulas M3Formulas -o glass.png
./target/release/mb3d examples/mandelbulb.m3s -s cut_y=-0.6 -s mc_reflections=true -s mc_reflection_amount=0.8 -o mirror.png
```

Like in MB3D these run after the calculation: the normals on the z-buffer
before shadows and ambient occlusion, the reflections after painting (and
before the depth of field). For every object pixel the view ray is
reflected, or refracted into transparent material with Fresnel reflection,
marched to the next surface, which gets its own normal, colour, hard or
soft shadow and DE ambient occlusion and is lit like a painted pixel. The
specular colours of the palette say how much light a surface reflects.

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

* **Coverage:** all of MB3D's rendering modes are ported (with phase 9:
  JIT formulas, interpolation hybrids, 2D slices, stereo, the reflection
  and z-buffer normal post processing and the Monte Carlo renderer), and all
  80 example parameter files render. The browser editor covers the main
  window, navigator, editors, the animation maker, Monte Carlo, MutaGen and
  the exports. Not ported: the interactive tools of the post processing
  window (recalculating a selection, doubling the image size), the formula
  editor and map sequences.
* **Monte Carlo:** the random numbers are seeded per row and pass, so an
  image is the same with any number of threads but not MB3D's; neighbour
  pixels steer the ray counts from their state at the start of a pass (MB3D
  reads them while other threads change them). The z-buffer output of MB3D's
  MC batch is not written. MB3D's development record format (`MCoptions`
  bit 8) is not read.
* **Exports and MutaGen:** the mesh export reads but does not reproduce
  BulbTracer2's floating-point order exactly, so meshes match MB3D's in shape,
  not vertex for vertex. MutaGen uses its own random generator, so
  a seed gives other mutations than in MB3D; mutating map or light settings
  (not in MB3D's MutaGen either) is not done.
* **Animation:** map sequences (per-frame maps,
  `MapSequences.pas`), JPEG output and `.m3a` files before version 5 are not
  supported. Without a loop, MB3D's render loop also counts the sub-frames of
  the last keyframe (repeating it); the port renders the last keyframe once,
  as MB3D's own frame count (`TotalBMPsToRender`) and time estimate do.
  MB3D's per-frame `.m3p` files carry the keyframe's light settings
  unchanged; the port writes the interpolated sliders instead. Batch
  rendering writes images (or `.m3p`), not `.m3i` files with G-buffer.
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
7. ✅ Animation (keyframes, MB3D's interpolation incl. light values, `.m3a` and `.m3k` files, frame rendering shared by several processes, animation maker in the editor) and batch rendering
8. ✅ Voxel export (`.m3v` projects), mesh export (BulbTracer2: OBJ, PLY, STL), the internal formula Aexion C, MutaGen; MutaGen and Export tabs in the editor
9. ✅ The remaining gaps: JIT formulas (`[SOURCE]` Pascal), interpolation hybrids, 2D slices, colour on iteration, stereo images and animations, reflections and transparency in normal renders, normals on the z-buffer, the Monte Carlo renderer (CLI and editor tab, `.m3c` files)

## License

MB3D is licensed under the LGPL 2.1. This port is a derivative work and uses
the same license.
