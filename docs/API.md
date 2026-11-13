# Using the engine from other programs

EXPERIMENTAL: the requests and the Rust functions below may still change
between versions.

Three ways, the first two without writing Rust:

| way | for | how |
|---|---|---|
| command line | one picture per call | `mb3d scene.m3p -o picture.png -W 1280` (`mb3d --help`) |
| `mb3d serve` | any language, many pictures | JSON lines on standard input and output, below |
| Rust | Rust programs | `mb3d::api`, below |

Nothing listens on the network in any of them.

## `mb3d serve`

A program starts `mb3d serve` and keeps it running. It writes one request
per line to the program's standard input and reads the answers, one per
line, from its standard output. The process keeps the open scenes, the
graphics card and the compiled formulas, so every picture after the first
starts at once.

    mb3d serve [--cpu] [--formulas DIR] [--maps DIR]

A request is a JSON object with `op` and, if wanted, an `id` (a number or
text) that the answer repeats. An answer has `ok`: `true` with the
results, or `false` with `error` (a sentence).

    → {"id":1,"op":"open","preset":"Amazing Box"}
    ← {"id":1,"ok":true,"scene":1,"width":800,"height":600,"formulas":[{"name":"Amazing Box","iterations":1}],"notes":[]}
    → {"id":2,"op":"set","scene":1,"changes":"zoom = 0.3\niterations = 12"}
    ← {"id":2,"ok":true,"scene":1,"width":800,"height":600,...}
    → {"id":3,"op":"render","scene":1,"width":640,"file":"/tmp/box.png"}
    ← {"id":3,"ok":true,"width":640,"height":480,"seconds":0.41,"device":"GPU (...)","file":"/tmp/box.png"}

Requests are answered in the order they were sent. The standard error
output carries messages for people; the standard output only answers.

| op | members | answer |
|---|---|---|
| `hello` | | `name`, `version`, `protocol` (1), `gpu` (built with graphics card support) |
| `open` | one of `file` (a path), `text` (scene text or MB3D text parameters), `data` (a file's bytes as Base64) with `name` (its file name), `preset` (a formula's name) | `scene` (its number), `width`, `height`, `formulas` (each with `slot`, `name`, `iterations` and `settings`, its own values), `rotation` (the view as the three angles of the `rotation` key, or null), `notes` (what the loader could not carry over) |
| `text` | `scene` | `text`: the scene as `key = value` lines |
| `set` | `scene`, `changes` (`key = value` lines of global keys) | like `open`; a wrong line changes nothing |
| `formula` | `scene`, `slot` (the formula's place, from `formulas`), `settings` (its own values by name, e.g. `{"Scale": 2.2}`), `iterations` (optional) | like `open`; a wrong name changes nothing |
| `resize` | `scene`, `width` and/or `height` | like `open`; the look of the picture is kept |
| `copy` | `scene` | like `open`: a second scene with the same settings |
| `save` | `scene`, `file` | writes a `.m3p` (a `.m3s` or `.txt` name: the scene text) |
| `close` | `scene` | |
| `render` | `scene`; `width`, `height` (this picture only); `file` (a PNG to write); `depth_file` (a 16 bit depth PNG); `format`; `depth`; `progress` | `width`, `height`, `seconds`, `device`, and the picture, below |
| `formulas` | `name` (optional) | without: `names`; with: `settings` (the formula's keys and default values) |
| `guide` | | `text`: which scene keys do what |
| `gpu` | `on` | calculate on the graphics card or the processor |
| `cancel` | | stops every `render` sent before it (running or waiting): they answer `"error":"cancelled"`; other requests are still carried out |

The picture of `render`:

* `format` `"png"` (the default without `file`): `png`, the PNG file as
  Base64; with `depth: true` also `depth_png`.
* `format` `"rgb"`: `rgb`, the pixels as Base64 (3 bytes each, rows from
  the top); with `depth: true` also `depth` (2 bytes each, little endian;
  65535 nearest, 0 background).
* `format` `"none"` (the default with `file`): only the files.

With `progress: true` lines like this come before the answer, about ten a
second: `{"id":3,"event":"progress","stage":"","done":120,"total":480}`
(`stage` is empty during the march, then names the stage: hard shadows,
ambient occlusion, ...; `done` starts again with every stage).

`formula` changes a formula's own values. To add or replace formulas or to
change lights, take the scene's `text`, edit it and `open` it again as
`text` (`set` changes global keys only; `guide` explains).

The process ends when its standard input is closed.

## Rust

    let mut f = mb3d::api::Fractal::preset("Integer Power")?;
    f.set("iterations = 14\nzoom = 1.1")?;
    f.resize(640, 480);
    let pic = f.render(&mb3d::api::RenderOptions::default())?;
    std::fs::write("bulb.png", pic.png())?;

`Fractal::open`, `from_bytes`, `from_text` and `preset` make a scene;
`text`, `set`, `set_formula`, `resize`, `formula_list`, `m3p` read and
change it; `render`
gives a `Picture` (`rgb`, `depth`, `png()`, `depth_png()`), reporting
progress and asking a `cancel` function through `RenderOptions`.
`api::use_gpu(true)` switches the graphics card on (a library starts with
it off); a program that did calls `api::finish()` before it ends. `cargo doc --open` shows the rest. The other modules of the crate
are the program's internals and change without notice.

## Other languages

Wrappers around `mb3d serve` (Python, JavaScript) and a C library with
examples are kept in their own repository.

## Licence

LGPL 2.1: a program that starts `mb3d` or loads it as a separate library
may have any licence; changes to mb3d itself are published under the LGPL.
