# iPad app: what comes over from the desktop version

The desktop app's windows compared with the iPad app (2026-11-21). The
user chose to bring over everything that fits a touch app, including the
big features. Each phase is checked on the GitHub Mac runner (workflow
`iPad app`) before the next one.

## Already in the app

Open (examples, own files from the Files app), Save (.m3p), Export, Share
(PNG); Navigator gestures (look, slide, fly, roll, reset); camera zoom and
field of view; iterations, DE stop, ray step; each formula (choice and
values); global lighting and each light (colour, strength, direction);
background colours, depth fog, far fog, glowing fog; shadows, ambient
occlusion and reflections on or off; render and stop.

## Phase 1: the main window and its effects

- Picture: aspect (square, 4:3, 3:2, 16:9, the screen), size, quality
  (preview, mid, high), a full-size render; save the picture to Photos
  (PNG, JPEG).
- Parameters to and from the clipboard (MB3D text, as on the forums).
- Undo and redo (History).
- Formulas: add and remove slots (up to 6), repeat from here, analytic DE
  off; Julia mode and its values; cutting planes.
- Lighting: a light on/off, add a light, spec power, shading style;
  palette (colour stops, start, end, cycling); ambient shadow.
- Effects: depth of field with tap-to-focus on the picture; shadows per
  light, soft; reflection amount; finishing (exposure, tone map, glow,
  contrast, saturation, warmth, vignette, chromatic); clouds; material;
  ground plane and water; surface detail.
- Camera lens (`optic`).
- Infos: author and notes.

## Phase 2: Variations (MutaGen)

A grid of 15 variations of the scene; a tap takes one, and the next
generation starts from it. Needs C functions over `mutagen::generation`.

## Phase 3

- Background picture: a photo from the Photos library or an HDR as
  background or light sphere.
- Own formulas: .m3f files from the Files app into the formula folder
  (no JIT on iOS).
- 18 languages (see Languages below).

## Phase 4

- Monte Carlo render (no OIDN on iOS).
- Stereo pairs; 360° panoramas (Facebook 360 photo tags).
- Droste effect. (The desktop's Curves window is the animation's value
  curves, not tone curves: it belongs to phase 5.)

## Phase 5

- Animation: keyframes from the views or a turntable, rendered into an
  MP4 (AVFoundation); the value curves (the Curves window: a chart per
  value, keyframe points dragged), motion blur (4, 8 or 16 pictures within
  the shutter time).
- 3D export: BulbTracer mesh as USDZ, seen in AR through Quick Look; OBJ,
  PLY, STL to share. (Gaussian splats stay desktop-only: no viewer on the
  iPad.)

## Languages

The texts are SwiftUI keys (English); `l10n/extract.py App l10n/en.json`
lists them, `l10n/<language>.json` hold the translations (17 languages, the
desktop's), `l10n/make_strings.py` writes `App/<language>.lproj`.

## Not for the iPad

Internal tab (threads, priority), benchmark, ini folders, themes, hotkeys,
swarm and network rendering, batch and big tiled renders, Z-buffer and
height map export, JIT formula editor, texture export, AI connection, log
window, m3p→m3i converter, Steps and noise.
