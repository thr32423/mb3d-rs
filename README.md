<img src="assets/social-preview.png" width="760" alt="mb3d-rs: Mandelbulb3D for Linux, Windows and macOS">

# mb3d-rs — Mandelbulb3D for Linux, Windows and macOS

<!-- languages:start -->
**English** · [Deutsch](docs/i18n/README.de.md) · [Español](docs/i18n/README.es.md) · [Français](docs/i18n/README.fr.md) · [Bahasa Indonesia](docs/i18n/README.id.md) · [Italiano](docs/i18n/README.it.md) · [Nederlands](docs/i18n/README.nl.md) · [Polski](docs/i18n/README.pl.md) · [Português](docs/i18n/README.pt.md) · [Kaszëbsczi](docs/i18n/README.csb.md) · [العربية](docs/i18n/README.ar.md) · [اردو](docs/i18n/README.ur.md) · [हिन्दी](docs/i18n/README.hi.md) · [日本語](docs/i18n/README.ja.md) · [한국어](docs/i18n/README.ko.md) · [简体中文](docs/i18n/README.zh-CN.md) · [繁體中文](docs/i18n/README.zh-TW.md)
<!-- languages:end -->

<!-- latest:start -->
**Latest release: [v0.3.5](https://github.com/thr32423/mb3d-rs/releases/tag/v0.3.5)** (8 Nov 2026, 16:24 UTC) · [Download](#download) · [What's new](docs/UPDATES.md)
<!-- latest:end -->

mb3d-rs is [Mandelbulb3D](https://github.com/thargor6/mb3d) (MB3D), the
3D fractal program, rewritten so that it runs on today's computers: on
**Linux, Windows and macOS**, and with the picture calculated on the
**graphics card** where MB3D could only use the processor. It opens MB3D's
parameter files (`.m3p`, `.m3i`, text parameters from forums), uses MB3D's
formulas and maps, and makes the same pictures.

<a href="docs/img/main_window.jpg"><img src="docs/img/main_window.jpg" width="760" alt="The main window"></a>

It comes as two programs:

* **Mandelbulb3D** — the desktop program, with MB3D's own windows: the
  main window, Formulas, Lighting, Post processing, Navigator, Animation,
  Batch, Big renders, Monte Carlo, Voxel export, Bulb Tracer 2, MutaGen
  and the rest. If you know MB3D, you know this program; MB3D's tutorials
  apply.

  <details><summary>Read more</summary>

  The windows are MB3D's own — made from MB3D's original window
  definitions, so every button, slider and hint is where it is in MB3D:
  
  * **Main window:** open and save parameters (`.m3p`, `.m3i`, text, the
    clipboard), save pictures (PNG, JPEG, BMP, 16 bit Z-buffer), the mouse
    modes, the rotation buttons, image size, the quality presets, undo,
    *Calculate 3D* with the growing picture and the statistics, and all
    the pages (Calculation, Internal, Infos, Cutting, Julia, Camera,
    Coloring, Stereo).
  * **Formulas:** the six formula slots, MB3D's formula lists and options,
    hybrid types, favourites, and the JIT formula editor.
  * **Lighting:** lights 1–6, presets, the palette, the Object, Ambient,
    d.Fog and Back pic pages; changes repaint the picture at once, without a
    new calculation.
  * **Post processing:** normals on the z-buffer, hard and ambient shadows,
    reflections and transparency, depth of field.
  * **Navigator:** walk and look around with keys and mouse, then *View to
    main*.
  * **Animation maker, Batch processing, Big renders, Monte Carlo, Voxel
    export, Bulb Tracer 2** (meshes) with the mesh preview, the **HeightMap
    generator, MutaGen, ZBuf16Bit, Map sequences, Ini dirs** — all as in
    MB3D.
  * **Swarm** (new, on the Tools tab): your computers render together,
    see [Rendering on several computers](#rendering-on-several-computers-the-swarm).

  </details>
* **mb3d** — a command line renderer for scripts and batch jobs: pictures,
  animations, batch lists, Monte Carlo renders, voxel and mesh exports,
  Gaussian splats, all without a window.

## Is it safe to run? Check it yourself

You do not have to take our word for it. [scanrepo.dev](https://www.scanrepo.dev)
is an independent service that examines a GitHub repository for the signs
of harmful software: code that starts other programs, connects to the
internet, hides what it does, or bypasses the safety checks of the
programming language. Anyone can run it on this repository at any time:

**[▶ Scan mb3d-rs on scanrepo.dev](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
(click *Rescan* for a fresh result of the current code)

Version 0.3.1 was rated **safe: 8 out of 100** (scanned 5 November 2026,
rules of the scanner's engine 10). Its warnings are about the lines that start
the formulas the program compiles at run time for speed: running freshly
made machine code cannot be written in Rust's checked form, and those lines
and the bridge to Intel's noise filter are the only places the program
allows it; the rest is an informational note. (Earlier: 0.2.85 was rated
5 out of 100; 0.2.100 was rated dangerous for a few hours, because two math
functions of the formula compiler had a name the scanner takes for running
hidden code; they were renamed the same day, and a test now keeps such names
out; 0.2.66 was rated
0 out of 100 by an older engine, see also
[the 0.2.44 result](docs/img/scan_0.2.44.png).) An earlier version was flagged,
and we changed the code in response ([issue #4](https://github.com/thr32423/mb3d-rs/issues/4));
it is now written to stay this way. The programs contain no code that
connects to the internet; the only other program they ask your system to
open is your file manager, when you click a button to open a folder.

A scan looks for known warning signs; it is not a guarantee. These let
you check further on your own:

* **All of the source code is here.** The downloads are built from it by
  GitHub's own build servers, by the steps you can read in
  [the release workflow](.github/workflows/release.yml), not on anyone's
  personal computer.
* **Every release lists checksums** (`SHA256SUMS`), so you can confirm
  that the file you downloaded is exactly the one that was published;
  the one-line install below does this for you. Releases from 0.2.45 on
  also carry GitHub's build attestations: `gh attestation verify
  FILE --repo thr32423/mb3d-rs` shows which commit a download was built
  from.

## What is new compared with Mandelbulb3D

* **The graphics card calculates the picture** — the image itself, the
  shadows, the ambient occlusion, the reflections and the Monte Carlo
  renders. On a typical computer this is several times faster than the
  processor alone; see [Speed](#speed) below.
* **Auto mode**: the graphics card and the processor work on every picture
  together, and nothing ever waits — if the card is not ready, the
  processor starts anyway and the card joins in.
* **An LED in the title bar** shows where the last picture was calculated.
* **Interface size** from 75 to 200 % for high resolution screens.

  <a href="docs/img/themes_150.png"><img src="docs/img/themes_150.png" width="300" alt="Visual themes at 150 %"></a>

  <details><summary>Read more</summary>

  **Prefs ▸ Visual themes ▸ Interface size** makes all windows and their
  text larger or smaller (75 to 200 %), for high resolution screens; the
  layouts stay exactly as in MB3D.
  
  | 100 % | 150 % |
  |---|---|
  | ![Visual themes at 100 %](docs/img/themes_100.png) | ![Visual themes at 150 %](docs/img/themes_150.png) |

  </details>
* **Eight looks** for the windows, all of them theme files you can copy and
  change: two reproduce MB3D's looks, six are new.

  <a href="docs/img/themes/Midnight.jpg"><img src="docs/img/themes/Midnight.jpg" width="300" alt="The Midnight theme"></a>

  <details><summary>Read more</summary>

  **Prefs ▸ Visual themes** offers eight looks, each a file in the `Themes`
  folder: *Glossy* (the dark default) and *Windows* reproduce MB3D's own
  two looks, the other six are new. Click a picture for its full size.
  
  | | |
  |---|---|
  | <a href="docs/img/themes/Glossy.jpg"><img src="docs/img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="docs/img/themes/Windows.jpg"><img src="docs/img/themes/Windows.jpg" width="380" alt="Windows"></a> |
  | **Glossy**: MB3D's default | **Windows**: MB3D's "Themes off" |
  | <a href="docs/img/themes/Graphite.jpg"><img src="docs/img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="docs/img/themes/Daylight.jpg"><img src="docs/img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
  | **Graphite**: lighter neutral grey, steel-blue accents | **Daylight**: warm light grey, teal accents |
  | <a href="docs/img/themes/Midnight.jpg"><img src="docs/img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="docs/img/themes/Ember.jpg"><img src="docs/img/themes/Ember.jpg" width="380" alt="Ember"></a> |
  | **Midnight**: deep navy, blue accents | **Ember**: warm charcoal, orange accents |
  | <a href="docs/img/themes/Forest.jpg"><img src="docs/img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="docs/img/themes/Highcontrast.jpg"><img src="docs/img/themes/Highcontrast.jpg" width="380" alt="High contrast"></a> |
  | **Forest**: dark green-grey, green accents | **High contrast**: black, white text, yellow highlights |
  
  A theme is a small text file in the `Themes` folder next to the program;
  copy one, change its colours, and it appears in the list at the next
  start.

  </details>
* **A crash leaves a note on every system**: a panic of the program (or
  the error it ends with) is appended to `mb3d_crash.txt`, next to the
  program or in its data folder (before only on Windows). Send that file
  with a bug report.
* **All the graphics cards of the computer calculate**, with nothing to
  set: every card found takes bands of rows of the picture from the same
  pool as the first card and the processor, so a faster card simply
  takes more; the shadows, the occlusion, the Monte Carlo rays and the
  reflections are shared the same way, and an animation renders a frame
  per card at once. A slow card (integrated graphics beside a dedicated
  one) does what it can and never holds a picture up: once the free rows
  are gone, the idle cards and the processor take over the rows it has
  not delivered. A card that fails is left out, the others carry on.
  `mb3d --list-cards` shows what was found. (Tested with one card opened
  twice and with a software Vulkan driver as a second card; not yet on a
  computer with two real cards.)
* **An unfinished picture is a `.m3i` file.** What a stopped, crashed or
  killed render has calculated is kept in the cache (the Unfinished
  renders window, `mb3d resume`); for a picture that file is now a
  `.m3i` image file at every moment, `image.m3i` in the render's folder,
  with the rows not calculated yet as background. Copy it out whenever
  you like, or press *Save m3i* in the Unfinished renders window (`mb3d
  resume --m3i KEY file.m3i` on the command line; both also write the
  `data.bin` of an earlier version as a `.m3i`), and open it here or in
  Mandelbulb3D, no conversion needed. (Monte Carlo renders keep their
  own records, the `.m3c` of a pass is written with `--m3c`.)
* **Hotkeys for the windows**: a key combination brings the main window,
  the Navigator, the Animation, Formulas, Lighting, Post processing or
  Monte Carlo window to the front from any window (Ctrl+1 to Ctrl+7), and
  Ctrl+W closes the window you are in (on macOS Cmd). Set your own under *Prefs > Hotkeys*: click a field and
  press the keys, Backspace takes a hotkey away.
* **The interface in your language**: every text of the windows comes
  from a language file; twelve languages ship, and the program writes the
  file of a new language for you to translate.

  <details><summary>Read more</summary>

  The language badge at the right end of the main window's title bar
  shows the interface language (`EN`); a click opens the language window.
  A language is a text file in the `Languages` folder next to the program.
  Shipped: English, Deutsch, Español, Français, Bahasa Indonesia, Italiano,
  Nederlands, Polski, Português, Kaszëbsczi, العربية, اردو, हिन्दी, தமிழ், 日本語,
  한국어, 简体中文 and 繁體中文 (each with its flag; the East Asian ones bring a
  subset of Noto Sans CJK for their characters, Hindi brings Noto Sans
  Devanagari and Tamil Noto Sans Tamil, whose joined letters and vowel
  signs are set with the rustybuzz text shaper; Arabic and Urdu are written
  right to left in Noto Sans Arabic).
  The translations were machine made and checked for structure, not by
  native speakers: corrections are welcome, each is one line of the file. To translate: type the name of your language and
  press *Create language file*; the program writes `Languages/<name>.lng`
  with every text in English, one per line (`Button2.Caption = Calculate
  3D`). Translate the texts right of the `=` with a text editor and press
  *Reload*: the windows follow at once, without a restart. A line left out
  keeps the English text. *Save + exit* keeps the language for the next
  start.

  </details>
* **Linux, Windows and macOS** (Intel and Apple Silicon).
* **A command line program** for everything that needs no window.
* **Very big pictures in one piece**: 15000 x 20000 pixels and more
  without running out of memory, and the depth of field uses all
  processor cores (11 times faster on 24 threads than before 0.2.45, with
  the same picture).
* **16 bit pictures:** PNG files with 65,536 instead of 256 steps per
  colour, for editing without banding.

  <a href="docs/img/png16_compare.png"><img src="docs/img/png16_compare.png" width="300" alt="16 bit pictures"></a>

  <details><summary>Read more</summary>

  MB3D saves 8 bit pictures: 256 steps per colour. That is plenty to look
  at, but when a picture is edited afterwards — brightness, contrast,
  curves — smooth areas such as the background or fog break into visible
  bands. **Save pic ▸ PNG** offers a second file type, **PNG 16 bit
  colour**: the colours as the program calculated them, before they are
  rounded to 256 steps, with all shadows, reflections and depth of field.
  The file looks the same, and stays smooth when it is edited.
  
  <a href="docs/img/png16_compare.png"><img src="docs/img/png16_compare.png" width="760" alt="A background gradient after a strong contrast increase: steps in 8 bit, smooth in 16 bit"></a>
  
  <a href="docs/img/save_png16.png"><img src="docs/img/save_png16.png" width="420" alt="The save dialog with 'PNG 16 bit colour'"></a>

  </details>
* **A noise filter** for Monte Carlo and all other pictures, the DCT
  denoising known from GIMP, and for Monte Carlo renders also a neural
  one, Intel Open Image Denoise.

  <a href="docs/img/mc_noise.png"><img src="docs/img/mc_noise.png" width="300" alt="Noise filter"></a>

  <details><summary>Read more</summary>

  A **noise filter** removes grain from the pictures: the **DCT denoising**
  of GIMP's *Filters ▸ Enhance ▸ Noise Reduction ▸ Denoise DCT*, which of
  GIMP's filters suited these pictures best (see the credits below).
  
  * **Monte Carlo window ▸ -noise** (a button MB3D had hidden): filters the
    shown picture, and the saved one, with a strength that follows the rays
    per pixel. The rendering itself goes on unfiltered, and its next update
    shows it unfiltered again.
  * **Post processing ▸ Noise Filter**: *Filter now* filters the picture of
    the main window; *Filter new pictures automatically* filters every new
    one. *Strength* is the strength of the noise to remove (4 is light;
    higher removes more grain and more fine detail).
  
  Monte Carlo pictures have the most grain. Against pictures rendered with
  512 rays per pixel, the filtered pictures were closer than the unfiltered
  ones in all 13 tests (four scenes, 4 to 32 rays): by up to 23 % at 4 rays,
  5 to 10 % at 8 rays, 1 to 3 % at 32 rays, where little grain is left.
  
  <a href="docs/img/mc_noise.png"><img src="docs/img/mc_noise.png" width="760" alt="A Monte Carlo picture with 4 rays per pixel, the same with the noise filter, and with 512 rays"></a>
  
  <a href="docs/img/postpro_noise.png"><img src="docs/img/postpro_noise.png" width="260" alt="Post processing: the Noise Filter section"></a>

  The **neural noise filter** is
  [Intel Open Image Denoise](https://www.openimagedenoise.org), the
  filter of Blender's Cycles and many other path tracers, trained on
  rendered pictures. It filters the rendering's averages of the rays
  (before exposure and saturation), so the picture keeps its brightness.
  Intel's library comes with the program (the `oidn` folder; in the .deb
  and .rpm, `/usr/lib/mb3d-rs/oidn`); `MB3D_OIDN` can name another one.

  * **Monte Carlo window ▸ -noise AI**: filters the shown picture, and the
    saved one, as *-noise* does; the rendering goes on unfiltered.
  * **Command line**: `mb3d montecarlo scene.m3p --rays 8 --denoise-ai`.
  * It takes 0.2 to 0.6 seconds for a 640 x 480 picture on the processor.

  The difference from the 512-ray picture (mean over all pixels, lower is
  better):

  | Scene | 4 rays | DCT filter | neural filter | 8 rays | DCT filter | neural filter |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  The neural filter is closest in three of the four scenes; on the Julia
  set's fine grain the DCT filter is a little closer. It shows most on
  glass and mirrors, where few rays leave the most grain:

  <a href="docs/img/mc_noise_ai.jpg"><img src="docs/img/mc_noise_ai.jpg" width="760" alt="A Monte Carlo picture with 4 rays per pixel, with the DCT filter, with the neural filter, and with 512 rays"></a>

  </details>
* **Transparent backgrounds** for pictures and animations.

  <a href="docs/img/transparent.png"><img src="docs/img/transparent.png" width="300" alt="Transparent background"></a>

  <details><summary>Read more</summary>

  Pictures and animation frames can be saved with a **transparent
  background**, to put the fractal onto another picture or into a video
  editor: **Save pic ▸ PNG ▸ PNG transparent background**, and in the
  Animation maker **Transparent background** (PNG frames). With anti-aliasing (a
  viewing scale of 1:2 or more, or `--aa`) the edges are soft, and they
  carry no fringe of the old background colour. Depth pictures for
  compositing were already there (*Save Z buffer too*, `--depth`).
  
  <a href="docs/img/transparent.png"><img src="docs/img/transparent.png" width="760" alt="A picture with its background, and with a transparent background shown over a checkerboard"></a>

  </details>
* **Videos of animations:** an AVI file straight from the program, or
  the ffmpeg command for a small MP4.

  <a href="docs/img/anim_video.png"><img src="docs/img/anim_video.png" width="300" alt="Video of an animation"></a>

  <details><summary>Read more</summary>

  In the Animation maker, **Also make a video (AVI)** joins the frames into
  a video when they are rendered, at the **Frames per second** you set:
  `<project name>.avi` in the output folder, next to the frames, which are
  kept. It plays in VLC and mpv and opens in video editors (Kdenlive,
  Shotcut, DaVinci Resolve). Each frame is stored as a whole picture, so the
  files are big: good for editing and keeping, less for sharing.
  
  For a small video to share, **FFmpeg command** puts the command for an MP4
  (or, with a transparent background, a WebM) on the clipboard and in the
  messages. Install [ffmpeg](https://ffmpeg.org), paste the command into a
  terminal when the frames are done, and it makes the video; this program
  starts no other programs itself.
  
  <a href="docs/img/anim_video.png"><img src="docs/img/anim_video.png" width="760" alt="The Animation maker with Also make a video (AVI), Frames per second and FFmpeg command"></a>

  </details>
* **A preview of parameter files** in the Open dialog.

  <a href="docs/img/open_preview.png"><img src="docs/img/open_preview.png" width="300" alt="Preview when opening parameters"></a>

  <details><summary>Read more</summary>

  **Open ▸ Open m3p** and **Open m3i** show a small picture of the
  clicked parameter file, with its size and formulas, before it is
  opened; the arrow keys move through the list with the preview. The
  picture is calculated in the background on the processor, so the
  dialog stays responsive, and each file's preview is kept while the
  program runs.
  
  <a href="docs/img/open_preview.png"><img src="docs/img/open_preview.png" width="760" alt="The Open dialog with the preview of a parameter file"></a>

  </details>
* **Navigator: exact values and Reset view.** Click a value and type it;
  go back to the view the Navigator started from.

  <a href="docs/img/navigator_values.png"><img src="docs/img/navigator_values.png" width="300" alt="Navigator: exact values and Reset view"></a>

  <details><summary>Read more</summary>

  * **Click a value** in the Navigator's panels — Julia values, formula
    values, 4D rotation, the Misc values — and type the exact number, as in
    MB3D (angles in degrees). The slider then continues from that value.
  * **Click the zoom** and type it, when the zoom steps are too coarse (a
    wish from MB3D's issue list); a typed zoom stays (*Fixed zoom*).
  * **Reset view** goes back to the position, direction and zoom the
    Navigator took from the main window; the adjusted values stay.
  
  <a href="docs/img/navigator_values.png"><img src="docs/img/navigator_values.png" width="640" alt="The Navigator with the formula values and the input box of a clicked value"></a>

  </details>
* **A new formula, SphereTree:** a tree of touching spheres that branch
  smaller and smaller, by Tom Lowe.

  <a href="docs/img/spheretree_ball.jpg"><img src="docs/img/spheretree_ball.jpg" width="300" alt="A new formula: SphereTree"></a>

  <details><summary>Read more</summary>

  **SphereTree** is a formula Mandelbulb3D does not have: a tree of spheres
  that touch each other and branch into smaller and smaller spheres, like
  foam or coral. Tom Lowe, who invented the Mandelbox, published it in 2020
  (see the credits below). It is in the formula list with the dIFS formulas
  and has four settings:
  
  * **PackRatio** (0 to 1.22): the size of the spheres; at 1 they just
    touch, above 1 the outer ones break off.
  * **Levels**: how deep the tree goes (6 is plenty for most pictures).
  * **Rotated**: 1 turns every level a twelfth of a turn, and the tree
    becomes a round ball of sphere clusters.
  * **Colour scale**: every branch of the tree has its own colour. Choose
    *2nd choice* (the orbit trap) in the colour settings of the Lighting to
    see them; this setting spreads them over the colour gradient.
  
  It works alone and after dIFS transforms (the third picture repeats it
  with *tilingbrickIFS*), and it is calculated on the graphics card as well
  as on the processor. Set the iterations to 1: the formula calculates the
  whole tree at once. Parameters with SphereTree open only in this program,
  not in Mandelbulb3D.
  
  <a href="docs/img/spheretree_ball.jpg"><img src="docs/img/spheretree_ball.jpg" width="760" alt="A SphereTree ball (Rotated) in the Monte Carlo renderer, at dusk"></a>
  
  | <a href="docs/img/spheretree_close.jpg"><img src="docs/img/spheretree_close.jpg" width="380" alt="A close-up of the sphere foam of SphereTree"></a> | <a href="docs/img/spheretree_swarm.jpg"><img src="docs/img/spheretree_swarm.jpg" width="380" alt="SphereTree repeated with tilingbrickIFS: a swarm of sphere trees"></a> |
  |:---:|:---:|

  </details>
* **A new formula, DomeTree:** a dome that sprouts a ring of smaller
  domes, each with its own ring, by Tom Lowe.

  <a href="docs/img/dometree_mc.jpg"><img src="docs/img/dometree_mc.jpg" width="300" alt="A new formula: DomeTree"></a>

  <details><summary>Read more</summary>

  **DomeTree** is Tom Lowe's second tree after SphereTree (see the
  credits): a ball whose one side grows a ring of domes, each dome its
  own smaller ring, level after level, made of circle inversions. It is
  in the formula list with the dIFS formulas and has five settings:

  * **Levels**: how deep the tree goes (16 is plenty).
  * **Children**: the domes in each ring, 4 to 10.
  * **Bend angle** (0.3 to 1.3 work best): how far the domes bulge; low
    values give flat discs, high ones round domes. Above about 1.3 the
    surface turns grainy.
  * **Colour scale**: every dome has its own colour. Choose *2nd choice*
    (the orbit trap) in the colour settings of the Lighting and a wide
    colour range (for example 0 to 100) to see them.
  * **Start curved**: 1 (the default) starts with the ball, 0 with a flat
    floor that grows the first ring.

  Use a small *Raystep* (0.2; 0.05 with 9 or 10 children): the shape's
  distance estimate is rough, as in TGlad's own Fragmentarium file. Like
  SphereTree it runs on the graphics card and the processor, needs only
  1 iteration, and parameters with it open only in this program.

  <a href="docs/img/dometree_mc.jpg"><img src="docs/img/dometree_mc.jpg" width="760" alt="DomeTree with 8 children in the Monte Carlo renderer, coloured per dome"></a>

  | <a href="docs/img/dometree_5.jpg"><img src="docs/img/dometree_5.jpg" width="250" alt="DomeTree, 5 children, bend angle 1.2"></a> | <a href="docs/img/dometree_flat.jpg"><img src="docs/img/dometree_flat.jpg" width="250" alt="DomeTree, 6 children, bend angle 0.5: flat domes"></a> | <a href="docs/img/dometree_10.jpg"><img src="docs/img/dometree_10.jpg" width="250" alt="DomeTree, 10 children: lace between the domes and a large ball"></a> |
  |:---:|:---:|:---:|

  </details>
* **A new formula, UlamSpiral:** Ulam's spiral of prime numbers in 3D, as
  a city, a skyline, spheres or a block.

  <a href="docs/img/ulam_2d.png"><img src="docs/img/ulam_2d.png" width="300" alt="A new formula: UlamSpiral"></a>

  <details><summary>Read more</summary>

  In 1963 the mathematician Stanisław Ulam sat through "a long and very
  boring paper" at a meeting and doodled on a sheet of squared paper. His
  first idea was to set up chess problems; instead he numbered the squares
  1, 2, 3, … in a spiral, 1 in the middle, and circled the prime numbers.
  The primes lined up on diagonal lines, as if they followed a pattern, and
  they still do when a computer draws millions of them (Ulam tried 100,000
  on the MANIAC II at Los Alamos; Martin Gardner put the picture on the
  cover of Scientific American in 1964). Nobody has fully explained it to
  this day.
  
  <a href="docs/img/ulam_2d.png"><img src="docs/img/ulam_2d.png" width="420" alt="The Ulam spiral: the primes among the numbers 1 to 78,961 on a square spiral, with the start of the spiral drawn in the corner"></a>
  
  **UlamSpiral** builds that spiral in 3D, one shape per number, in four
  forms (*Mode*):
  
  * **0, prime towers**: a plate for every number and a tower on each
    prime. The diagonal lines of primes become rows of towers.
  * **1, divisor skyline**: a column for every number, the higher the more
    divisors it has; the primes are the low streets between them.
  * **2, divisor spheres**: a sphere for every number, the larger the more
    divisors it has (the primes are the small ones).
  * **3, a block of primes**: the numbers wound around cube shells instead
    of a square, a cube on each prime.
  
  *Rings* sets the size of the spiral (60 rings are the numbers up to
  14,641), *Height* the height of the towers or columns, *Gap* the space
  between them. With *2nd choice* (the orbit trap) in the colour settings
  the colour follows the number of divisors, so the primes have a colour
  of their own. Like SphereTree it runs on the graphics card and the
  processor, works after dIFS transforms, needs only 1 iteration, and
  parameters with it open only in this program.
  
  <a href="docs/img/ulam_city.jpg"><img src="docs/img/ulam_city.jpg" width="760" alt="UlamSpiral, prime towers: the diagonal rows of prime numbers as a city, in the Monte Carlo renderer with fog, hard shadows and depth of field"></a>
  
  | <a href="docs/img/ulam_skyline.jpg"><img src="docs/img/ulam_skyline.jpg" width="250" alt="UlamSpiral, divisor skyline"></a> | <a href="docs/img/ulam_spheres.jpg"><img src="docs/img/ulam_spheres.jpg" width="250" alt="UlamSpiral, divisor spheres"></a> | <a href="docs/img/ulam_block.jpg"><img src="docs/img/ulam_block.jpg" width="250" alt="UlamSpiral, a block of primes on cube shells"></a> |
  |:---:|:---:|:---:|

  </details>
* **A stopped render continues where it was.** What a render calculated
  is kept when it is stopped, and also when the program is closed or
  crashes, or the computer goes off (it is saved to the disk every 5
  minutes): Calculate 3D, Monte Carlo (also in the middle of a pass), the
  frame of an animation or the picture of a batch under way, a voxel
  export, a Bulb Tracer mesh, Big render tiles, and a swarm render (Stop
  on the computer that made the job keeps every finished part, and each
  computer keeps the part it was at). Calculate 3D and Monte Carlo of the
  same parameters ask whether to continue, the others continue by
  themselves; the **Resume** button (Open tab) lists the unfinished
  renders, to continue or delete them. On the command line the same
  command continues by itself (also a picture in `--tiles`, `montecarlo`,
  `animate`, `batch`, `voxel` and `mesh`); `--fresh` starts again, `mb3d
  resume` lists them. A continued render is the same as one calculated in
  one go.
* **Rendering shared between your computers**, a "swarm": computers that
  see one shared folder render a picture, a Big render, a Monte Carlo
  picture or an animation together. See
  [Rendering on several computers](#rendering-on-several-computers-the-swarm).
* **Community formulas and maps**: 129 formulas and 154 maps that the
  official repository lacks, from the Mandelbulb3D 1.99.37 package that
  Machina Infinitum offers as a free download. The formulas are by Edgar
  Malinovsky, gnj, DarkBeam (Luca GN), bugman, Aexion, Buddhi, pupukuusikko,
  Jos Leys, Knighty, trafassel, Nrx, Eiffie, Quasihedron, Inigo Quilez,
  Yannis Picart, Jotero and others whose files name no author;
  `M3Formulas/CREDITS-community.md` names the author of each (`teapot.m3f`
  by DarkBeam is under CC BY-SA 3.0). The maps are from DarkBeam's
  collection, under his terms (free to use, not to be sold unchanged), with
  pictures shared on DeviantArt by CmdrChaos, Fractales75, Nic022 and
  UndeadAcademy; maps 3046–3048 are by Kronpano
  (`M3Maps/CREDITS-community.md`). The 608 light presets in `M3Lights` come from the same package; their
  files name no maker (`M3Lights/CREDITS-community.md`).
* **Ideas from Mandelbulb3D SE**: Glenn Olander's Mandelbulb3D SE, a macOS
  app built on mb3d-rs, gave two ideas: checking every graphics card shader
  in the tests without a graphics card, and Stop ending the painting of a
  picture at once.
* **A new formula, PseudoKleinian:** the "Kleinian architecture" of
  halls, arches and spheres as one formula with its own exact distance.

  <a href="docs/img/kleinian_hall.jpg"><img src="docs/img/kleinian_hall.jpg" width="300" alt="A new formula: PseudoKleinian"></a>

  <details><summary>Read more</summary>

  The halls, arches, pillars and nested spheres known as "Kleinian
  architecture" are among the best-loved 3D fractal pictures. In MB3D they
  can only be made as a hybrid of several formulas with carefully matched
  settings, the recipe Theli-at found in 2011; knighty then wrote the same
  thing as one formula with its own distance estimate, and that is
  **PseudoKleinian**. Each level folds the point into a box, inverts it in
  a sphere and adds an offset, and the distance is measured exactly to a
  floor plane after the folds, so the walls come out clean and the picture
  renders fast. The settings are knighty's: *Levels* (8), *Size* (the
  inversion sphere), *CSize X/Y/Z* (the box; his 0.924, 0.908, 0.924 give
  the classic halls, and small changes give new buildings), *Julia X/Y/Z*
  (an offset per level), *Shape* (0 the floor plane, 1 a cylinder-plane of
  *Thickness* through *Offset*, 2 a rounded box), *DE offset* (hollows the
  surface) and *Colour scale* (with *2nd choice*, the orbit trap: the
  smallest orbit radius, the classic Kleinian banding).
  
  The structure fills all of space, so you look at it from inside: start
  with the view centre near the origin and the camera a unit or two back,
  and move through the halls with the Navigator. In the Monte Carlo
  renderer an interior gets almost no sky light, so put a positional light
  into the hall (the first picture) or cut the structure open with a
  cutting plane and look in from above (the second). It runs on the graphics
  card and the processor, needs 1 iteration, works after dIFS transforms
  and in DE combinations like the other dIFS formulas (though inside a
  space-filling structure those change little), and parameters with it
  open only in this program.
  
  <a href="docs/img/kleinian_hall.jpg"><img src="docs/img/kleinian_hall.jpg" width="760" alt="PseudoKleinian from knighty's viewpoint: a hall of arches and nested spheres lit by a lamp inside, in the Monte Carlo renderer with fog, hard shadows and depth of field"></a>
  
  <a href="docs/img/kleinian_top.jpg"><img src="docs/img/kleinian_top.jpg" width="760" alt="PseudoKleinian cut open with a cutting plane and seen from above: the lattice of spheres and the cells between them"></a>

  </details>
* **Stereo pairs in one step**: both eye images rendered and shown side by
  side, for parallel or cross-eyed viewing. Contributed by Rathinagiri S
  ([SRathinaGiri](https://github.com/SRathinaGiri)).

  <a href="docs/img/stereo_pair.jpg"><img src="docs/img/stereo_pair.jpg" width="300" alt="A stereo pair of Dainbramage's Hydra, side by side for parallel viewing"></a>

  <details><summary>Read more</summary>

  **Stereo ▸ Render stereo pair** renders the left and then the right eye
  image with the tab's stereo settings and shows them side by side in the
  main window. They are placed for parallel viewing (left image on the
  left); **Cross-eyed preview** swaps them for cross-eyed viewing. It is a
  preview: lighting and colours
  cannot be changed afterwards without rendering again, and it does not
  work with tiles. The buttons for the single eye images are still there.
  
  <a href="docs/img/stereo_pair.jpg"><img src="docs/img/stereo_pair.jpg" width="760" alt="A stereo pair of Dainbramage's Hydra, side by side for parallel viewing"></a>

  </details>
* **Time left for the whole picture**: the bar under the picture fills once
  over the whole calculation, and beside it the time left until the picture
  is finished, shadows, occlusion and reflections included.

  <a href="docs/img/time_left.png"><img src="docs/img/time_left_bar.png" width="300" alt="The progress bar and the time left in total"></a>

  <details><summary>Read more</summary>

  MB3D's bar starts again at every stage. Here it shows the whole
  calculation: the main rendering, then the hard shadows and ambient
  occlusion, the reflections and the depth of field. How long the stages
  after the main rendering take is estimated from what the scene asks for
  and how the main rendering went; the estimate was fitted on about 1400
  renders of 366 scenes. A picture calculated again expects the times it
  took the last time.
  
  The first picture with new formulas on the graphics card waits for its
  shaders to compile: the line then says *shaders compiling* with the time
  left, which takes the processor's pace and, when the shaders will be
  ready in time, the card joining in. Details in
  [TECHNICAL.md](docs/TECHNICAL.md#time-left-and-the-progress-bar).
  
  <a href="docs/img/time_left.png"><img src="docs/img/time_left.png" width="640" alt="The main window during the hard shadows and ambient occlusion, with the time left in total"></a>

  </details>
* **History** behind the clock icon of the title bar: every picture you
  calculated with its thumbnail, every file you opened or saved, as a list,
  as rows or as tiles of any size, grouped by day or in one run; search
  it, sort it, show only the pictures that differ, and load any entry
  with a double click. A history of hundreds of thousands of entries
  opens at once.
* **AI assistants can use mb3d** (`mb3d mcp`, the Model Context
  Protocol): ask Claude's desktop app or Claude Code in plain words, or by
  voice through the assistant, to describe, change and render your
  parameter files, breed variations or render what you worked on last;
  the assistant sees the pictures and can adjust them. The **AI** button
  on the Prefs tab connects it with a click (or `mb3d mcp --setup`). mb3d stays offline (the assistant's app starts it on your
  computer), writes only into the `AI` folder and never deletes anything;
  pictures above 4K need your agreement.
* **A log of everything** behind the page icon of the title bar: every
  click, setting and typed value, every calculation with its stages and
  times, files opened and saved, windows, messages, warnings and errors,
  in a window that follows them live and in a file a day. A session that
  froze or was killed is reported at the next start, and a crash report
  carries the log's last lines.
* **Lighting stays live** after hard shadows and reflections: move a light
  or change its colour and the picture changes at once, as in Mandelbulb3D.

  <a href="docs/img/lighting_live.png"><img src="docs/img/lighting_live_small.png" width="300" alt="The main window and the Lighting window right after a lighting change"></a>

  <details><summary>Read more</summary>

  Render plainly, then try out light positions and colours: each change in
  the Lighting window repaints at once with the shadows as they were
  calculated, as in Mandelbulb3D. Changing a light's colour or intensity
  never calculates the shadows again. The shadows of a light you moved,
  the reflections and the depth of field come back with the buttons of
  the Post processing window, as in Mandelbulb3D.
  
  A calculation you stopped can also still be continued after you changed
  the lighting. A way of working with it: [Lighting workflow](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow)
  in the wiki. Details in
  [TECHNICAL.md](docs/TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field).
  
  <a href="docs/img/lighting_live.png"><img src="docs/img/lighting_live.png" width="760" alt="The main window and the Lighting window right after a lighting change: the picture already brighter"></a>

  </details>
* **A ground plane or water** under (or behind) the fractal, an idea from
  Mandelbulber: a plane across the X, Y or Z axis, calculated together
  with the fractal, with its shadows, ambient occlusion, reflections and
  Monte Carlo light, in its own colour, flat or with waves.

  <a href="docs/img/ground_water.jpg"><img src="docs/img/ground_water.jpg" width="760" alt="A floor with the fractal's shadow, reflecting water with waves, and a Monte Carlo render on a green floor"></a>

  <details><summary>Read more</summary>

  Main window, page *Cutting*: tick *Ground plane*, choose the axis and
  the position, the colour, how much it reflects (0 to 1: a mirror when
  the reflections are calculated) and the waves' height and length. The
  solid side is always the one away from the camera. It is calculated on
  the graphics card and on the processor, and left out of the voxel and
  mesh exports. Mandelbulb3D has no place for it in its parameters:
  mb3d-rs keeps it in a small block after them (in .m3p and .m3i files,
  and as `{mb3d-rs: ...}` after the title of text parameters), which
  Mandelbulb3D does not read. Scene keys `ground`, `ground_color`,
  `ground_reflect`, `ground_waves`; details in
  [TECHNICAL.md](docs/TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field).

  **Transparent water**: with *Water* above 0 (how much light it lets
  through) a fractal half under water shows beneath the surface, bent by
  the refraction (*Refraction*: 1.33 for water) and turning into the
  colour of deep water (the colour button) with depth: the *Absorption*
  says how murky it is (0 clear, 3 murky), and in a
  blue-green water red fades first, so what lies under it turns
  turquoise. The lights' glints on the waves stay on the surface. The
  normal render shows it with the reflections on (Post processing),
  Monte Carlo always; without the reflections the water stays opaque.
  Scene keys `ground_transparency`, `ground_absorption`,
  `ground_water_color`, `ground_ior`; an example in
  [CLOUDS_WATER.md](docs/CLOUDS_WATER.md#transparent-water).

  **Scattering** (the row below *Refraction*): the water scatters light
  as real water does. It glows turquoise looking towards the sun, the
  wave crests shine through, the sun's light falls into it as shafts cut
  by the fractal's shadows, and what lies under the water glows softly
  in its colour; Monte Carlo traces the light through the water as it
  scatters. The second field is its direction (0.8: mostly forward). 0
  is the look without it. Scene keys `ground_scattering`,
  `ground_scatter_dir`; see
  [CLOUDS_WATER.md](docs/CLOUDS_WATER.md#transparent-water).

  **Try it**: on the *Cutting* page tick *Ground plane* and put its height
  through the fractal, set *Water* to 1, *Absorption* to 1 and
  *Scattering* to 0.5, and switch the reflections on (*Postprocess*,
  *Reflections + Transparency*, *Calculate R. (+ T.) automatically*),
  then *Calculate 3D*. Three finished scenes to paste, with their settings:
  [WATER_EXAMPLES.md](docs/WATER_EXAMPLES.md).

  *Water* (transparency) 0, 0.5 and 1:

  <a href="docs/img/water_transparency.jpg"><img src="docs/img/water_transparency.jpg" width="760" alt="Transparency 0, 0.5 and 1: grey opaque water, then the submerged half of the bulb showing through in turquoise"></a>

  *Absorption* 0.3, 1 and 3, from clear to murky:

  <a href="docs/img/water_absorption.jpg"><img src="docs/img/water_absorption.jpg" width="760" alt="Absorption 0.3, 1 and 3: the submerged half clearly green, fading, then hidden"></a>

  *Scattering* 0, 0.5 and 1 looking towards the sun, and the light shafts
  with a low sun in front:

  <a href="docs/img/water_scattering.jpg"><img src="docs/img/water_scattering.jpg" width="760" alt="Scattering 0, 0.5 and 1 looking towards the sun: dark water, then glowing turquoise water"></a>

  <a href="docs/img/water_shafts.jpg"><img src="docs/img/water_shafts.jpg" width="760" alt="A low sun in front: the fractal's shadow volume stretching through the water as a blue shaft"></a>

  **The waves** are an irregular sea of sixteen waves along one wind
  direction (before 0.2.98 three regular waves made a checkerboard;
  scenes with waves look different since). Old (left of each pair) and
  new, small and large waves, the sun in front and behind:

  <a href="docs/img/water_waves.jpg"><img src="docs/img/water_waves.jpg" width="760" alt="Old and new waves side by side: the old regular grid of bright diamonds, the new irregular sea"></a>

  </details>
* **Light from an HDR picture**: a 360° Radiance `.hdr` picture as the
  background lights a Monte Carlo render, with its full brightness, and
  its sun becomes a light with soft shadows (an idea from Mandelbulber).

  <a href="docs/img/hdr_sun.jpg"><img src="docs/img/hdr_sun.jpg" width="640" alt="A Monte Carlo render lit only by an HDR sky: left the sun found by chance with bright speckles, right the sun as a light"></a>

  <details><summary>Read more</summary>

  Choose the `.hdr` file as the background picture in the Lighting window
  (not "direct"). Monte Carlo then lights the scene with the sky, and the
  brightest spot of the picture, its sun, is taken out and added as a light
  with shadows of the sun's size: found only by chance, the sun would leave
  bright speckles (left). Background pictures of 8 or 16 bits stay as they
  were.

  </details>
* **OpenEXR pictures** for compositing (Blender, Nuke, DaVinci Resolve):
  the colours in floating point, not clipped above white, with the layers
  opacity, depth and normals. *Save pic* > *OpenEXR*, or `mb3d -o
  picture.exr` (also with tiles and the swarm).

  <a href="docs/img/exr_layers.jpg"><img src="docs/img/exr_layers.jpg" width="760" alt="The layers of an OpenEXR picture: the colours as saved, the colours as stored (linear), the normals and the depth"></a>

  <details><summary>Read more</summary>

  The colours are linear, as OpenEXR expects: shown with the usual display
  curve they look as the PNG, and the highlights keep their brightness above
  white. The layers: `A` (opacity, as for a transparent PNG), `Z` (the
  distance from the camera in fractal units) and `N.X`, `N.Y`, `N.Z` (the
  surface normals). Details in [TECHNICAL.md](docs/TECHNICAL.md).

  </details>
* **A fisheye camera and 360° stereo for VR** (ideas from Mandelbulber):
  a fisheye of up to 360°, and panoramas for VR headsets with depth in
  every direction.

  <a href="docs/img/fisheye_water.jpg"><img src="docs/img/fisheye_water.jpg" width="640" alt="A fractal on water through the fisheye at 120 and 200 degrees: the horizon curves"></a>
  <a href="docs/img/vr_panorama.jpg"><img src="docs/img/vr_panorama.jpg" width="300" alt="A 360 degree stereo panorama, the left eye above the right"></a>

  <details><summary>Read more</summary>

  Main window, page *Camera*: *Fisheye* (the field of view, FOVy, up to
  360°, spans the picture's height; a square picture shows a round view).
  A 360° panorama with stereo gives each ray its own eye position, turned
  with the direction it looks in, so the depth is right all around:
  `mb3d -s optic=2 --stereo-pair top-bottom` writes both eyes, the left one
  above, as VR players expect (a 2:1 picture per eye). The eyes are 6.5 cm
  apart, the point looked at as far away as the stereo viewing distance
  (`stereo_screen`'s second value, 2 m). Mandelbulb3D has no fisheye: the
  mb3d-rs block of the parameters keeps it.

  </details>
* **Volumetric clouds** (an idea from Mandelbulber): a layer of clouds
  above, below or around the fractal, lit by the scene's light, in front of
  and behind the fractal.

  <a href="docs/img/clouds.jpg"><img src="docs/img/clouds.jpg" width="480" alt="A fractal over the sea under a sky of clouds"></a>

  Three examples with clouds, water and reflections, with their parameters
  to paste in and where each setting is:
  [docs/CLOUDS_WATER.md](docs/CLOUDS_WATER.md).

  <details><summary>Read more</summary>

  Post processing window, section *Clouds*: tick *Paint clouds*, set the
  layer (an axis and two heights on it), how thick the clouds are, how much
  of the sky they cover, the size of their shapes and their colour, then
  *Paint now*. They are painted over the finished picture in a fraction of
  a second, so trying out settings is quick; each pixel looks through the
  layer up to the surface it hit. An animation can move them
  (`cloud_offset` between keyframes). Reflections show them, and Monte Carlo too; Mandelbulb3D does not have them (the mb3d-rs block of the
  parameters keeps them). They cast their shadows on the fractal and the ground, also from outside the picture.

  </details>
* **Surface materials** (ideas from Mandelbulber) on top of the palette:
  Fresnel (more reflection at grazing angles, as glass and lacquer have),
  iridescence (the shimmering colours of a thin film) and glow.

  <a href="docs/img/material.jpg"><img src="docs/img/material.jpg" width="760" alt="Iridescence, iridescence with Fresnel and reflections, and glow"></a>

  <details><summary>Read more</summary>

  Post processing window, section *Material*: tick *Surface material*, set
  Fresnel (0 to 1), iridescence (0 to 1, and a shift of its colours) and glow,
  then *Paint now*. Fresnel shows best with the reflections calculated. Off,
  the pictures are exactly Mandelbulb3D's; the ground plane keeps its own
  colour and reflection. Also in Monte Carlo.

  </details>
* **Two fractals in one scene** (an idea from Mandelbulber): with a DE
  combination, the second hybrid part can stand apart from the first,
  moved, turned and scaled, combined as union, intersection or cut.

  <a href="docs/img/two_objects.jpg"><img src="docs/img/two_objects.jpg" width="360" alt="A Mandelbulb and an Amazing Box side by side"></a>

  <details><summary>Read more</summary>

  Formula window, with a DE combination chosen: *Part 2 at* (its middle),
  *Size* and *Turned* (degrees about x, y and z). Union (*Min*) puts the two
  objects side by side, intersection (*Max*) keeps only what both share, the
  inverted max cuts the second out of the first, the smooth union melts them
  together. Calculated on the graphics card and the processor; the mb3d-rs
  block of the parameters keeps the place (Mandelbulb3D shows the two parts
  in one place).

  </details>
* **Animation curves** (an idea from Mandelbulber): a value of the scene
  over all frames of an animation, and its keyframes as handles to drag.

  <a href="docs/img/curves.png"><img src="docs/img/curves.png" width="560" alt="The Curves window: the zoom over all frames, the keyframes as squares, one chosen"></a>

  <details><summary>Read more</summary>

  *Animation maker* > *Curves*: choose the zoom, a coordinate of the position,
  an angle of the rotation, the field of view, Z start, the ground's position
  or the clouds' drift, and see it over every frame as the animation
  interpolates it. The keyframes are squares on their frames: drag one up or
  down, or click it and type a value. The quadratic Bezier interpolation
  passes beside the keyframes, and the curve shows that before anything is
  rendered. A third interpolation, *Catmull-Rom* (not in MB3D), is smooth and passes through every keyframe (`mb3d animate --catmull-rom`).

  </details>
* **Finishing the picture**: exposure, a glow around bright light, filmic
  tone mapping, contrast, saturation, warmth, a vignette and the colour
  fringes of a lens, as a last pass over the whole picture.

  <a href="docs/img/finishing.jpg"><img src="docs/img/finishing.jpg" width="760" alt="A temple at dawn as calculated, and finished: glow around the bright pillars, softer highlights, warmer light, darker corners"></a>

  <details><summary>Read more</summary>

  Post processing window, section *Finishing*: tick *Finish the picture*,
  set the exposure in stops, *Filmic* (bright light rolls off softly instead
  of turning flat white), the glow (strength, reach as a part of the
  picture's diagonal, and from which brightness it glows), contrast,
  saturation, warmth, vignette and colour fringes, then *Paint now*. The
  painting then runs in floating point, so the light above white is kept
  until the tone mapping. It applies to every picture, animation frame and
  Monte Carlo render (after the noise filter); off, the pictures are exactly
  as before. Scene keys `finish_exposure`, `finish_tonemap = filmic`,
  `glow = strength, radius, threshold`, `finish_contrast`,
  `finish_saturation`, `finish_warmth`, `vignette`, `chromatic`. In a big
  render made of separate tile pictures each tile glows on its own.

  </details>
* **The Droste effect**: the picture repeated in itself, as in Escher's
  *Print Gallery*, with Escher's spiral or as concentric copies, and an
  endless zoom when the phase is animated.

  <a href="docs/img/droste.jpg"><img src="docs/img/droste.jpg" width="760" alt="The Dodeca Torus mix example as calculated, and with the Droste effect: the fractal spirals into itself towards the middle"></a>

  <details><summary>Read more</summary>

  Post processing window, section *Finishing*, button *Droste effect...*:
  the window shows the calculated picture at a reduced size with the
  effect, live. Tick *Repeat the picture in itself*, click or drag on the
  preview to set the centre, turn the mouse wheel over it for the outer
  radius (with Shift the inner one), then *Paint now* paints it at full
  size. The ring between the two radii is the only part of the picture
  kept: it is repeated towards the centre and outwards, ever smaller and
  ever larger. A *twist* of 1 joins the copies along Escher's spiral, 0
  gives concentric copies; the *crossfade* blends neighbouring copies
  where the ring's inner edge meets its outer one; the *phase* zooms
  through one step, and the picture at phase 1 is the one at phase 0, so
  an animation of the phase from 0 to 1 is a seamless endless zoom made
  from one calculated picture. It works best with an object in the
  middle of the picture or a view into a hole; a horizon through the ring
  shows the copies' edges. Scene keys `droste = centre x, centre y, inner
  radius, outer radius` (parts of the picture's size, or `off`),
  `droste_twist`, `droste_phase`, `droste_blend`; they are saved with the
  parameters and blended between keyframes. The effect needs the whole
  picture, so the tiles of a big render and the Bulb Tracer's views do not
  get it; a 360 panorama has none. The mathematics is Bart de Smit's and
  Hendrik Lenstra's (2003), the filter's controls follow Josh Sommers's
  and Tom Beddard's.

  </details>
* **A fine relief on the surface**: rough, stone (ridges and cracks) or
  grain, finer than the calculation's detail, with a colour variation.

  <a href="docs/img/relief.jpg"><img src="docs/img/relief.jpg" width="760" alt="A smooth part of a Mandelbulb, and the same with the stone relief and colour variation"></a>

  <details><summary>Read more</summary>

  Post processing window, section *Material*: *Relief* with its strength, the
  size of its features (in the fractal's units, so it stays on the surface
  when the camera moves) and its fineness, and *Colour variation*. It tilts
  the surface where the light is calculated, so shading, highlights,
  reflections and Monte Carlo all see it; the shape itself is not changed
  (the outline stays smooth). Not on the ground plane. Scene keys
  `detail = stone, 1.5, 0.12, 0.5` (kind, strength, size, fineness) and
  `detail_color`.

  </details>
* **Turntables, recorded flights and motion blur** for animations.

  <a href="docs/img/motion_blur.jpg"><img src="docs/img/motion_blur.jpg" width="640" alt="A frame of a fast turntable, without and with motion blur"></a>

  <details><summary>Read more</summary>

  *Animation maker* > *Turntable*: eight keyframes of one whole turn around
  the middle of the main window's view, a smooth loop (`mb3d animate
  scene.m3p --orbit 8 --frames 25`). *Navigator* > *Rec*: while the button is
  down, every new view of the flight (at most two a second) becomes the next
  keyframe; with Catmull-Rom the animation passes through all of them.
  *Motion blur* in the Animation maker: each frame is the average of that
  many pictures within the *Shutter* time (a part of the time to the next
  frame), so what moves smears; it takes that many times as long
  (`mb3d animate --motion-blur 8,0.5`).

  *Animation maker* > *Animation templates...* makes keyframes from the main
  window's view for a duration in seconds (at the window's frames per
  second): a *360° orbit* around an axis of your choice (left to right, over
  the top, a roll, all three at once, or the fractal's X, Y or Z axis;
  `--orbit 8 --duration 10 --orbit-axis pitch`) or a *Fly-through*, in which
  the camera flies on into the fractal, the zoom following the distance to
  it, keeping its course and turning only to get round what it would hit
  (`--fly --duration 20`; `--duration 0`: until the view shows only sky or
  only fractal). *360° VR* in the Animation maker (`--vr`) makes every frame
  a spherical panorama (2:1); with *Render stereo animation* the video has
  the eyes one above the other (left on top), and the ffmpeg command ends
  with `mb3d vr360 video.mp4 --stereo tb`, which marks the MP4 as a 360
  video for YouTube, the Meta Quest and VLC. A spherical panorama saved as
  JPEG carries the Photo Sphere XMP, so Facebook shows it as a 360 photo.

  <a href="docs/img/orbit_template.jpg"><img src="docs/img/orbit_template.jpg" width="640" alt="Five frames of a 360° orbit around a point on a Mandelbulb's surface"></a>

  <a href="docs/img/fly_through.jpg"><img src="docs/img/fly_through.jpg" width="640" alt="A fly-through, one frame a second"></a>

  The animation templates, 360° photos and 360° VR videos were contributed
  by Rathinagiri S ([SRathinaGiri](https://github.com/SRathinaGiri),
  [#30](https://github.com/thr32423/mb3d-rs/pull/30)). The orbit keeps the
  camera at the same distance from its pivot in every frame, also between
  the keyframes: the middle of the view is put on the pivot first, so the
  keyframes differ only in their rotation. Added after other keyframes, it
  ends with its first keyframe again, so the turn is whole; *Start from*
  takes the main window's view or the last keyframe. A fly-through is flown
  in the background with a progress bar, and *Stop* keeps what was flown.
  Videos (attached to the v0.2.100 release, rendered with the version after
  it): [an orbit with 4 keyframes, the first version and the exact one side
  by
  side](https://github.com/thr32423/mb3d-rs/releases/download/v0.2.100/anim-templates-orbit-compare.mp4),
  [a
  tumble](https://github.com/thr32423/mb3d-rs/releases/download/v0.2.100/anim-templates-tumble.mp4),
  [a
  fly-through](https://github.com/thr32423/mb3d-rs/releases/download/v0.2.100/anim-templates-fly-through.mp4),
  [a 360° VR
  orbit](https://github.com/thr32423/mb3d-rs/releases/download/v0.2.100/anim-templates-vr360-orbit.mp4).
  All four with pictures and notes:
  [docs/ANIMATION_TEMPLATES.md](docs/ANIMATION_TEMPLATES.md).

  </details>
* **Gaussian splats** of the fractal, to look at from every side in
  Blender (5.3 and later), SuperSplat and other splat viewers: *Bulb
  Tracer2* > *Gaussian splats...*, or `mb3d splat scene.m3p -o fractal.ply`.

  <a href="docs/img/splats.jpg"><img src="docs/img/splats.jpg" width="760" alt="The program's picture of dIFS heart shrub, and its Gaussian splats in Blender from the left and from behind"></a>

  <details><summary>Read more</summary>

  The window exports the main window's scene; its settings are those of
  the command line. The fractal is calculated from 16 directions
  (*Directions*, `--views`), each a picture of 640 x 640 pixels (*Size of
  each view*, `--size`), with its lights, shadows,
  occlusion and reflections. Every pixel of the object becomes a flat
  Gaussian lying on the surface, with the colour it has in the picture.
  Where the pictures overlap, each patch of the surface comes from the
  one that saw it most squarely. The colours change with the direction you
  look from (spherical harmonics, *Colours by direction*, `--sh`), so
  highlights move as they do in the pictures.

  An object seen from outside is calculated from all around the scene's
  middle. A scene seen from inside (a hall, a cave, where the fractal fills
  the whole picture) is calculated from the scene's camera, looking in
  every direction (*Calculate from*, `--around camera`; *Auto*, the
  default, chooses). There,
  a larger `--size` keeps more detail, as each view is 110 degrees wide.

  The lights that turn with the camera, and the ambient light's top and
  bottom colours, are fixed to the object as they light the scene's own
  view, so all sides are lit alike. Depth of field, clouds, glow, vignette
  and the ground plane are left out. The file is in the scene's view (x to
  the right, y into the picture, z up: Blender's front view looks as the
  scene does), the middle or the camera at 0 and the fractal scaled to a
  radius of 1. It takes about 160 bytes per splat, typically 100 to 300 MB;
  `--sh 0` makes it a third of that, without the moving highlights.

  </details>

## Download

Ready-to-run programs for the latest release, no installation needed:

| System | Download |
|---|---|
| Windows (64 bit) | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux (64 bit) | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian, Ubuntu, Mint (installs like other programs) | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora, RHEL, Rocky, openSUSE (installs like other programs) | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
| macOS, Apple silicon (M1 and later) | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS, Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

Unpack it and start **Mandelbulb3D** (`Mandelbulb3D.exe` on Windows) in
the unpacked folder. The folder is laid out like an MB3D installation,
with MB3D's formulas, maps and 80 example parameter files.

* If Windows shows "Windows protected your PC", click *More info* ▸
  *Run anyway*.
* macOS blocks downloaded programs that are not signed: run
  `xattr -dr com.apple.quarantine .` in the unpacked folder once.

**Alternative for Linux and macOS:** instead of downloading by hand, this
one line fetches the latest release into `~/mb3d-rs` (run it again later
to update; the script [download.sh](download.sh) is also in the program's
folder):

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

It checks the download before unpacking it, puts **Mandelbulb3D** in
your application menu (Linux) and makes `Mandelbulb3D` and `mb3d`
commands you can type in any folder. `sh ~/mb3d-rs/download.sh
--uninstall` takes the menu entry and the commands away again; your files
stay.

The Linux programs run on distributions from about 2018 on (Ubuntu 18.04,
Debian 10, Rocky/RHEL 8 and newer).

**Installed like other programs:** on Debian, Ubuntu and Mint, open the
`.deb` file above (or `sudo apt install ./mb3d-rs-linux-x64.deb`); on
Fedora, RHEL and Rocky the `.rpm` file (`sudo dnf install
./mb3d-rs-linux-x64.rpm`, on openSUSE `sudo zypper install
./mb3d-rs-linux-x64.rpm`).
Mandelbulb3D is then in the application menu and `mb3d` works in any
folder; your settings and History are kept in `~/.config/mb3d-rs` and
`~/.local/share/mb3d-rs`.

[Getting started](GETTING_STARTED.md) walks through the first steps; older
versions and the release notes are on the
[releases](https://github.com/thr32423/mb3d-rs/releases) page.

## Guides and help

Step-by-step guides are in the [wiki](https://github.com/thr32423/mb3d-rs/wiki):

* [Installing](https://github.com/thr32423/mb3d-rs/wiki/Installing): Windows, macOS and Linux, updating, where your files are
* [Lighting workflow](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow): render plainly, then try out lights, shadows and reflections
* [Swarm setup](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup): rendering on several computers through a shared folder
* [Troubleshooting and FAQ](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ): when something does not work as expected
* [Twelve wishes of the Mandelbulb Maniacs group](docs/COMMUNITY_WISHES.md): 360 photos Facebook recognises, maps of any name, the colour picker's hex field and eyedropper, errors in plain words, the Share window, Mandelbulber settings files opening, the Formula book, step settings for the post passes and the noise doctor, background pictures 1:1, texture export, mesh export without flying parts and decimated, and the benchmark

Every release with its changes: [the whole list](docs/UPDATES.md).

## Rendering on several computers (the swarm)

Computers that see one shared folder render together: a picture in
tiles, a Big render, a Monte Carlo picture, an animation or a batch. Each
computer takes the parts nobody else has, so a fast one takes more of
them, and the computer that started the render puts the results
together. The folder does the sharing: the program opens no network
connection and talks to nobody, so it is as safe as your file sharing.

<a href="docs/img/swarm_window.png"><img src="docs/img/swarm_window.png" width="760" alt="Mandelbulb3D with the Swarm window, opened with the Swarm button on the Tools tab, while two computers render a picture in tiles: the shared folder, Share this computer, the computers and what each is doing, the job, and the preview with rendered tiles, a tile under way and missing ones"></a>

### Setting it up

Step by step with what to check: [Swarm setup](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup) in the wiki.

1. **A folder every computer can write to:** a share on a NAS, a Windows
   share, a Samba share or NFS. On Linux, mount it as a folder, for
   example
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`;
   a share only opened in the file manager (`smb://...`) is not a folder
   the program can use.
2. **The same version of mb3d-rs on every computer.** A computer with
   another version is listed but takes no part.
3. **In Mandelbulb3D:** the **Swarm** button on the Tools tab. Choose the
   shared folder (the same share on every computer; its path may differ),
   give the computer a name for the list, and tick **Share this
   computer**.

That is all. The status line says what happens, for example *Sharing
with omarchy: renders are split between the 2 computers*. Alone in the
swarm, everything renders on this computer as usual.

### What is shared

While *Share this computer* is ticked, the computer renders the parts
other computers put into the folder, and its own renders go to the swarm
whenever another computer is in it:

* **Calculate 3D** (and the Batch window) — the picture in N x M tiles
  (set in the Swarm window), put together here with everything the usual
  calculation gives: lighting changes, post processing, saving as m3i (and
  its memory, see below).
* **Big render** — the tiles are saved one by one as they arrive; the
  whole picture is never held in memory, so this is the way for pictures
  of 100000 x 100000 pixels and more.
* **Monte Carlo** — as on one computer: every computer adds passes to its
  own block until you press Stop, the picture cleans up with everyone's
  rays, and Stop averages the blocks. In the window's batch mode the
  blocks together stop at the batch's ray count.
* **The Animation maker** — the frames, one by one.

The window lists the computers with their cores, graphics card and what
each is doing, the jobs with their progress, and a preview of the newest
job: the tiles rendered, the ones under way with a progress bar, and the
ones still missing.

### On the command line

On every computer that should help, start a worker on the folder:

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

Then, on any computer, render with `--swarm` and the same folder:

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER` shows who is in the swarm, what each computer is
doing and how far the jobs are. A Monte Carlo picture on the command line
is split into blocks with their own random rays: 4 blocks of 64 rays per
pixel give the 256 asked for with `--rays 256`.

### Good to know

* **If the other computers stay idle** and their job list is empty while
  this computer renders: the computer that serves the folder with Samba
  and uses it directly lets the others keep an old listing of it. Turn
  that caching off in `/etc/samba/smb.conf`: `smb3 directory leases = no`
  in `[global]`, and `oplocks = no` and `level2 oplocks = no` in the
  share's section, then `sudo systemctl restart smb` and mount the share
  again on the other computers.
* **Memory:** a picture takes about 20 bytes per pixel in this program
  (its image buffer), 10 more in the main window, and then the larger of
  two stages: the main calculation with its shadows and occlusion (16 per
  pixel and 1.2 GB for the card), or the painting (3 per pixel, 20 more
  with depth of field, and one band of the reflections: 3.5 GB at most,
  less when little memory is free, since the band is sized to half of what
  is free when the pass starts). A 12000 x 6000 picture: 6 GB. When a
  picture needs more than the free memory of the computer (asked of Linux,
  Windows and macOS), Calculate 3D says how much it needs and how much is
  free, and asks before it starts: the calculation would swap and the
  program could stop responding for minutes at a time. Big render, alone
  or with the swarm, is the way out: its tiles are saved one by one, the
  whole picture is never held. Calculate 3D with the swarm holds the
  picture here as the usual calculation does (without the main
  calculation's stage, which the tiles' computers do with each tile's
  memory); a computer of the swarm leaves a tile too big for its memory to
  the others.
* **Safety:** a computer renders only what a job asks for and writes only
  into the job's folder; formula code from a job runs in the formula
  emulator like every formula.
* **Speed:** tiles travel as full image buffers, a few hundred megabytes
  for a very big picture, so a fast share pays off. A sync service that
  copies with a delay works too, but slower.
* **Computers that go away:** the part a computer was rendering is taken
  by another once it has not been heard of for half a minute. A job whose
  owner has gone for half an hour is removed by whoever sees it. A job
  that fails on a computer is noted in the window and left to the
  others.
* **Stopping:** unticking *Share this computer* lets the part under way
  finish first; closing the program gives it up, and another computer
  takes it.
* **Stopping the job** (Stop on the computer that made it, or closing
  it, or it crashes): every part already finished is kept on that
  computer, and each computer keeps the part it was rendering. Starting
  the same render again makes a job of only the missing parts, and a
  computer that gets back a part it had begun continues it. The same
  works on the command line: run the same command again. An open-ended
  Monte Carlo render (the window's) puts its blocks together when it is
  stopped, as before.

## How the rendering works

A fractal picture is calculated pixel by pixel: for every pixel a ray is
followed into the scene until it hits the fractal's surface, the formula
being evaluated dozens of times at every step. MB3D does this on the
processor, using all its cores. mb3d-rs can do the same, and it can also
hand the work to the **graphics card**, which has thousands of small
processors and is built for exactly this kind of job.

The **"Calculate on"** setting (Internal tab of the main window) chooses:

| | |
|---|---|
| **Auto** (default) | The card and the processor share the picture: the card takes rows from the top, the processor's threads from the bottom, until they meet. The shadows and the ambient occlusion are shared the same way, or done by the card alone when it is clearly the faster one. |
| **CPU** | The processor only, as in MB3D. |
| **Graphics card** | The card only (it waits until its program is ready). |

The first time a combination of formulas is used, the graphics driver
has to prepare a program for the card — the *shader* — which takes 10 to
15 seconds on the computer measured below. In Auto mode the processor
simply starts without it and the card joins when it is ready, so a small
picture never waits. The prepared shader is kept for the next time (by
the graphics driver, on Linux in the `ShaderCache` folder next to the
program), so the next picture with the same formulas starts on the card
at once.

The **LED** right of the window title tells where the last picture was
calculated: green on the graphics card, orange partly, red on the
processor. Pointing at it shows the details.

[How the rendering works](docs/RENDERING.md) tells the whole story with
pictures: what Auto does the first time, why the card is not always the
faster choice, and what the colours of the LED mean.

Not everything can go to the card: a few formulas cannot be translated for
it, and views zoomed in very deeply are calculated in double precision
(which only some cards can do). The program decides all of this by itself; you only see it in the
LED.

## Speed

All measurements on one computer: an AMD Ryzen 9 3900X (12 cores, 24
threads) with an NVIDIA GeForce RTX 2060, on Linux. The original
**Mandelbulb3D 1.99sr37** was run on the same machine under Wine (its
Windows program runs there unchanged, at full speed, using all cores),
and timed from the click on *Calculate 3D* to the finished picture.
Each parameter file was rendered at the same image size by all three:
MB3D, mb3d-rs on the processor only (**CPU**) and mb3d-rs in **Auto**
mode (graphics card and processor together). The Auto times are those of
the second render of a file, with the card's shader already prepared; the
first render of a new formula combination adds the 10–15 s of shader
preparation on this card.

### MB3D's 80 example parameter files

The example files that come with MB3D (and in this program's
`M3Parameter` folder), each at its own image size.

MB3D could not calculate one of the files (*Test8*: it stops with a "Division by zero" error), so the comparisons count 79 files.

* **mb3d-rs on the processor alone is faster than MB3D** in 71 of the 79 files, a median **1.7 times**; all files together took MB3D 18 min 21 s and mb3d-rs on the processor 17 min 10 s.
* **Auto mode** is faster than MB3D in 77 of 79 files (median 2.1 times); all files together 8 min 17 s. In 30 of the files the processor finished alone, mostly because it needs only a few seconds, before the card gets a turn — there Auto is simply the processor plus a moment's overhead. The card joined in 49 files, the slower ones, where it made the picture a median 1.6 times faster than the processor (up to 13 times: *Insect Menger*, 4.7 s instead of 63.0 s).
* In 4 files Auto was noticeably slower than the processor alone (*Ellarien - Shrooms are coming*, *MengerTrees*, *toricaleggs*, *TreePlanet*): there the card is slower than the 24 processor threads for the scene.
* "Rows on the card" is the share of the picture the graphics card marched in Auto mode (– when the processor finished alone).

<details>
<summary>All 80 files (click to open)</summary>

| Parameter file | MB3D (Wine) | mb3d-rs CPU | mb3d-rs Auto | rows on the card | Auto vs MB3D |
|:--|--:|--:|--:|--:|--:|
| 0Encrypted0 - ABoxSOff4d example | 12.8 s | 7.4 s | 1.9 s | 80 % | 6.7× |
| 6 AM - Torii temple | 23.0 s | 20.4 s | 17.8 s | 60 % | 1.3× |
| Abominog-Ua - 12.06.2014 | 7.6 s | 8.0 s | 7.4 s | 50 % | 1.0× |
| ABoxPartlyJuliaRoff | 2.5 s | 1.4 s | 1.6 s | – | 1.6× |
| ABoxScale2Start | 1.0 s | 0.1 s | 0.3 s | – | 3.3× |
| ABoxScale3Start | 1.5 s | 0.1 s | 0.3 s | – | 5.0× |
| AboxShapes-Misty | 27.5 s | 25.6 s | 17.7 s | – | 1.6× |
| Aexion 10bulbs | 11.5 s | 9.8 s | 2.6 s | 73 % | 4.4× |
| AkuraPare - 463b | 4.0 s | 2.0 s | 0.7 s | 72 % | 5.7× |
| ApolloBalloons dIFS | 75.8 s | 73.2 s | 24.9 s | 57 % | 3.0× |
| AureliusCat - Bamboo | 2.0 s | 1.2 s | 0.7 s | 58 % | 2.9× |
| BatJorge - Pong 703 - Swirls | 4.0 s | 2.8 s | 3.1 s | – | 1.3× |
| Bill-Snowzell - Tweak - Canyon | 4.0 s | 1.6 s | 1.7 s | 44 % | 2.4× |
| Blood on the dancefloor | 7.6 s | 4.4 s | 3.2 s | – | 2.4× |
| BulboxCut | 4.0 s | 2.3 s | 2.3 s | – | 1.7× |
| Chrystal | 5.6 s | 5.2 s | 1.7 s | 79 % | 3.3× |
| cutted sphere over carpet | 7.1 s | 20.7 s | 4.5 s | 48 % | 1.6× |
| Dainbramage - Home of the Ancients | 3.0 s | 1.3 s | 1.2 s | 25 % | 2.5× |
| Dainbramage - Hydra | 3.5 s | 1.7 s | 1.6 s | 41 % | 2.2× |
| Dainbramage - Nothing left but bones | 4.0 s | 3.0 s | 2.4 s | 49 % | 1.7× |
| DEcomb1 | 83.9 s | 93.5 s | 80.3 s | 48 % | 1.0× |
| dIFS heart shrub | 2.5 s | 3.5 s | 3.0 s | – | 0.8× |
| dIFS heart shrub otrap | 5.1 s | 15.6 s | 3.6 s | 93 % | 1.4× |
| dIFS shrub | 4.0 s | 17.3 s | 3.6 s | 93 % | 1.1× |
| Dodeca Torus mix | 2.0 s | 0.5 s | 0.6 s | 14 % | 3.3× |
| DsyneGrafix - Getting Loopy | 2.0 s | 0.8 s | 0.8 s | 25 % | 2.5× |
| Ellarien - Embellished | 3.5 s | 1.1 s | 1.2 s | 49 % | 2.9× |
| Ellarien - Shrooms are coming | 8.1 s | 6.2 s | 6.8 s | – | 1.2× |
| ExcludeBulbMeng | 17.3 s | 12.9 s | 8.5 s | – | 2.0× |
| Fishbowl | 4.6 s | 3.5 s | 1.9 s | 49 % | 2.4× |
| Fractales75 - Kiosk | 3.0 s | 1.8 s | 1.9 s | 47 % | 1.6× |
| Genetic Menger | 1.5 s | 0.5 s | 0.6 s | 46 % | 2.5× |
| genQuat helix mix | 4.0 s | 3.3 s | 1.8 s | 61 % | 2.2× |
| HalTenny - Freak Of Nature | 2.5 s | 0.9 s | 1.2 s | – | 2.1× |
| Hal-Tenny-Resistance (slow) | 18.9 s | 20.3 s | 8.7 s | 78 % | 2.2× |
| Hyperben2 - Ozosphere tweak | 2.0 s | 0.8 s | 0.6 s | 56 % | 3.3× |
| ImpossibleWorld dIFS | 399.8 s | 367.5 s | 99.0 s | 71 % | 4.0× |
| InAndOutside box | 5.1 s | 2.7 s | 1.4 s | 68 % | 3.6× |
| Insect Menger | 4.0 s | 63.0 s | 4.7 s | 93 % | 0.9× |
| Jost1 - Tweak - New tiny temple | 3.5 s | 1.6 s | 1.0 s | 60 % | 3.5× |
| julia on box | 4.6 s | 4.0 s | 1.6 s | 73 % | 2.9× |
| JuliaSetSample | 3.0 s | 1.9 s | 2.2 s | – | 1.4× |
| KentPress - Parameter 3 tweak fogits | 4.0 s | 2.5 s | 3.0 s | – | 1.3× |
| KochSurf Sample 6 | 3.0 s | 1.4 s | 0.8 s | 75 % | 3.8× |
| Lenord - ASurf 2 | 5.1 s | 1.7 s | 1.9 s | 48 % | 2.7× |
| Lenord - Between Dark and Light | 5.6 s | 3.8 s | 2.1 s | 60 % | 2.7× |
| Lenord - The Beanstalk | 3.5 s | 1.8 s | 2.0 s | – | 1.8× |
| LightBulbMoon - A Fabulous Excuse | 3.5 s | 2.0 s | 1.6 s | – | 2.2× |
| LightBulbMoon - No, Never | 5.1 s | 4.0 s | 1.2 s | 82 % | 4.2× |
| Mandelwerk - Makin seashell | 12.0 s | 8.1 s | 8.1 s | 30 % | 1.5× |
| Mandelwerk - TgladTetra inner spirals | 8.6 s | 4.5 s | 1.1 s | 80 % | 7.8× |
| MarkJayBee - Curvichrome | 2.5 s | 1.3 s | 1.3 s | – | 1.9× |
| material colors | 3.0 s | 1.4 s | 1.9 s | – | 1.6× |
| Melting spot bloxx | 23.2 s | 20.4 s | 7.6 s | 73 % | 3.1× |
| Mengerplus for MC | 3.5 s | 2.2 s | 2.5 s | – | 1.4× |
| MengerTrees | 19.3 s | 15.4 s | 16.9 s | – | 1.1× |
| MutaGenD4B12E71C70D4FD00357_canary~ | 7.1 s | 3.7 s | 4.1 s | – | 1.7× |
| Oxnot - Flexing | 2.5 s | 0.7 s | 0.7 s | 59 % | 3.6× |
| Oxnot - Shells | 5.6 s | 3.3 s | 3.6 s | – | 1.6× |
| PaMonk - Ringlets | 5.1 s | 2.7 s | 2.8 s | – | 1.8× |
| QuatP4hybridJulia | 1.5 s | 0.2 s | 0.4 s | – | 3.8× |
| Rama - Elysium | 11.5 s | 10.6 s | 5.5 s | 74 % | 2.1× |
| Recycledrelatives - Fractal Fan | 2.0 s | 0.4 s | 0.6 s | 31 % | 3.3× |
| Recycledrelatives - Test for SurfMod | 4.6 s | 3.0 s | 3.1 s | – | 1.5× |
| Rowratty - Spirals | 4.0 s | 2.5 s | 1.0 s | 69 % | 4.0× |
| Sabine62 - Housing problem | 11.7 s | 10.5 s | 6.8 s | – | 1.7× |
| spineJulia | 91.9 s | 71.7 s | 50.7 s | 35 % | 1.8× |
| Surreal shell | 2.0 s | 0.6 s | 0.8 s | – | 2.5× |
| Test8 | error in MB3D | 0.1 s | 0.3 s |  | – |
| Theli-At - Black Mandelbulb | 1.0 s | 0.2 s | 0.5 s | 22 % | 2.0× |
| Theli-At - Bone Menger using MH | 2.0 s | 0.7 s | 1.1 s | – | 1.8× |
| Theli-At - MengerSpheres | 2.0 s | 0.6 s | 0.6 s | 50 % | 3.3× |
| ThePearl dIFS | 2.5 s | 1.4 s | 1.2 s | 25 % | 2.1× |
| TimeMachine | 1.0 s | 0.5 s | 0.7 s | – | 1.4× |
| toricaleggs | 8.6 s | 5.6 s | 6.6 s | 40 % | 1.3× |
| TreePlanet | 7.6 s | 6.6 s | 7.5 s | – | 1.0× |
| UndeadAcademy tweak - Peacock in blue | 7.6 s | 4.5 s | 1.2 s | 74 % | 6.3× |
| Virtual tubes world (UndeadAcademy style) | 9.1 s | 6.9 s | 4.3 s | 49 % | 2.1× |
| Wada basin - Reflect test | 6.6 s | 4.1 s | 4.6 s | 40 % | 1.4× |
| Waste-and-tragedy - Mandalex | 5.1 s | 3.4 s | 3.8 s | – | 1.3× |

</details>

The pictures are the same: mb3d-rs on the processor makes MB3D's picture
to within rounding, and the Auto pictures differ from the processor's by a
fraction of a percent of the colour range on average (fine detail may
sparkle in different pixels; the [technical notes](docs/TECHNICAL.md)
have the measurements).

### Experimental speed-ups

A few faster ways of calculating change the picture a little, so they are
off unless you switch them on: **Internal** tab → **Experimental...**, tick
*Use the experimental speed-ups*, then the ones you want. They are saved
with the scene (so the command line, the swarm and the AI assistant use
them too); on the command line `-s experimental=on` with `-s step_relax=1.3`,
`-s secant_search=on`, `-s free_steps=on`, `-s occlusion_half=on` or `-s cone_march=on`.
The program keeps your choice from one start to the next, also for the files you open.

Measured on the 80 example files in Auto mode (about 0.4 megapixels,
the faster of two runs each), against the same files without them:

| Speed-up | Whole render | Pictures (pixels off by more than 8 of 255) |
|---|---|---|
| **Step relax 1.3**: longer march steps, an overstep taken again | −5 % (median), up to −20 %; 62 of 80 faster | median 2.6 %; fine shading. *Thin shells can show holes* (BatJorge – Pong 703 – Swirls) |
| **Secant search** for the surface | −2 % (median), up to −18 % | median 0.7 %; fine detail moves slightly |
| **No step limit without dynamic fog** | −1 % (median), up to −30 % in open scenes | mostly the same picture (median 0.03 %) |
| **DE occlusion at half resolution** | −18 % over the 10 files with DE occlusion (DEcomb1 −24 %); that stage up to 4 times faster | 0–27 % of pixels, softer occlusion in fine detail; it looks alike |
| **Cone march**: a coarse ray per block of pixels, every pixel's march starts where it stopped | the march −20 % (median −22 %) on the processor, up to −66 % in open scenes (68 of 69 faster); on the card whole renders of open scenes up to −47 % | median 1.0 %, as much as the march's own random first step changes; *0.2.92 could lose thin parts where the distance estimate is far too large; fixed in 0.2.93* |

Compare with a render without them before relying on one for a picture.
A page with pictures, the gains of all five together and how they affect
each other: [docs/EXPERIMENTAL.md](docs/EXPERIMENTAL.md).

## The command line

`mb3d` renders without a window. Open a terminal in the program's folder
(on Windows a command prompt in the unpacked folder, and type `mb3d`
instead of `./mb3d`). After the one-line install on Linux or macOS, `mb3d`
works in any folder, `man mb3d` shows its manual, and the Tab key
completes its options in bash, zsh and fish:

```sh
./mb3d "M3Parameter/6 AM - Torii temple.m3p" -o torii.png      # full size
./mb3d "M3Parameter/6 AM - Torii temple.m3p" --scale 0.25 -o small.png
./mb3d pasted.txt -o pasted.png        # parameters copied from a forum post
./mb3d City.m3p --tiles 4x4 -o city.png    # a big picture in tiles
./mb3d resume                              # stopped renders: the same command continues
./mb3d --cpu City.m3p -o city.png          # processor only
./mb3d --card City.m3p -o city.png         # graphics card only
./mb3d City.m3p --bits 16 -o city16.png    # 16 bit colour
./mb3d City.m3p --denoise 4 -o city.png    # with the noise filter
./mb3d City.m3p --aa 2 --transparent -o city.png   # transparent background
./mb3d City.m3p -o - | display -          # the picture into another program
./mb3d --help                              # all options
```

Everything else has a sub-command with its own `--help`:

```sh
./mb3d batch M3Parameter --scale 0.2 -o previews        # a whole folder
./mb3d animate start.m3p middle.m3p end.m3p --frames 60 -o frames
./mb3d animate project.m3a -o frames                     # MB3D animations
./mb3d animate project.m3a -o frames --transparent      # ... see-through
./mb3d animate project.m3a -o frames --video flight.avi  # ... and a video
./mb3d montecarlo scene.m3p --rays 64 -o mc.png
./mb3d montecarlo scene.m3p --rays 8 --denoise -o mc.png # ... with the noise filter
./mb3d voxel scene.m3p -o voxels --slices 256            # for 3D printing
./mb3d mesh scene.m3p -o object.stl                      # Bulb Tracer 2
./mb3d splat scene.m3p -o fractal.ply                    # Gaussian splats
./mb3d mutagen scene.m3p -o muta                         # 15 variations
```

The formulas and maps are found next to the program; those of another
MB3D installation can be given with `--formulas` and `--maps`.
Animations are MB3D's: keyframes with MB3D's interpolation, `.m3a`
projects opened and saved, frames written like MB3D so that an
interrupted animation continues and several computers can share one.

## Good to know

* **Same pictures, not bit-identical.** MB3D calculated in the 80-bit
  precision of old x87 arithmetic; this program uses today's 64-bit. The
  pictures match in every structure, but single pixels can differ in the
  last digit of their colour. Monte Carlo renders and MutaGen use their
  own random numbers, so their results are not MB3D's.
* **The graphics card's pictures** differ slightly from the processor's
  (single precision); in very fine detail the sparkle can fall on other
  pixels. Deep zooms switch to double precision by themselves.
* **Speed on the processor** is close to MB3D's: the formulas' machine
  code from MB3D's `.m3f` files was translated so that it runs on any
  processor (also Apple Silicon), at a small cost; the ray marching
  itself is faster. See the tables above. On Linux and Windows the
  formulas of a scene's hybrid are also compiled together into one
  function when a picture starts (in a few milliseconds, with Cranelift),
  so that they hand their values on in the processor's registers.
  Together with a faster loop over the formulas (on every system), CPU
  renders of the 80 example files take 20 % less time than in v0.2.74
  (up to 60 % in some), the pictures the same to the bit. Both parts of a
  DE combination are compiled, each as its own function (before only the
  first: two objects side by side render up to 27 % faster since v0.3.3).
  A scene keeps a compiled hybrid only if it
  measures faster; `MB3D_NO_JIT=1` switches it off. On macOS it is on
  too, but still being tested there: `./jit_check.sh` in the unpacked
  folder checks it (it compares
  the compiled formulas with the translated ones and renders the example
  files with and without it; the pictures must be the same).
* **Big pictures** can be rendered in one piece (a 15000 x 20000 picture
  needs about 15 GB of memory; with depth of field it takes hours) or in
  tiles, also on several computers (`--tiles`, and the Big renders
  window), which needs less memory. Calculate 3D does not start a
  picture that does not fit into the free memory (on Linux, where the
  free memory can be told) and says so, and a computer of the swarm
  leaves a tile too big for its memory to the others.
* **Unfinished renders** are kept in the cache folder (`~/.cache/mb3d-rs/resume`,
  on Windows `%LOCALAPPDATA%\mb3d-rs\resume`, or next to the program):
  18 bytes per pixel for a picture (a 15360 x 15360 picture takes 4.2 GB),
  64 for a Monte Carlo render. What was calculated is written at once and
  flushed to the disk every 5 minutes. A render stopped within its first
  10 seconds keeps nothing; unfinished renders older than 30 days go, and the
  oldest ones when all of them take more than 50 GB. A render continues
  only with the same parameters and program version (and the same choice
  of card or processor; a Monte Carlo render's exposure and saturation
  may change). A Monte Carlo render stays unfinished until it reaches its
  rays (a batch, `--rays`, `--passes`) or is deleted in the Resume list.
* **Newer or your own formulas, maps, themes and languages** go into
  `~/.local/share/mb3d-rs/M3Formulas`, `.../M3Maps`, `.../Themes` and `.../Languages`
  (any way of installing; on Windows `%APPDATA%\mb3d-rs\...`). They
  are used in addition to the ones that
  come with the program, and replace a shipped file of the same name, so
  an updated formula from the forums takes effect without touching the
  program's folder.

## Updates

The newest releases with the time they were published and the changes
(commits) they brought; every release is in [the whole list](docs/UPDATES.md).
Each commit links to its full description and code on GitHub; the downloads
of a release are on its page. Times are in UTC.

<!-- updates:start -->
### [v0.3.5](https://github.com/thr32423/mb3d-rs/releases/tag/v0.3.5) · 8 Nov 2026, 16:24 UTC

| Commit | What it does |
|---|---|
| [`b762730`](https://github.com/thr32423/mb3d-rs/commit/b7627303740a4624b1f7da4715bc100086369d11) | Tamil: the 151 labels of today's windows translated (the Formula book, the Share window, the colour picker, the error explanations, Steps and noise, the Benchmark, Texture export, the Bulb Tracer fields); the twelve labels of the parked workspace layout taken out, so every label of the file is one the program has |
| [`cdd523f`](https://github.com/thr32423/mb3d-rs/commit/cdd523fb2a438fef64b8c94f6dc108cc4ff6f12f) | The Formula book module and the benchmark's card name are gated behind the gui and gpu features: the program builds without them again (the command line alone) |
| [`b933d6a`](https://github.com/thr32423/mb3d-rs/commit/b933d6a795a9bbbcaa18aecd8e80d76c6a692328) | Version number and release notes of this release |
| [`fb0c940`](https://github.com/thr32423/mb3d-rs/commit/fb0c9402729c1bdb947816afe852dbc925299976) | Languages: Tamil (தமிழ்), all 1,992 labels, with the flag; Tamil.ttf a subset of Noto Sans Tamil UI with its shaping rules (OFL-NotoSansTamil.txt), its joined letters and vowel signs shaped with rustybuzz like Devanagari (the Tamil block added to the shaped scripts). README and TECHNICAL.md name the fonts of every shaped script |
| [`0c2d08b`](https://github.com/thr32423/mb3d-rs/commit/0c2d08bc2f47842b628f2baaab7aa5aa8807989c) | Share window: says whether the picture got the 360 tags (RICOH THETA camera, Photo Sphere XMP) or why not (the calculated picture must be a 2:1 spherical panorama); a shrunk panorama a pixel off 2:1 still counts. Formula book: the example clicked in the list is the one shown, not always the first. |
| [`ffc338c`](https://github.com/thr32423/mb3d-rs/commit/ffc338cceca71221dfa3fa6924d9d044164320ab) | Five more wishes of the Mandelbulb Maniacs group (docs sections 8-12). Step settings for the passes after the main one (Scene::post_steps, post_raystep / post_de_stop in the mb3d-rs block): the shadows, the DE occlusion, the volumetric light and the reflections march with their own ray step and DE stop while the main pass keeps a fine one, the group's trick without the hand work; a test renders with coarser post steps and gets the main pass's surface. The noise doctor (noise.rs, the Steps and noise window of the Internal tab) renders the middle of the picture with the main pass alone under three step settings, counts the specks (a pixel far outside the range of all its neighbours) and holes, and applies the recommended one. A direct background picture of the render's size is shown pixel for pixel instead of through the spline at a half-pixel offset that blurred it. Texture export (texture.rs, window): colour, 16-bit height (checked against the command line's depth map), normal map in the OpenGL convention, 16-bit ambient occlusion, mirrored tileable sets (normals mirrored too), the camera preset of the group's recipe (planar, 1 degree). Mesh export: islands (loose pieces below a share of the largest piece's triangles) removed and vertex-clustering decimation to a share of the vertices, in the Bulb Tracer window (Islands %, Keep %) and mb3d mesh --islands --decimate, with tools/blender_import_mb3d.py importing the result with a vertex colour material. The benchmark (bench.rs, window, mb3d benchmark): the Integer Power preset at 640 x 512 with 120 iterations on the processor and the card alone, the result as text with the version, times, processor, threads, card and system. The results of the doctor and the benchmark are taken in the idle work as well as by the windows' timers (the timers do not tick headless). 34 labels in all 16 languages (font subsets regrown), screenshots by examples/communityshots.rs, README, TECHNICAL.md. |
| [`5f13642`](https://github.com/thr32423/mb3d-rs/commit/5f136420cdd988e160b948e8d244d683921455ec) | Mandelbulber settings files open, and the Formula book (wishes 6 and 7 of the Mandelbulb Maniacs group). A .fract file opens like any parameter file (Open, dropped, command line, MCP, or pasted text starting with "# Mandelbulber settings file"): the camera is rebuilt from the eye, target and top vector (the rows of vgrads, the pivot, the start plane a hundredth of the eye distance before the eye, the zoom from the pixel angle on it, the vertical field of view from the horizontal one), picture size, iterations, Julia constant, detail level, DE factor and background colours carry over, and the formulas both programs have (a good part of Mandelbulber's list was ported from MB3D: Mandelbox, the bulbs, Menger, Amazing Surf, the ABox mods, Aexion, BristorBrot, the Msltoe mods, Kalisets1, IQ-bulb, Sierpinski ...) map one to one with their values, similar ones at their defaults; hybrids become the slots, boolean objects a DE combination; lights, materials, colours and formulas without a counterpart are left out and listed in the notes. Tests check the camera (eye, direction, up, field of view through splat::eye and start_plane), the formulas, the hybrid slots, the Julia constant and the notes against two fixture files. The Formula book window ("?" in the Formulas window, "About this formula..." in a list's menu): every formula with a picture (the first example file, else the formula alone, a transformation with a bulb behind it), its kind in words from the DE option, the author's description, its values explained by their names (Scale, Fold, Min R, Power ...), hints, and the example files that use it, with search and a kind filter; the preview queue now reads every kind of parameters through animfile::load_scene. 58 labels in all 16 languages (CJK, Arabic and Urdu font subsets regrown), docs/COMMUNITY_WISHES.md sections 6 and 7 with screenshots (examples/communityshots.rs steps import and book), TECHNICAL.md, README. |
| [`9bae123`](https://github.com/thr32423/mb3d-rs/commit/9bae123af6f3410f162d72eaebde556f01d309c0) | docs/COMMUNITY_WISHES.md: the five wishes of the Mandelbulb Maniacs group with screenshots from the program (360 photos Facebook recognises, maps of any name, the colour picker's hex field, recent colours and eyedropper and the borrowed palette, errors in plain words with the DE combination fix, the Share window, and the two toolkit fixes), linked from the README's guides; examples/communityshots.rs makes the screenshots without windows. The 57 new labels translated in all 16 languages (Arabic, both Chinese, Deutsch, Espanol, Francais, Hindi, Indonesia, Italiano, Japanese, Kaszebsczi, Korean, Nederlands, Polski, Portugues, Urdu; the Formula window's tab names quoted as each language shows them), the Japanese, Korean and Chinese font subsets grown by their new characters (tools/font_subsets.py); the Share and colour windows checked in German and Japanese |
| [`9253814`](https://github.com/thr32423/mb3d-rs/commit/9253814d4057c093f9e5715f2ad1b30d57750f95) | Share window: a share icon in the main window's title bar (left of the log icon) opens a window that makes what a post in a group needs, in a folder of its own (Pictures/Share by default): the shown picture as a JPEG of the size Facebook keeps sharp (long side 2048, a 360 photo up to 6000 wide, with its 360 tags) or at full size, the parameters as a text file with the title, artist, credit, formulas and program above them (Facebook turns parameters pasted into a post into links and has flagged them as spam, so the Mandelbulb Maniacs group asks for a .txt file), and the post's text, editable in the window and put on the clipboard. The artist and the credit come from MB3D's Orig and Mod authors (a tweak credits the original artist, as the groups ask), the formulas from the scene (a DE combination as both parts with its kind); a folder button and Open the folder; the 360 camera tag is switched there (Pano360CameraTag). The shared text file opens again (header lines before Mandelbulb3Dv18{). Two toolkit fixes found on the way: the light box behind a title bar icon under the mouse (History, Log, now Share) was painted opaque white and hid the icon (round_rect paints opaque; Canvas::round_fill_alpha keeps the alpha), and a hint wider than a narrow window (the Lighting and Formula windows) was cut off: hints wrap at the window's width there. Tests: formula summaries, file names, the shrink keeps the mean and 2:1, the share folder with its three files and a second folder of the same name |
| [`37656c9`](https://github.com/thr32423/mb3d-rs/commit/37656c97fe900a956ef66a20638643290dc1636a) | Errors in plain words: the messages new users of MB3D meet most (asked about again and again in the Mandelbulb Maniacs group) get a window that says what they mean and what to do (src/app/explain.rs). Parameters that name a formula not installed (opened, dropped, from the History or pasted, which said 'no text parameters found' before): which formula, that formulas are .m3f files shared with parameters or in packs, the formula folders to put it in, and installed formulas of a similar name. A calculation that cannot start: dIFS formulas mixed with others in one chain (the two kinds of formulas explained, and the DE combination that shows both; when the slots hold one kind first and the other after it, a button sets the combination up with min and calculates again; when they take turns, how to reorder them), a part of a DE combination that mixes them, dIFS transforms without a dIFS shape, dIFS in an interpolation hybrid, and no slot with a fractal formula ('formula option is not valid', 'no formula with iterations'). Message boxes can have their own button captions (Ui::message_box_with). Tests: the formula name of the error, similar names, the fix for both orders calculates (CalcParams), no fix when the kinds take turns, the empty-chain errors |
| [`5c817a7`](https://github.com/thr32423/mb3d-rs/commit/5c817a77e89b48786d6e7e51a9bfa40937e59c91) | A better colour picker. The colour dialog (light colours, palette, ground, clouds, ...) has the colour as hexadecimal #RRGGBB to type or copy (also RRGGBB, 0x.. and the short #RGB), the eight colours chosen last (kept in the ini file as RecentColors), and an eyedropper: after its button, a click on any pixel of the program's windows (the picture, a map preview, the palette, another window, also those the dialog blocks) takes that colour, with a cross-hair cursor, Esc stops; the release of that click reaches no control. It reads the window as it is painted (Ui::form_pixel), so it works at any interface scale and without a display. The colour adjustment window takes the colours (surface palette with its transparency, inside colours, no-interpolation) from a parameter file (.m3p, .m3i, text, .m3s), a light file (.m3l) or parameters on the clipboard, leaving lights, fog, maps and the fractal as they are, and saves them as a light file for MB3D too. Asked for in the Mandelbulb Maniacs group (hex codes, borrowing another artist's palette); Mandelbulb3D-SE has an eyedropper too. Tests: hex parsing, recent colours, the hex field and sliders, the eyedropper on a painted swatch at scale 1 and 2, colours from .m3l and .m3p; examples/communityshots.rs starts the documentation screenshots |
| [`3b189de`](https://github.com/thr32423/mb3d-rs/commit/3b189de9cfe1cfbf732f86a781ee29b5d05217af) | Maps of any name: a picture chosen for a map (a right click on the map number of the Lighting window's light map and colour map, and now also on the map number of a height map formula in the Formula window) no longer has to be renamed to a number by hand. Parameters refer to maps by number as in MB3D, so a picture whose name gives no number (or whose number finds another file) is copied into the maps folder as '<number> <its name>' with the next free number (its own number when that is free); the same picture chosen again keeps its number, nothing is overwritten, and the main window's messages say where the copy went. A background picture from outside the map folders is copied there under its name too, so the scene finds it the next time (it was found for the session only). Before, a picture without a number did nothing at all, and the first digits anywhere in a name were taken (brick2.png became map 2, another picture). Tests: numbers from names, copies, reuse, a taken number, backgrounds; map options of the formulas |
| [`9dc9b4b`](https://github.com/thr32423/mb3d-rs/commit/9dc9b4bd95a85232565b0065e0b22352aad0bbb3) | 360 pictures carry the camera of a 360 camera for Facebook: a spherical panorama (2:1) saved as JPEG gets, next to its Photo Sphere XMP, an EXIF block with Make RICOH, Model RICOH THETA S and Software mb3d-rs (the tag Facebook's upload goes by when it ignores the XMP; the Mandelbulb Maniacs group sets it by hand in the file's properties), and a PNG gets both as eXIf and iTXt XML:com.adobe.xmp chunks before the image data. Every way a picture is saved writes them: the main window's PNG, JPEG and 16 bit PNG, the Monte Carlo window, the batch and the command line (mb3d, montecarlo, tiles and swarm, vr360 on an existing JPEG). The camera tag can be switched off (Pano360CameraTag=0 in the ini file, MB3D_360_CAMERA_TAG=0 for the command line). Tests: the segment order of the JPEG, the TIFF structure of the EXIF block, the PNG chunks before IDAT; exiv2 reads Make, Model, Software and GPano from both files |
| [`0c829d5`](https://github.com/thr32423/mb3d-rs/commit/0c829d5ec23171e9920814c0264e405bf6a6faa5) | README: Updates for v0.3.4 |

### [v0.3.4](https://github.com/thr32423/mb3d-rs/releases/tag/v0.3.4) · 8 Nov 2026, 06:22 UTC

| Commit | What it does |
|---|---|
| [`d5eff1b`](https://github.com/thr32423/mb3d-rs/commit/d5eff1b7382aac5b2e2398e07e29072aa806b67c) | Version number and release notes of this release |
| [`568f351`](https://github.com/thr32423/mb3d-rs/commit/568f351a5ed474d40f6d3e24907db8320800455a) | The background picture's load options of the Lighting window work as in MB3D: "Smooth image on load" (TBoptions bit 19), "Scale rows to geographic projection" (AdditionalOptions bit 7) and "Fit left+right borders" (AdditionalOptions bit 4) were read by neither the file reader nor the window, so a scene saved with them rendered with the raw picture. maps::background applies SmoothLightMap (a 5 point filter down the columns and along the rows with wrap-around, 8 bit pictures only) and FitLMborder (the 6 outer columns of each side blended into the other) ported step for step from maps/Maps.pas, including the values the last row and column see; the bits are kept in .m3p/.m3i and as background_smooth, background_geographic, background_fit_borders in the text parameters; the three check boxes of the Lighting window are wired. Of the 80 example files only "material colors" (geographic + fit borders) changes; the other three with a background picture are byte-identical. Found through Mandelbulb3D-SE (Rodrigo-Spacecraft), whose copy of the engine reads the bits; the filters here follow MB3D's Pascal, not its approximation. Tests: the filters on synthetic pictures, the bits and keys round trip; TECHNICAL.md |
| [`33b631f`](https://github.com/thr32423/mb3d-rs/commit/33b631f4acfa6daa8543acddbc39fe55532d031d) | TECHNICAL.md: the reflection rounds' first-round share and one-round rule, the command line's unlimited wait for a shader compile |
| [`f42eba6`](https://github.com/thr32423/mb3d-rs/commit/f42eba6a7a3ed3352bfe35a7ead135eaed169a99) | The first reflection round keeps half the rays for the CPU threads only when the round is a few slot-fulls of the card (under four times its 32768 slots): at half size and above the rounds share well by themselves and the fixed share cost ImpossibleWorld dIFS 7 %; MB3D_REFLECT_CPU_FINISH_ONE sets the one-round rule's share. Reflection stage against v0.3.3, best of two: a quarter size 20 to 38 % faster on four scenes, half and three-quarter sizes the same within 1 % |
| [`1381625`](https://github.com/thr32423/mb3d-rs/commit/1381625f642c873634a229f4cea7686719da9629) | Reflections shared with the card: one round with the CPU threads at three times the card's rate sends the remaining rounds to the CPU (before, both kinds of round had to show the CPU's part at 60 %). Reflection stage at a quarter size against v0.3.3, best of three: ImpossibleWorld dIFS 23.1 s to 17.5 s, Hal-Tenny 1.78 to 1.33, ApolloBalloons 2.61 to 2.07, Melting spot bloxx 1.05 to 0.66, Rama 0.53 to 0.40, Sabine62 0.82 to 0.60, Fishbowl 0.17 to 0.14, Torii temple and cutted sphere the same |
| [`1e5e263`](https://github.com/thr32423/mb3d-rs/commit/1e5e2637cc5faeb37dd4f5ba745eda481353a922) | Reflections shared with the card: the first round, before any rate is measured, keeps half the rays for the CPU threads (the card claimed up to two thirds and the threads waited on it when it was the slower side; MB3D_REFLECT_CPU_PRIOR sets the share). Reflection stage at a quarter size, best of three: Hal-Tenny 2.16 s to 1.30 s, Melting spot bloxx 1.06 to 0.71, ApolloBalloons 2.61 to 2.15, ImpossibleWorld dIFS 23.2 to 21.7, cutted sphere 0.29 to 0.28 |
| [`de33b3e`](https://github.com/thr32423/mb3d-rs/commit/de33b3eef468fa883a8da35e68cf1a191c0e7c4d) | The command line waits for a background shader compile as long as it takes before ending (MB3D_COMPILE_WAIT_SECS limits it), saying so every 30 s: the limit of 120 s cut off the scenes whose compile takes about that long (Abominog-Ua: 115-135 s a pipeline on an RTX 2060), so every run of them compiled again and the driver's cache never got them; now the second run has them in 0.1 s |
| [`38db5f3`](https://github.com/thr32423/mb3d-rs/commit/38db5f33f46a3b356b704390762b32d767ea9799) | Test: a render stopped in the march or in a post pass and continued from its checkpoint equals the uninterrupted render byte for byte, also with another thread count |
| [`4c037d1`](https://github.com/thr32423/mb3d-rs/commit/4c037d1dc2c3b102e214150c3634280e9c26e4c5) | README: Updates for v0.3.3 |

### [v0.3.3](https://github.com/thr32423/mb3d-rs/releases/tag/v0.3.3) · 7 Nov 2026, 16:03 UTC

| Commit | What it does |
|---|---|
| [`7f9f78d`](https://github.com/thr32423/mb3d-rs/commit/7f9f78d87161becb0c8b32277f523dc97fccdadc) | Version number and release notes of this release |
| [`5b98668`](https://github.com/thr32423/mb3d-rs/commit/5b98668d0c22e06eeb04ed2a21b367fe0da0637d) | Both parts of a DE combination run through compiled loops: the machine keeps one loop per slot range and the iteration takes the one of the range it is switched to, so part 2 no longer calls its formulas one by one (the dIFS loop the same); the timing that decides whether a loop is kept runs the whole distance estimate for a DE combination (the iteration alone never ran part 2). Two objects side by side render up to 27 % faster on the CPU (the example files DEcomb1 14 %, julia on box 21 %, ExcludeBulbMeng 6 %), the pictures the same to the bit (80 example files, tools/jit_check.sh, and the 11 DE combinations against v0.3.2); a test renders a two-object scene through both loops against the formulas called one by one. README (17 languages) and TECHNICAL.md |
| [`767c6ad`](https://github.com/thr32423/mb3d-rs/commit/767c6ad5212555522f2acb5e505f40a9925e1429) | README: Updates for v0.3.2 |

All 116 releases, back to v0.1.0: [the whole list](docs/UPDATES.md).
<!-- updates:end -->

## For developers

The [technical notes](docs/TECHNICAL.md) describe how the graphics card
is used, what was ported from MB3D's Delphi sources and how, the three
ways MB3D's formula machine code is run, the scene file format and its
keys, the checks against MB3D, and building from source:

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

## License

MB3D is licensed under the LGPL 2.1. This port is a derivative work and uses
the same license.

## Credits

Mandelbulb3D, which this program is a port of, was made by these people:

| Author | What they did |
|---|---|
| **Jesse** (fractalforums.com) | Wrote the original Mandelbulb3D (2010–2013, Delphi and 32-bit assembly): the distance estimation renderer, Navigator, formula and hybrid system, post processing (shadows, ambient occlusion, depth of field, reflections), Animation maker, Big renders, Monte Carlo renderer. Released it as freeware; his last release was 1.8.9 (April 2013) |
| **Andreas Maschke** (thargor6) | Maintainer since 2015: published Jesse's source code on GitHub, moved it to Delphi XE6, released 1.9.0 to 1.99.37 (2023). Added the JIT formula editor, MutaGen, BulbTracer and BulbTracer2 mesh export, the HeightMap generator, ZBuf16Bit, animated maps, the themed interface, point cloud export, Monte Carlo batch and network rendering. His code is under the LGPL 2.1 |
| **DarkBeam** (Luca GN) | By far the most formulas (credited in 210 of 464 `.m3f` files); the formula bundle of 1.8.2; the interim builds 1.8.9 x1 and x2 |
| **David Makin** | Helped Jesse implement the distance estimation; formulas (12 files) |
| **Tglad** (Tom Lowe) | Invented the Amazing Box (Mandelbox); formulas (13 files) |
| **Buddhi** | The fast distance estimation of the Amazing Box; formulas (6 files) |
| **Daniel White** | With Paul Nylander, the 2009 "triplex" Mandelbulb that MB3D's built-in bulb formula is based on; the fast integer power formulas Jesse used |
| **Paul Nylander** | With Daniel White, the Mandelbulb |
| **M. Benesi** | Formulas (33 files) |
| **Knighty** | Formulas (18 files), among them Knots2, koch_surf, the JCube3 approximation, and JosKn-KleinIFS with Jos Leys |
| **Aexion** | Formulas (12 files), among them the built-in "Aexion C" |
| **Kali** | Formulas (8 files), among them Amazing Surf |
| **Bethchen** | Formulas (8 files) |
| **msltoe** | The Riemann formula |
| **Fracmonk** | The CommQuat formula |
| **Trafassel** | The IdesFormula |
| **Syntopia** | Inspiration and testing; the Sierpinski4 and Menger4 formulas |
| **Jos Leys** | JosKn-KleinIFS, with Knighty |
| **Mark Townsend, Inigo Quilez, T. Gintz, Kerry Mitchell, Pseudogenius, dwsel** | Formulas |
| **Alef** (Edgar Malinovsky) | The JIT formula pack (`EM_JIT_M3Formulas`) |
| **Schmiegel** | Test parameters for the 1.99 releases |
| **Matteo** | Suggested and supported Monte Carlo batch rendering |
| **Zeyu-Li** | Documentation on GitHub (2020) |
| **NASA, GIMP** | Source material for the maps |

* **Contributors to this program**: Rathinagiri S
  ([SRathinaGiri](https://github.com/SRathinaGiri)): the stereo pair
  preview ([#12](https://github.com/thr32423/mb3d-rs/pull/12)), saving stereo pairs, the
  Navigator's axes and the grouped taskbar
  ([#13](https://github.com/thr32423/mb3d-rs/pull/13)), the stereo controls
  ([#14](https://github.com/thr32423/mb3d-rs/pull/14)), sorting in the file dialogs
  ([#17](https://github.com/thr32423/mb3d-rs/pull/17)) and their sortable columns
  ([#19](https://github.com/thr32423/mb3d-rs/pull/19)), the program icon in the Windows
  executable, and the animation templates, 360° photos and 360° VR videos
  ([#30](https://github.com/thr32423/mb3d-rs/pull/30)).
* **Bug reports and suggestions**:
  [kronpano](https://github.com/kronpano)
  ([#1](https://github.com/thr32423/mb3d-rs/issues/1),
  [#2](https://github.com/thr32423/mb3d-rs/issues/2),
  [#3](https://github.com/thr32423/mb3d-rs/issues/3),
  [#4](https://github.com/thr32423/mb3d-rs/issues/4),
  [#8](https://github.com/thr32423/mb3d-rs/issues/8)),
  [lukastrips](https://github.com/lukastrips)
  ([#6](https://github.com/thr32423/mb3d-rs/issues/6),
  [#20](https://github.com/thr32423/mb3d-rs/issues/20)),
  [Rodrigo-Spacecraft](https://github.com/Rodrigo-Spacecraft)
  ([#9](https://github.com/thr32423/mb3d-rs/issues/9)), Michael Pettovello,
  Geoffrey N. Barnes.
* **The PseudoKleinian formula** is knighty's (Fragmentarium, 2011,
  "Theli-at's Pseudo Kleinian" after Theli-at's hybrid discovery of the
  look on fractalforums.com); this program's version is written in Rust
  and WGSL from its algorithm.
* **The SphereTree formula** is by Tom Lowe (TGlad): recursive Eisenstein
  Ford sphere packings on a Riemann sphere projection, published on
  [fractalforums.org](https://fractalforums.org/fractal-mathematics-and-new-theories/28/new-sphere-tree/3557)
  in June 2020 with a Fragmentarium file
  ([TGlad/FragmentariumExamples](https://github.com/TGlad/FragmentariumExamples)).
  This program's version is written in Rust and WGSL from that
  algorithm; the colouring per branch was added.
* **The DomeTree formula** is by Tom Lowe (TGlad), `DomeTree.frag` in
  [TGlad/FragmentariumExamples](https://github.com/TGlad/FragmentariumExamples);
  this program's version is written in Rust and WGSL from that
  algorithm, with a colour per dome added.
* **The fonts of the languages** are Noto Sans CJK (subsets), Noto Sans
  Devanagari, Noto Sans Tamil and Noto Sans Arabic by the Noto Project
  Authors (SIL Open Font License, `Languages/OFL-*.txt`); Devanagari, Tamil
  and Arabic are shaped with
  [rustybuzz](https://github.com/harfbuzz/rustybuzz) (MIT), the Rust port
  of HarfBuzz.
* **The icon** is Mandelbulb3D's own icon (`m3d128.ico` from
  [thargor6/mb3d](https://github.com/thargor6/mb3d), LGPL 2.1), with its
  background made transparent and the letters "RS" (for the Rust port)
  added; the letters are set in Liberation Sans Bold (SIL Open Font
  License).
* **The noise filter** is the DCT image denoising algorithm of Guoshen Yu
  and Guillermo Sapiro: *DCT image denoising: a simple and effective image
  denoising algorithm*, Image Processing On Line 1 (2011),
  [doi:10.5201/ipol.2011.ys-dct](https://doi.org/10.5201/ipol.2011.ys-dct).
  It is the algorithm of **GIMP**'s *Denoise DCT* filter (from GIMP's
  image library [GEGL](https://gegl.org)); GIMP's filters were compared
  on Monte Carlo pictures, and this one came out best. This program has
  its own implementation of the algorithm, written from the paper; it
  contains no code of GIMP or GEGL (GEGL is under the LGPL 3, this
  program under the LGPL 2.1).
* **The neural noise filter** is
  [Intel Open Image Denoise](https://www.openimagedenoise.org) by Intel
  (Apache License 2.0), shipped as Intel releases it, with the
  [oneTBB](https://github.com/uxlfoundation/oneTBB) library it uses
  (Apache License 2.0); their licences are in `oidn/doc`. The program
  loads it while running.
* **The formulas are compiled at run time** with
  [Cranelift](https://cranelift.dev) of the Bytecode Alliance
  (Apache License 2.0 with LLVM exception), on Linux and Windows.
