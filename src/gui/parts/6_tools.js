// ------------------------------------------------------------ animation
// MB3D's animation maker: keyframes are taken from the editor; the server
// interpolates the frames, renders a flipbook preview and the frame files.
let anim = null, animVer = -1, animSel = -1, animPollTimer = null;
let flip = { gen: -1, pos: 0, playing: false, timer: null, cache: new Map() };
async function animLoad() { try { setAnim(await api("/api/anim")); } catch (e) { showError(e.message); } }
function setAnim(a) {
  const structural = !anim || a.ver !== animVer;
  anim = a; animVer = a.ver;
  if (a.flip.gen !== flip.gen) { flip.gen = a.flip.gen; flip.cache = new Map(); flip.pos = 0; }
  if (isOpen("anim")) {
    const busy = document.activeElement && $("w_anim").contains(document.activeElement) && document.activeElement.tagName === "INPUT"
      && document.activeElement.type !== "range";
    if (structural && !busy) renderWin("anim"); else animUpdate();
  }
  scheduleAnimPoll();
}
function scheduleAnimPoll() {
  clearTimeout(animPollTimer);
  const active = anim && (anim.flip.running || anim.job.running);
  if (isOpen("anim") || active) animPollTimer = setTimeout(animLoad, active ? 400 : 1500);
}
async function animKey(op, extra = {}) {
  try {
    const r = await post("/api/anim/key", form({ op, ...extra }));
    if (op === "edit" || op === "edit_frame") applyState(r.state);
    setAnim(r.anim);
  } catch (e) { showError(e.message); }
}
async function animSet(o) { try { setAnim(await post("/api/anim/set", form(o))); } catch (e) { showError(e.message); } }
function framePosText(i) {
  // which keyframe a file index belongs to
  if (!anim) return "";
  let k = -1;
  anim.keys.forEach((key, j) => { if (key.first <= i) k = j; });
  if (k < 0) return "";
  const sub = Math.round((i - anim.keys[k].first) / Math.max(1, anim.index_step));
  return `keyframe ${k + 1}` + (sub ? ` + ${sub}/${anim.keys[k].frames}` : "");
}
function flipShow(n) {
  const f = anim && anim.flip;
  if (!f || !f.total) return;
  flip.pos = Math.max(0, Math.min(f.total - 1, n));
  const im = $("flipImg"), sl = $("flipSlider");
  if (sl) sl.value = flip.pos;
  if (im && f.ready[flip.pos]) {
    let c = flip.cache.get(flip.pos);
    if (!c) { c = new Image(); c.src = `/api/anim/frame?i=${flip.pos}&g=${f.gen}`; flip.cache.set(flip.pos, c); }
    im.src = c.src;
  }
  const t = $("flipText");
  if (t) t.textContent = `frame ${String(f.indices[flip.pos]).padStart(6, "0")} · ${framePosText(f.indices[flip.pos])}` + (f.ready[flip.pos] ? "" : " (not rendered yet)");
}
function flipPlay(on) {
  flip.playing = on;
  clearInterval(flip.timer);
  const b = $("flipPlay"); if (b) b.textContent = on ? "❚❚" : "▶";
  if (!on) return;
  const fps = +($("flipFps") ? $("flipFps").value : 12);
  flip.timer = setInterval(() => {
    const f = anim.flip;
    if (!f.total) return;
    let n = flip.pos + 1;
    if (n >= f.total || !f.ready[n]) n = 0;
    flipShow(n);
  }, 1000 / fps);
}
function animUpdate() {
  // dynamic parts only (progress, flipbook, job)
  if (!anim) return;
  const f = anim.flip, j = anim.job;
  const ready = f.ready.filter(x => x).length;
  const fb = $("flipBar"); if (fb) fb.firstElementChild.style.width = f.total ? (100 * (ready + (f.running ? f.progress / 100 : 0)) / f.total) + "%" : "0";
  const fs = $("flipState"); if (fs) fs.textContent = f.error ? f.error : f.total ? `${ready} of ${f.total} preview frames` + (f.running ? " …" : "") : "";
  const sl = $("flipSlider"); if (sl) sl.max = Math.max(0, f.total - 1);
  if (!flip.playing) flipShow(flip.pos);
  const jb = $("jobBar"); if (jb) jb.firstElementChild.style.width = j.total ? (100 * (j.done + j.skipped + (j.running ? j.progress / 100 : 0)) / j.total) + "%" : "0";
  const js = $("jobState");
  if (js) {
    let t = "";
    if (j.total) {
      const n = j.done + j.skipped;
      t = j.running ? `frame ${j.current} of ${j.total} · ${j.progress.toFixed(0)} %` : `${j.done} rendered, ${j.skipped} skipped of ${j.total}`;
      if (j.done && j.running) { const per = j.seconds / Math.max(1, n); t += ` · about ${fmtTime(per * (j.total - n))} left`; }
      if (!j.running) t += ` · ${fmtTime(j.seconds)}`;
      if (j.error) t += ` · ${j.error}`;
    }
    js.textContent = t;
  }
  const jl = $("jobLast"); if (jl) jl.textContent = j.last ? "last: " + j.last : "";
  const rb = $("jobStart"); if (rb) rb.disabled = j.running;
  for (const k of anim.keys) { const im = $("th" + k.id); if (im && k.thumb && !im.getAttribute("src")) im.src = "/api/anim/thumb?id=" + k.id; }
}
function fmtTime(s) { s = Math.round(s); return s >= 3600 ? `${Math.floor(s / 3600)} h ${Math.floor(s / 60) % 60} min` : s >= 60 ? `${Math.floor(s / 60)} min ${s % 60} s` : `${s} s`; }

function animTab(p) {
  if (!anim) { p.append(el("div", { class: "hint" }, "loading…")); animLoad(); return; }
  const a = anim;
  // keyframes
  const kc = el("div", { class: "card" }, el("h3", {}, `Keyframes`, el("span", { class: "sp" }),
    el("span", { class: "hint" }, `${a.keys.length} keyframes · ${a.frame_count} frames`),
    el("button", { title: "Reverse the order", onclick: () => animKey("reverse") }, "⇅"),
    el("button", { title: "Delete all keyframes", onclick: () => { if (confirm("Delete all keyframes?")) animKey("clear"); } }, "✕ all")));
  kc.append(el("div", { class: "hint" }, "Navigate in the editor, then add the view as a keyframe. Frames: sub-frames from a keyframe to the next one. Click an image to load the keyframe into the editor."));
  a.keys.forEach((k, i) => {
    const th = el("img", { id: "th" + k.id, alt: "" });
    if (k.thumb) th.src = "/api/anim/thumb?id=" + k.id;
    const fr = el("input", { type: "text", id: "kfr" + k.id, value: k.frames, title: "Sub-frames to the next keyframe" });
    fr.addEventListener("change", () => animKey("frames", { i, frames: fr.value }));
    const last = i === a.keys.length - 1 && !a.loop;
    const box = el("div", { class: "kf" + (i === animSel ? " sel" : "") },
      el("div", { class: "th", title: "Load into the editor", onclick: () => { animSel = i; animKey("edit", { i }); } }, th),
      el("div", { class: "meta" }, el("b", {}, `#${i + 1}`), el("span", { class: "hint" }, `frame ${String(k.first).padStart(6, "0")}`),
        last ? el("span", { class: "hint" }, "(last)") : el("span", {}, "frames ", fr)),
      el("div", { class: "btns" },
        el("button", { title: "Replace this keyframe by the editor's parameters", onclick: () => { if (confirm(`Replace keyframe ${i + 1} by the current parameters?`)) animKey("replace", { i }); } }, "⇦ set"),
        el("button", { title: "Insert the editor's parameters after this keyframe", onclick: () => animKey("insert", { i }) }, "+ after"),
        el("button", { title: "Move up", onclick: () => animKey("up", { i }) }, "↑"),
        el("button", { title: "Move down", onclick: () => animKey("down", { i }) }, "↓"),
        el("button", { title: "Delete", onclick: () => animKey("delete", { i }) }, "✕")));
    kc.append(box);
  });
  const df = el("input", { type: "text", id: "aDefFrames", value: a.default_frames, size: 4, title: "Frames of new keyframes" });
  df.addEventListener("change", () => animSet({ default_frames: df.value }));
  const allBtn = el("button", { title: "Set this frame count for all keyframes", onclick: () => animKey("all_frames", { frames: df.value }) }, "for all");
  kc.append(el("div", { class: "addrow" }, el("button", { class: "primary", onclick: () => animKey("add") }, "+ Add current view as keyframe"),
    el("span", { class: "hint" }, " frames"), df, allBtn));
  p.append(kc);

  // flipbook preview
  const pw = el("select", { id: "flipW" }, ...[120, 160, 200, 320, 400].map(w => el("option", { value: w }, w + " px")));
  pw.value = String(a.flip.width || 200);
  const pe = el("select", { id: "flipEvery" }, ...[1, 2, 3, 5, 10].map(n => el("option", { value: n }, n === 1 ? "all frames" : `every ${n}.`)));
  const pf = el("input", { type: "checkbox", id: "flipFast", checked: "" });
  const fps = el("select", { id: "flipFps" }, ...[6, 12, 25, 30].map(n => el("option", { value: n }, n + " fps")));
  fps.value = "12";
  fps.onchange = () => { if (flip.playing) flipPlay(true); };
  const sl = el("input", { type: "range", id: "flipSlider", min: 0, max: Math.max(0, a.flip.total - 1), value: flip.pos });
  sl.addEventListener("input", () => { flipPlay(false); flipShow(+sl.value); });
  p.append(el("div", { class: "card" }, el("h3", {}, "Preview"),
    el("div", { class: "multi" }, pw, pe, el("label", { title: "No shadows, volumetric light or DEAO (like MB3D's fast preview)" }, pf, " fast"),
      el("button", { class: "primary", onclick: async () => {
        try { setAnim(await post("/api/anim/preview", form({ width: pw.value, every: pe.value, fast: pf.checked }))); flipPlay(false); }
        catch (e) { showError(e.message); } } }, "Preview frames"),
      el("button", { onclick: async () => setAnim(await post("/api/anim/stop", "")) }, "Stop")),
    el("div", { class: "bar", id: "flipBar" }, el("div")),
    el("div", { class: "hint", id: "flipState" }),
    el("img", { id: "flipImg", alt: "" }),
    el("div", { class: "flipctl" }, el("button", { id: "flipPlay", onclick: () => flipPlay(!flip.playing) }, "▶"), sl, fps),
    el("div", { class: "addrow" }, el("span", { class: "hint", id: "flipText" }),
      el("button", { title: "Load this interpolated frame into the editor (e.g. to add it as a keyframe)", onclick: () => {
        const f = anim.flip; if (!f.total) return;
        const fi = f.indices[flip.pos];
        animKey("edit_frame", { f: Math.round((fi - anim.start_index) / Math.max(1, anim.index_step)) });
      } }, "Open frame in editor"))));

  // settings
  const inp = (id, v, key, size) => {
    const e = el("input", { type: "text", id, value: v, size: size || 6 });
    e.addEventListener("change", () => animSet({ [key]: e.value }));
    return e;
  };
  const chk = (id, v, key) => {
    const e = el("input", { type: "checkbox", id });
    e.checked = v;
    e.addEventListener("change", () => animSet({ [key]: e.checked }));
    return e;
  };
  const sel = (id, v, key, opts) => {
    const e = el("select", { id }, ...opts.map(([val, lab]) => el("option", { value: val }, lab)));
    e.value = v;
    e.addEventListener("change", () => animSet({ [key]: e.value }));
    return e;
  };
  p.append(el("div", { class: "card" }, el("h3", {}, "Settings"),
    el("div", { class: "row" }, el("label", {}, "frame size"), el("div", { class: "multi" }, inp("aW", a.width, "width"), "×", inp("aH", a.height, "height"))),
    el("div", { class: "row" }, el("label", { title: "MB3D's image scale: frames are calculated n times larger and reduced" }, "anti-aliasing"),
      sel("aAA", String(a.aa), "aa", [["1", "off"], ["2", "2×2"], ["3", "3×3"], ["4", "4×4"]])),
    el("div", { class: "row" }, el("label", {}, "interpolation"), sel("aIp", a.interp, "interp", [["bezier", "quadratic bezier (smooth)"], ["linear", "linear"]])),
    el("div", { class: "row" }, el("label", { title: "After the last keyframe the first one follows again" }, "loop animation"), chk("aLoop", a.loop, "loop"))));

  // output
  const from = el("input", { type: "text", id: "jFrom", size: 6, placeholder: "first" });
  const to = el("input", { type: "text", id: "jTo", size: 6, placeholder: "last" });
  const sample = `${a.output_abs.replace(/[\\/]$/, "")}/${a.name}${String(a.start_index).padStart(6, "0")}.${a.format}`;
  p.append(el("div", { class: "card" }, el("h3", {}, "Render frames"),
    el("div", { class: "row" }, el("label", {}, "folder"), inp("aOut", a.output, "output", 20)),
    el("div", { class: "row" }, el("label", {}, "name"), inp("aName", a.name, "name", 14)),
    el("div", { class: "row" }, el("label", {}, "format"), sel("aFmt", a.format, "format", [["png", "PNG"], ["bmp", "BMP"], ["m3p", "parameter files (.m3p)"]])),
    el("div", { class: "row" }, el("label", {}, "file index from / step"), el("div", { class: "multi" }, inp("aStart", a.start_index, "start_index"), inp("aStep", a.index_step, "index_step"))),
    el("div", { class: "row" }, el("label", { title: "Otherwise frames whose file exists are skipped (continue a render, or share it with other processes)" }, "overwrite existing"), chk("aOver", a.overwrite, "overwrite")),
    el("div", { class: "row" }, el("label", {}, "depth images too"), chk("aDepth", a.depth, "depth")),
    el("div", { class: "row" }, el("label", { title: "Stereo: a right and a left eye image per frame (see the scene's stereo_screen), or only the 'very left' image" }, "stereo"),
      sel("aStereo", a.stereo, "stereo", [["off", "off"], ["pair", "right + left eye"], ["very_left", "only 'very left' eye"]])),
    el("div", { class: "row" }, el("label", {}, "only file indices"), el("div", { class: "multi" }, from, "to", to)),
    el("div", { class: "hint mono" }, sample),
    el("div", { class: "multi" },
      el("button", { class: "primary", id: "jobStart", onclick: async () => {
        try { setAnim(await post("/api/anim/render", form({ from: from.value, to: to.value }))); } catch (e) { showError(e.message); } } }, "Render frames"),
      el("button", { onclick: async () => setAnim(await post("/api/anim/stop", "")) }, "Stop")),
    el("div", { class: "bar", id: "jobBar" }, el("div")),
    el("div", { class: "hint", id: "jobState" }),
    el("div", { class: "hint mono", id: "jobLast" })));

  // files
  const fi = el("input", { type: "file", accept: ".m3k,.m3a", hidden: "" });
  fi.onchange = async () => {
    const f = fi.files[0]; if (!f) return;
    try {
      const r = await post("/api/anim/open?name=" + encodeURIComponent(f.name), await f.arrayBuffer());
      animSel = -1; setAnim(r.anim);
      if (r.notes.length) showError(r.notes.join("\n"));
    } catch (e) { showError(e.message); }
  };
  p.append(el("div", { class: "card" }, el("h3", {}, "Animation file"), fi,
    el("div", { class: "multi" },
      el("button", { onclick: () => fi.click() }, "Open .m3k / .m3a…"),
      el("button", { onclick: () => { location.href = "/api/anim/save?fmt=m3k"; } }, "Save .m3k"),
      el("button", { onclick: () => { location.href = "/api/anim/save?fmt=m3a"; } }, "Save .m3a (MB3D)")),
    el("div", { class: "hint" }, ".m3k is this program's text format (keyframes as MB3D text parameters); .m3a opens in MB3D's animation maker. On the command line: mb3d animate file.m3k")));
  animUpdate();
}

// ------------------------------------------------------------ MutaGen
// MB3D's MutaGen: a tree of 15 mutations of the editor's scene (two
// children per member, four levels). Click to select, double-click to load
// a mutation into the editor, "Breed" makes a new generation from it.
let muta = null, mutaVer = -1, mutaSel = -1, mutaPollTimer = null;
const mutaCfg = { formula_weight: "0.75", params_weight: "1", params_strength: "1", julia_weight: "0.5", julia_strength: "1",
  its_weight: "0.5", its_strength: "1", probing: true };
async function mutaLoad() { try { setMuta(await api("/api/muta")); } catch (e) { showError(e.message); } }
function setMuta(m) {
  const structural = !muta || m.current !== muta.current || m.count !== muta.count || m.running !== muta.running;
  if (structural && muta && (m.current !== muta.current || m.count !== muta.count)) mutaSel = -1;
  const changed = m.ver !== mutaVer;
  muta = m; mutaVer = m.ver;
  if (isOpen("muta") && changed) { if (structural) renderWin("muta"); else mutaUpdate(); }
  clearTimeout(mutaPollTimer);
  if (isOpen("muta") || m.running) mutaPollTimer = setTimeout(mutaLoad, m.running ? 300 : 1500);
}
function hashStr(s) { let h = 0; for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0; return (h >>> 0).toString(36); }
function sceneAspect() {
  const g = k => parseFloat((model && model.global.find(l => l.k === k) || { v: "" }).v);
  const w = g("width"), h = g("height");
  return w > 0 && h > 0 ? h / w : 0.75;
}
async function mutaStart(i) {
  const o = { ...mutaCfg };
  if (i !== undefined) o.i = i;
  try { mutaSel = -1; setMuta(await post("/api/muta/start", form(o))); } catch (e) { showError(e.message); }
}
async function mutaUse(i) {
  try { applyState(await post("/api/muta/use", form({ i }))); setStatus(`mutation ${muta.members[i].label} loaded into the editor`); }
  catch (e) { showError(e.message); }
}
function mutaUpdate() {
  if (!muta) return;
  for (const m of muta.members) {
    const box = $("mm" + m.i);
    if (!box || box.dataset.key) continue;
    const key = hashStr(m.caption);
    box.dataset.key = key;
    box.classList.remove("wait");
    box.textContent = "";
    box.title = m.caption;
    box.append(el("img", { alt: "", src: `/api/muta/img?g=${muta.current}&i=${m.i}&k=${key}` }), el("span", {}, m.label));
  }
  const n = muta.members.length;
  const st = $("mutaState");
  if (st) st.textContent = muta.error ? muta.error : muta.count ? `generation ${muta.current + 1} of ${muta.count} · ${n} of 15 mutations` + (muta.running ? " …" : "") : "";
  const bar = $("mutaBar"); if (bar) bar.firstElementChild.style.width = (100 * n / 15) + "%";
  for (const id of ["mutaUse", "mutaBreed"]) { const b = $(id); if (b) b.disabled = mutaSel < 0 || mutaSel >= n || muta.running; }
  const cap = $("mutaCap");
  if (cap) cap.textContent = mutaSel >= 0 && mutaSel < n ? `${muta.members[mutaSel].label}: ${muta.members[mutaSel].caption}` : "";
  for (let i = 0; i < 15; i++) { const b = $("mm" + i); if (b) b.classList.toggle("sel", i === mutaSel); }
}
function mutaTab(p) {
  if (!muta) { p.append(el("div", { class: "hint" }, "loading…")); mutaLoad(); return; }
  const m = muta;
  const inp = (k, title) => {
    const e = el("input", { type: "text", class: "num", id: "mc_" + k, value: mutaCfg[k], title: title || "" });
    e.addEventListener("change", () => { mutaCfg[k] = e.value; });
    return e;
  };
  const prob = el("input", { type: "checkbox", id: "mc_probing" });
  prob.checked = mutaCfg.probing;
  prob.addEventListener("change", () => { mutaCfg.probing = prob.checked; });
  p.append(el("div", { class: "card" }, el("h3", {}, "Mutations"),
    el("div", { class: "hint" }, "Weights: how often a kind of mutation is chosen (0 = never). Strengths: how far parameters move."),
    el("div", { class: "row" }, el("label", { title: "Add, replace or remove a formula of the hybrid" }, "formulas: weight"), el("div", { class: "multi" }, inp("formula_weight"))),
    el("div", { class: "row" }, el("label", {}, "parameters: weight / strength"), el("div", { class: "multi" }, inp("params_weight"), inp("params_strength"))),
    el("div", { class: "row" }, el("label", {}, "julia: weight / strength"), el("div", { class: "multi" }, inp("julia_weight"), inp("julia_strength"))),
    el("div", { class: "row" }, el("label", {}, "iterations: weight / strength"), el("div", { class: "multi" }, inp("its_weight"), inp("its_strength"))),
    el("div", { class: "row" }, el("label", { title: "Try up to 9 mutations in a tiny render and keep the one with the most structure that differs from its parent" }, "probing"), prob),
    el("div", { class: "multi" },
      el("button", { class: "primary", id: "mutaGo", disabled: m.running ? "" : null, onclick: () => mutaStart() }, "Mutate editor scene"),
      el("button", { onclick: async () => setMuta(await post("/api/muta/stop", "")) }, "Stop"))));

  // the tree
  const asp = sceneAspect();
  const cw = 1 / 0.9, ch = asp / 0.8; // cell size in units of the preview width
  const tree = el("div", { class: "mtree", id: "mtree" });
  tree.style.aspectRatio = `${5 * cw} / ${4 * ch}`;
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", "0 0 5 4"); svg.setAttribute("preserveAspectRatio", "none");
  m.parents.forEach((par, i) => {
    if (par < 0) return;
    const [ax, ay] = m.layout[par], [bx, by] = m.layout[i];
    const ln = document.createElementNS(ns, "line");
    Object.entries({ x1: ax + 2.5, y1: ay + 2, x2: bx + 2.5, y2: by + 2, stroke: "#e0a24a", "stroke-width": 1.5, "vector-effect": "non-scaling-stroke",
      opacity: i < m.members.length ? 0.9 : 0.25 }).forEach(([k, v]) => ln.setAttribute(k, v));
    svg.append(ln);
  });
  tree.append(svg);
  m.layout.forEach(([x, y], i) => {
    if (!m.count) return;
    const box = el("div", { class: "mm wait", id: "mm" + i, onclick: () => { mutaSel = i; mutaUpdate(); },
      ondblclick: () => { if (i < muta.members.length) mutaUse(i); } }, m.running ? "…" : "–");
    box.style.left = `${(x + 2.5) * 20}%`; box.style.top = `${(y + 2) * 25}%`;
    box.style.width = `${100 / (5 * cw)}%`;
    tree.append(box);
  });
  p.append(el("div", { class: "card" }, el("h3", {}, "Generation", el("span", { class: "sp" }),
      el("button", { title: "Previous generation", disabled: m.current > 0 && !m.running ? null : "", onclick: async () => setMuta(await post("/api/muta/nav", form({ d: -1 }))) }, "◀"),
      el("button", { title: "Next generation", disabled: m.current + 1 < m.count && !m.running ? null : "", onclick: async () => setMuta(await post("/api/muta/nav", form({ d: 1 }))) }, "▶")),
    el("div", { class: "bar", id: "mutaBar" }, el("div")),
    el("div", { class: "hint", id: "mutaState" }),
    m.count ? tree : el("div", { class: "hint" }, "No mutations yet. \"Mutate editor scene\" makes a tree of 15: 1 is a mutation of the editor's scene, 1.1 and 1.2 mutations of 1, and so on."),
    el("div", { class: "hint mono", id: "mutaCap" }),
    el("div", { class: "multi" },
      el("button", { id: "mutaUse", class: "primary", onclick: () => mutaUse(mutaSel) }, "Open in editor"),
      el("button", { id: "mutaBreed", title: "A new generation with the selected mutation as its parent", onclick: () => mutaStart(mutaSel) }, "Breed from this")),
    el("div", { class: "hint" }, "Double-click a mutation to open it in the editor. On the command line: mb3d mutagen scene.m3p")));
  mutaUpdate();
}

// ------------------------------------------------------------ export
// MB3D's voxel stack export (PNG slices for voxel and 3D-printing tools) and
// BulbTracer2's mesh export.
let exp = null, expPollTimer = null, expWasRunning = false; let voxPrevUrl = "";
const expCfg = { slices: "100", test: "de", de: "", zscale: "", axes: false, vfolder: "voxels",
  resolution: "128", sharpness: "0.5", scale: "0.5", format: "stl", colors: false, close: false, smooth: "0" };
async function exportLoad() { try { setExport(await api("/api/export")); } catch (e) { showError(e.message); } }
function setExport(e) {
  const first = !exp;
  exp = e;
  for (const id of ["voxel", "btracer"]) if (isOpen(id)) { if (first || expWasRunning !== e.running) renderWin(id); else exportUpdate(); }
  expWasRunning = e.running;
  clearTimeout(expPollTimer);
  if (e.running) expPollTimer = setTimeout(exportLoad, 300);
}
function voxQuery() {
  const o = { slices: expCfg.slices, test: expCfg.test, axes: expCfg.axes };
  if (expCfg.de) o.de = expCfg.de;
  if (expCfg.zscale) o.zscale = expCfg.zscale;
  return o;
}
async function exportStart(kind) {
  const o = kind === "voxel" ? { kind, ...voxQuery(), folder: expCfg.vfolder }
    : { kind, resolution: expCfg.resolution, sharpness: expCfg.sharpness, scale: expCfg.scale, format: expCfg.format,
        colors: expCfg.colors, close: expCfg.close, smooth: expCfg.smooth };
  try { setExport(await post("/api/export/start", form(o))); } catch (e) { showError(e.message); }
}
function exportUpdate() {
  if (!exp) return;
  const b = $("expBar"); if (b) b.firstElementChild.style.width = (exp.running || exp.message ? exp.progress : 0) + "%";
  const s = $("expState");
  if (s) {
    const what = exp.kind === "voxel" ? "voxel slices" : exp.kind === "mesh" ? "mesh" : "";
    s.textContent = exp.running ? `${what} · ${exp.progress.toFixed(0)} %` : exp.error ? `${what}: ${exp.error}` : exp.message ? `${what}: ${exp.message}` : "";
    s.style.color = exp.error && !exp.running ? "var(--err)" : "";
  }
  const f = $("expFile"); if (f) f.hidden = exp.running || !exp.file;
  const fd = $("expFolder"); if (fd) fd.textContent = !exp.running && exp.folder ? "written to " + exp.folder : "";
  for (const id of ["expVox", "expMesh"]) { const x = $(id); if (x) x.disabled = exp.running; }
}
function exportTab(p, kind) {
  if (!exp) { p.append(el("div", { class: "hint" }, "loading…")); exportLoad(); return; }
  const inp = (k, title, ph) => {
    const e = el("input", { type: "text", id: "ex_" + k, value: expCfg[k], title: title || "", placeholder: ph || "" });
    e.addEventListener("change", () => { expCfg[k] = e.value; });
    return e;
  };
  const chk = k => {
    const e = el("input", { type: "checkbox", id: "ex_" + k });
    e.checked = expCfg[k];
    e.addEventListener("change", () => { expCfg[k] = e.checked; });
    return e;
  };
  const sel = (k, opts) => {
    const e = el("select", { id: "ex_" + k }, ...opts.map(([v, l]) => el("option", { value: v }, l)));
    e.value = expCfg[k];
    e.addEventListener("change", () => { expCfg[k] = e.value; });
    return e;
  };
  const voxImg = el("img", { id: "voxImg", alt: "" });
  if (voxPrevUrl) voxImg.src = voxPrevUrl;
  const voxPrev = () => { voxPrevUrl = voxImg.src = "/api/export/voxpreview?" + form({ ...voxQuery(), t: Date.now() }); };
  voxImg.onerror = () => { voxImg.removeAttribute("src"); voxPrevUrl = ""; showError("voxel preview failed"); };
  if (kind === "voxel") p.append(el("div", { class: "card" }, el("h3", {}, "Voxel slices"),
    el("div", { class: "hint" }, "A stack of black-and-white PNG slices through the object's bounding cube (as in MB3D's voxel export), for voxel editors and 3D printing. The cube is the area seen by the camera."),
    el("div", { class: "row" }, el("label", {}, "slices"), inp("slices", "Number of slices; the images are as large")),
    el("div", { class: "row" }, el("label", {}, "inside when"), sel("test", [["de", "distance estimate < DE stop"], ["iterations", "max. iterations reached"]])),
    el("div", { class: "row" }, el("label", { title: "A voxel is solid where the distance estimate is below this (in voxels)" }, "DE threshold"), inp("de", "", "from the DE stop")),
    el("div", { class: "row" }, el("label", {}, "z scale"), inp("zscale", "Slice spacing relative to the pixel size", "1")),
    el("div", { class: "row" }, el("label", { title: "Slice along the formula's x/y/z axes instead of the camera's view" }, "formula axes"), chk("axes")),
    el("div", { class: "row" }, el("label", {}, "folder"), inp("vfolder", "Relative to the server's working directory; a subfolder per scene")),
    el("div", { class: "multi" },
      el("button", { onclick: voxPrev, title: "All slices stacked, seen from the front" }, "Preview"),
      el("button", { class: "primary", id: "expVox", onclick: () => exportStart("voxel") }, "Write slices")),
    voxImg));
  if (kind === "mesh") p.append(el("div", { class: "card" }, el("h3", {}, "Mesh"),
    el("div", { class: "hint" }, "A triangle mesh by marching cubes over the distance estimate (MB3D's BulbTracer2): binary STL, OBJ or PLY. Vertex colours come from the scene's palette."),
    el("div", { class: "row" }, el("label", { title: "Grid cells per side of the cube; time and memory grow with its cube" }, "resolution"), inp("resolution")),
    el("div", { class: "row" }, el("label", { title: "0.5: the cube is twice the visible 2.2 / zoom; larger = smaller cube" }, "scale"), inp("scale")),
    el("div", { class: "row" }, el("label", { title: "Higher values make finer surfaces (and lower the DE stop)" }, "sharpness"), inp("sharpness")),
    el("div", { class: "row" }, el("label", {}, "format"), sel("format", [["stl", "STL (binary)"], ["obj", "OBJ"], ["ply", "PLY"]])),
    el("div", { class: "row" }, el("label", { title: "OBJ and PLY only" }, "vertex colours"), chk("colors")),
    el("div", { class: "row" }, el("label", { title: "Close the mesh where the object leaves the cube (for 3D printing)" }, "close at the cube"), chk("close")),
    el("div", { class: "row" }, el("label", { title: "Taubin smoothing passes (0 = off)" }, "smoothing passes"), inp("smooth")),
    el("div", { class: "multi" }, el("button", { class: "primary", id: "expMesh", onclick: () => exportStart("mesh") }, "Build mesh"))));
  p.append(el("div", { class: "card" }, el("h3", {}, "Progress"),
    el("div", { class: "bar", id: "expBar" }, el("div")),
    el("div", { class: "hint", id: "expState" }),
    el("div", { class: "hint mono", id: "expFolder" }),
    el("div", { class: "multi" },
      el("a", { id: "expFile", href: "/api/export/file", hidden: "" }, el("button", { class: "primary" }, "Download " + (exp.file ? `${exp.file.name} (${(exp.file.size / 1048576).toFixed(1)} MB)` : ""))),
      el("button", { onclick: async () => setExport(await post("/api/export/stop", "")) }, "Stop")),
    el("div", { class: "hint" }, "On the command line: mb3d voxel scene.m3p -o voxels · mb3d mesh scene.m3p -o mesh.stl")));
  exportUpdate();
}


// ------------------------------------------------------------ Monte Carlo
// MB3D's Monte Carlo renderer: ambient bounces, soft shadows, reflections
// and transparency, refined pass by pass where the image is noisy.
let mcState = null, mcVer = -1, mcPollTimer = null, mcWasRunning = false;
const mcUi = { rays: "64", scale: "1" };
const MC_DEF = { mc_depth: "3", mc_reflections: "false", mc_reflection_depth: "1", mc_reflection_amount: "0.5",
  mc_transparency: "false", mc_only_difs: "false", mc_diffuse_reflects: "0", mc_soft_shadow_radius: "1", mc_exposure: "128",
  mc_saturation: "32", mc_options: "2", mc_refraction_index: "1.5", mc_absorption: "0.2", mc_scattering: "1" };
function mcVal(k) { const l = model && model.global.find(x => x.k === k); return l ? l.v : MC_DEF[k]; }
function mcSet(o) {
  for (const [k, v] of Object.entries(o)) {
    const l = model.global.find(x => x.k === k);
    if (l) l.v = String(v); else model.global.push({ k, v: String(v), c: "" });
  }
  // the scene only keeps the mc keys when one differs from the default: write them all
  for (const k of Object.keys(MC_DEF)) if (!model.global.find(x => x.k === k)) model.global.push({ k, v: MC_DEF[k], c: "" });
  commit();
}
async function mcLoad() { try { setMc(await api("/api/mc")); } catch (e) { showError(e.message); } }
function setMc(m) {
  const first = !mcState;
  const changed = first || m.ver !== mcVer;
  mcState = m; mcVer = m.ver;
  if (isOpen("mc")) { if (first || mcWasRunning !== m.running) renderWin("mc"); else if (changed) mcUpdate(); else mcProgress(); }
  mcWasRunning = m.running;
  clearTimeout(mcPollTimer);
  if (m.running || isOpen("mc")) mcPollTimer = setTimeout(mcLoad, m.running ? 700 : 2500);
}
function mcProgress() {
  const m = mcState; if (!m) return;
  const b = $("mcBar"); if (b) b.firstElementChild.style.width = (m.running ? m.progress : (m.passes ? 100 : 0)) + "%";
}
function mcUpdate() {
  const m = mcState; if (!m) return;
  mcProgress();
  const s = $("mcState");
  if (s) {
    let t = m.passes || m.running ? `${m.w} × ${m.h} · pass ${m.passes + (m.running ? 1 : 0)} · ${m.rays.toFixed(1)} rays per pixel (max ${m.max_rays}) · noise ${m.noise.toFixed(4)} · ${fmtTime(m.seconds)}` : "";
    if (m.running) t += ` · until ${m.target} rays`;
    if (m.error) t += " · " + m.error;
    s.textContent = t;
  }
  const im = $("mcImg");
  if (im && (m.passes || m.rays > 0)) { im.src = "/api/mc/img?v=" + m.ver + "&t=" + Date.now(); im.hidden = false; }
  for (const id of ["mcDl", "mcM3c"]) { const x = $(id); if (x) x.hidden = !(m.passes || m.rays > 0); }
  const c = $("mcCont"); if (c) c.disabled = m.running || !m.passes;
}
function mcTab(p) {
  if (!mcState) { p.append(el("div", { class: "hint" }, "loading…")); mcLoad(); return; }
  const m = mcState;
  const inp = (k, title, size) => {
    const e = el("input", { type: "text", id: "mc_" + k, value: mcVal(k), title: title || "", size: size || 6 });
    e.addEventListener("change", () => mcSet({ [k]: e.value }));
    return e;
  };
  const chk = (k, title) => {
    const e = el("input", { type: "checkbox", id: "mc_" + k, title: title || "" });
    e.checked = mcVal(k) === "true";
    e.addEventListener("change", () => mcSet({ [k]: e.checked }));
    return e;
  };
  const opts = +mcVal("mc_options") || 0;
  const bit = (b, title) => {
    const e = el("input", { type: "checkbox", title: title || "" });
    e.checked = (opts & b) !== 0;
    e.addEventListener("change", () => mcSet({ mc_options: e.checked ? opts | b : opts & ~b }));
    return e;
  };
  const row = (label, ctl, title) => el("div", { class: "row" }, el("label", { title: title || "" }, label), ctl);
  const slider = (k, max, title) => {
    const e = el("input", { type: "range", min: 0, max, value: mcVal(k), title: title || "", id: "mc_" + k });
    const v = el("span", { class: "hint" }, mcVal(k));
    e.addEventListener("input", () => { v.textContent = e.value; });
    e.addEventListener("change", () => mcSet({ [k]: e.value }));
    return el("div", { class: "multi" }, e, v);
  };
  const bokeh = el("select", { id: "mc_bokeh" }, ...[1, 2, 3, 4, 5, 6].map(n => el("option", { value: n }, ["disc", "disc, bright rim", "pentagon", "pentagon, bright rim", "heptagon", "heptagon, bright rim"][n - 1])));
  bokeh.value = String(((opts >> 4) & 7) + 1);
  bokeh.onchange = () => mcSet({ mc_options: (opts & 0x0F) | ((+bokeh.value - 1) << 4) });
  p.append(el("div", { class: "card" }, el("h3", {}, "Light paths"),
    el("div", { class: "hint" }, "Path tracing with the scene's lights, colours and background. Ambient light comes from bounces of light between the surfaces, lights get a size and cast soft shadows."),
    row("ambient bounces", inp("mc_depth"), "MCDepth: diffuse bounces (1 = ambient occlusion only)"),
    row("light size", inp("mc_soft_shadow_radius"), "Size of the light sources (soft shadows)"),
    row("reflections", chk("mc_reflections")),
    row("reflection depth", inp("mc_reflection_depth"), "Reflections and refractions of reflections ..."),
    row("reflection amount", inp("mc_reflection_amount"), "Amount of reflected light (the palette's specular colours); 0..1 is realistic"),
    row("diffuse reflections", inp("mc_diffuse_reflects"), "Rough reflections: 0 (sharp) .. 2.5"),
    row("transparency", chk("mc_transparency", "With reflections: the alpha of the palette's specular colours is the transparency")),
    row("only dIFS transparent", chk("mc_only_difs")),
    row("refraction index", inp("mc_refraction_index")),
    row("absorption", inp("mc_absorption"), "Light absorption inside transparent material (coloured by the diffuse colour)"),
    row("light scattering", inp("mc_scattering"), "Light scattered inside transparent material"),
    row("secant search", bit(2, "Finer surface search")),
    row("clip spec. + diffuse", bit(4, "Scale specular and diffuse colours down when they add up to more than 1")),
    row("gaussian anti-aliasing", bit(8)),
    row("bokeh shape", bokeh, "Depth of field aperture shape (Post processing ▸ Depth of Field)")));
  p.append(el("div", { class: "card" }, el("h3", {}, "Image"),
    row("exposure", slider("mc_exposure", 255), "128 = 1"),
    row("colour saturation", slider("mc_saturation", 127), "32 = 1"),
    row("HDR soft clipping", bit(1)),
    el("div", { class: "hint" }, "Exposure, saturation and soft clipping change the image without recalculating it; gamma is the one of the Lighting window.")));
  const rays = el("input", { type: "text", id: "mcRays", value: mcUi.rays, size: 5 });
  rays.onchange = () => { mcUi.rays = rays.value; };
  const scale = el("select", { id: "mcScale" }, ...[["1", "full size"], ["0.5", "1/2"], ["0.25", "1/4"]].map(([v, l]) => el("option", { value: v }, l)));
  scale.value = mcUi.scale; scale.onchange = () => { mcUi.scale = scale.value; };
  const start = async cont => { try { setMc(await post("/api/mc/start", form({ rays: rays.value, scale: scale.value, cont }))); } catch (e) { showError(e.message); } };
  const fi = el("input", { type: "file", accept: ".m3c", hidden: "" });
  fi.onchange = async () => {
    const f = fi.files[0]; if (!f) return;
    try { applyState(await post("/api/mc/open?name=" + encodeURIComponent(f.name), await f.arrayBuffer())); mcLoad(); }
    catch (e) { showError(e.message); }
  };
  p.append(el("div", { class: "card" }, el("h3", {}, "Render"),
    el("div", { class: "multi" }, el("span", { class: "hint" }, "until"), rays, el("span", { class: "hint" }, "rays per pixel"), scale),
    el("div", { class: "multi" },
      el("button", { class: "primary", id: "mcStart", disabled: m.running ? "" : null, onclick: () => start(false) }, "Start"),
      el("button", { id: "mcCont", title: "More rays for the image below", onclick: () => start(true) }, "Continue"),
      el("button", { onclick: async () => setMc(await post("/api/mc/stop", "")) }, "Stop")),
    el("div", { class: "bar", id: "mcBar" }, el("div")),
    el("div", { class: "hint", id: "mcState" }),
    el("a", { href: "/api/mc/img", target: "_blank", title: "Full size" }, el("img", { id: "mcImg", alt: "", hidden: "" })),
    el("div", { class: "multi" },
      el("a", { id: "mcDl", href: "/api/mc/img", download: "mc.png", hidden: "" }, el("button", {}, "Save PNG")),
      el("a", { id: "mcM3c", href: "/api/mc/m3c", hidden: "" }, el("button", { title: "MB3D's Monte Carlo file: continue later, here or in MB3D" }, "Save .m3c")),
      el("button", { onclick: () => fi.click() }, "Open .m3c…"), fi),
    el("div", { class: "hint" }, "Start renders the editor's scene: the first pass shoots 4 rays per pixel, every further pass adds rays where the image is still noisy. On the command line: mb3d montecarlo scene.m3p --rays 64")));
  mcUpdate();
}

defWin("anim", "Animation maker", 440, p => animTab(p), { selfRefresh: true, onopen: () => animLoad() });
defWin("muta", "MutaGen", 440, p => mutaTab(p), { selfRefresh: true, onopen: () => mutaLoad() });
defWin("voxel", "Voxel export", 400, p => exportTab(p, "voxel"), { selfRefresh: true, onopen: () => exportLoad() });
defWin("btracer", "Bulb Tracer2", 400, p => exportTab(p, "mesh"), { selfRefresh: true, onopen: () => exportLoad() });
defWin("mc", "Monte carlo rendering", 440, p => mcTab(p), { selfRefresh: true, onopen: () => mcLoad(), sceneChanged: () => { if (mcState) renderWin("mc"); } });
