
// ------------------------------------------------------------ windows
// MB3D's separate windows (Formulas, Lighting, Post processing, Navigator,
// Animation maker, ...) as movable panels; positions are remembered.
const WINDOWS = {};
const openWins = new Set();
let zTop = 20;
function defWin(id, title, width, build, hooks = {}) { WINDOWS[id] = { title, width, build, ...hooks }; }
function raiseWin(w) {
  w.style.zIndex = ++zTop;
  for (const x of document.querySelectorAll(".win")) x.classList.toggle("top", x === w);
}
function openWin(id) {
  const def = WINDOWS[id];
  if (!def) return;
  let w = $("w_" + id);
  if (!w) {
    const n = Object.keys(WINDOWS).indexOf(id);
    const pos = store.get("win." + id, { x: 60 + (n % 8) * 34, y: 80 + (n % 8) * 26 });
    w = el("div", { class: "win", id: "w_" + id },
      el("div", { class: "wt" }, el("span", {}, def.title),
        SOLO ? null : el("button", { class: "pop", title: "Open in its own browser window (to put it next to the editor or on another screen)", onclick: () => popOut(id) }, "⧉"),
        el("button", { title: "Close", onclick: () => SOLO ? window.close() : closeWin(id) }, "✕")),
      el("div", { class: "wb" }));
    w.style.width = def.width + "px";
    w.style.left = Math.max(0, Math.min(innerWidth - 120, pos.x)) + "px";
    w.style.top = Math.max(0, Math.min(innerHeight - 60, pos.y)) + "px";
    w.addEventListener("mousedown", () => raiseWin(w), true);
    const bar = w.querySelector(".wt");
    bar.addEventListener("mousedown", e => {
      if (e.target.tagName === "BUTTON") return;
      const sx = e.clientX - w.offsetLeft, sy = e.clientY - w.offsetTop;
      const mv = ev => { w.style.left = Math.max(-w.offsetWidth + 80, ev.clientX - sx) + "px"; w.style.top = Math.max(0, ev.clientY - sy) + "px"; };
      const up = () => { removeEventListener("mousemove", mv); removeEventListener("mouseup", up); store.set("win." + id, { x: w.offsetLeft, y: w.offsetTop }); };
      addEventListener("mousemove", mv); addEventListener("mouseup", up);
      e.preventDefault();
    });
    $("wins").append(w);
  }
  w.hidden = false;
  openWins.add(id);
  raiseWin(w);
  renderWin(id);
  if (def.onopen) def.onopen();
  if (!SOLO) store.set("open", [...openWins]);
}
function closeWin(id) {
  const w = $("w_" + id);
  if (w) w.hidden = true;
  openWins.delete(id);
  if (WINDOWS[id].onclose) WINDOWS[id].onclose();
  if (!SOLO) store.set("open", [...openWins]);
}
const isOpen = id => openWins.has(id);
function popOut(id) {
  const def = WINDOWS[id];
  const sz = store.get("pop." + id, { w: def.width + 24, h: Math.min(screen.availHeight - 80, 820) });
  const w = window.open("/?win=" + id, "mb3d_" + id, `popup,width=${sz.w},height=${sz.h}`);
  if (!w) return showError("The browser blocked the new window: allow pop-ups for this page.");
  closeWin(id);
}
function renderWin(id) {
  const w = $("w_" + id);
  if (w && !w.hidden) rerender(w.querySelector(".wb"), WINDOWS[id].build);
}
function refreshAll() {
  renderTop();
  renderRight();
  for (const id of openWins) if (!WINDOWS[id].selfRefresh) renderWin(id);
  for (const id of openWins) if (WINDOWS[id].sceneChanged) WINDOWS[id].sceneChanged();
}
for (const b of document.querySelectorAll("[data-win]")) b.addEventListener("click", () => isOpen(b.dataset.win) && !$("w_" + b.dataset.win).classList.contains("top") ? raiseWin($("w_" + b.dataset.win)) : (isOpen(b.dataset.win) ? closeWin(b.dataset.win) : openWin(b.dataset.win)));

// ------------------------------------------------------------ top bar: Open / Save / Save pic / Tools / Prefs
let pc2 = store.get("pc2", "Open");
function openFile(accept) {
  const f = $("fileIn");
  f.accept = accept;
  f.value = "";
  f.onchange = async () => {
    const file = f.files[0];
    if (!file) return;
    try {
      undoStack = []; redoStack = [];
      applyState(await post("/api/open?name=" + encodeURIComponent(file.name), await file.arrayBuffer()), true);
      msg(`Parameters loaded from ${file.name}, press "Calculate 3D" to render.`);
      for (const n of notes) msg(n, "n");
    } catch (e) { showError(e); }
  };
  f.click();
}
async function fromClipboard() {
  let t = "";
  try { t = await navigator.clipboard.readText(); } catch (e) { t = ""; }
  if (t && /Mandelbulb3D|\[formula\]|^\s*width\s*=/m.test(t)) return loadText(t);
  $("pasteText").value = t;
  $("dPaste").showModal();
}
async function loadText(t) {
  try { applyState(await post("/api/open?name=clipboard.txt", t)); msg("Parameters loaded from the clipboard."); for (const n of notes) msg(n, "n"); return true; }
  catch (e) { showError(e); return false; }
}
$("pasteCancel").onclick = () => $("dPaste").close();
$("pasteOk").onclick = async () => { if (await loadText($("pasteText").value)) $("dPaste").close(); };
async function toClipboard() {
  try {
    const r = await fetch("/api/save?fmt=txt");
    const t = await r.text();
    await navigator.clipboard.writeText(t);
    msg("Parameters copied to the clipboard (MB3D text parameters).");
  } catch (e) { showError("clipboard not available: " + e.message); }
}
const viewN = () => Math.max(1, +$("viewScale").value || 1);
async function savePic(kind) {
  if (lastFinal <= 0 && kind !== "zbuf") return showError('Calculate the image first ("Calculate 3D").');
  if (kind === "png") { location.href = "/api/pic?scale=" + viewN(); return; }
  if (kind === "zbuf") { location.href = "/api/zbuf.png"; return; }
  try {
    const blob = await (await fetch("/api/pic?scale=" + viewN())).blob();
    const bmp = await createImageBitmap(blob);
    const c = el("canvas"); c.width = bmp.width; c.height = bmp.height;
    c.getContext("2d").drawImage(bmp, 0, 0);
    const q = Math.min(100, Math.max(10, +$("jpgQ").value || 95)) / 100;
    c.toBlob(b => { const a = el("a", { href: URL.createObjectURL(b), download: (title || "mb3d") + ".jpg" }); a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 5000); }, "image/jpeg", q);
  } catch (e) { showError(e); }
}
function renderTop() {
  const box = $("pc2");
  const tabs = box.querySelector(".tabs"), page = box.querySelector(".page");
  tabs.replaceWith(tabsRow(["Open", "Save", "Save pic", "Tools", "Prefs"], pc2, n => { pc2 = n; store.set("pc2", n); renderTop(); }, "tabs"));
  const pg = el("div", { class: "page" });
  if (pc2 === "Open") {
    const preset = select("preset", "", [["", "New…"], ...formulaNames.slice(0, 8).map(n => [n, n])], async n => {
      if (!n) return;
      try { applyState(await post("/api/preset", form({ name: n }))); msg(`New parameters: ${n}`); } catch (e) { showError(e); }
    }, { title: "New parameters from a built-in formula" });
    pg.append(btn("Open m3i", () => openFile(".m3i"), { title: "Open a MB3D image file (parameters)" }),
      btn("Open m3p", () => openFile(".m3p,.m3s,.txt"), { title: "Open a MB3D parameter file (.m3p), a scene (.m3s) or text parameters" }),
      btn("From Clipboard", fromClipboard, { title: "Paste MB3D text parameters" }), preset);
  } else if (pc2 === "Save") {
    pg.append(btn("m3i", () => location.href = "/api/m3i", { title: "Image file with the calculated G-buffer (opens in MB3D without recalculating)" }),
      btn("m3p", () => location.href = "/api/save?fmt=m3p", { title: "MB3D parameter file" }),
      btn("m3s", () => location.href = "/api/save?fmt=m3s", { title: "Scene text file of this port" }),
      btn("To Clipboard", toClipboard, { title: "Copy the parameters as MB3D text" }),
      el("label", { class: "dim" }, "Title:"), field("titleIn", title, v => { post("/api/title", v); title = v; document.title = `Mandelbulb 3D — ${v}`; }, { style: "width:120px", title: "Name for saving" }));
  } else if (pc2 === "Save pic") {
    pg.append(btn("PNG", () => savePic("png"), { title: "Save the calculated image, reduced by the viewing scale (1:2, 1:3 anti-aliased)" }),
      btn("JPEG", () => savePic("jpg")), btn("ZBUF", () => savePic("zbuf"), { title: "16 bit PNG of the z-buffer" }),
      el("label", { class: "dim" }, "jpg q.:"), el("input", { type: "text", id: "jpgQ", value: store.get("jpgQ", 95), style: "width:36px", onchange: e => store.set("jpgQ", e.target.value) }));
  } else if (pc2 === "Tools") {
    pg.append(btn("m3p->m3i", () => openWin("batch"), { title: "Batch processing: calculate many parameter files" }),
      btn("Voxelstack", () => openWin("voxel"), { title: "Voxel export (PNG slices)" }),
      btn("Big render", () => openWin("tiling"), { title: "Big renders in tiles" }),
      btn("M.C.", () => openWin("mc"), { title: "Monte Carlo rendering" }),
      btn("Text", () => openWin("text"), { title: "All parameters as text (.m3s)" }));
  } else {
    pg.append(btn("Ini Dirs", () => openWin("dirs"), { title: "Formula and map folders" }),
      btn("Map Sequences", () => openWin("mapseq"), { title: "Animated maps: image sequences for the map numbers" }),
      btn("Themes", () => { const c = document.documentElement.classList.toggle("classic"); store.set("classic", c); }, { title: "Switch between the dark and the classic light theme" }));
  }
  page.replaceWith(pg);
  // the image group
  for (const [id, k] of [["iW", "width"], ["iH", "height"]]) if (document.activeElement !== $(id)) $(id).value = gv(k);
}
$("iW").onchange = () => setSize(+$("iW").value, null);
$("iH").onchange = () => setSize(null, +$("iH").value);
function setSize(w, h) {
  const ow = gnum("width", 640), oh = gnum("height", 480);
  if (w && !h) h = $("keepAsp").checked ? Math.round(w * oh / ow) : oh;
  if (h && !w) w = $("keepAsp").checked ? Math.round(h * ow / oh) : ow;
  if (!(w >= 16 && h >= 16)) return refreshAll();
  setG({ width: Math.round(w), height: Math.round(h), tile: null, tiles: null });
}
const aspect = r => setSize(gnum("width"), Math.round(gnum("width") / r));
$("a43").onclick = () => aspect(4 / 3);
$("a53").onclick = () => aspect(5 / 3);
$("aUser").onclick = () => {
  const r = prompt("Aspect ratio (width : height), e.g. 16:9 or 1.5", store.get("userAsp", "16:9"));
  if (!r) return;
  store.set("userAsp", r);
  const m = r.split(/[:/x]/).map(Number);
  const v = m.length > 1 ? m[0] / m[1] : m[0];
  if (v > 0) aspect(v);
};
$("sUp").onclick = () => nav({ op: "scale", f: 2, destop: $("desC").checked ? 1 : 0 });
$("sDn").onclick = () => nav({ op: "scale", f: 0.5, destop: $("desC").checked ? 1 : 0 });
$("keepAsp").checked = store.get("keepAsp", true);
$("keepAsp").onchange = () => store.set("keepAsp", $("keepAsp").checked);
$("desC").checked = store.get("desC", true);
$("desC").onchange = () => store.set("desC", $("desC").checked);

// ------------------------------------------------------------ viewing scale and the preview size
$("viewScale").value = String(store.get("viewScale", 0));
function applyViewing() {
  const n = +$("viewScale").value;
  const v = $("view"), img = $("img");
  v.classList.toggle("fit", n === 0);
  if (n === 0) { img.style.width = img.style.height = ""; }
  else { img.style.width = Math.round(gnum("width") / n) + "px"; img.style.height = Math.round(gnum("height") / n) + "px"; }
}
$("viewScale").onchange = () => { store.set("viewScale", +$("viewScale").value); applyViewing(); sendView(true); };
function viewWidth() {
  const n = +$("viewScale").value;
  const W = gnum("width", 640), H = gnum("height", 480);
  const v = $("view");
  const w = n === 0 ? Math.min(v.clientWidth, v.clientHeight * W / H) : W / n;
  return Math.max(64, Math.min(1600, W, Math.round(w)));
}
let sentView = 0;
async function sendView(force) {
  if (SOLO) return;
  applyViewing();
  const w = viewWidth();
  if (!force && Math.abs(w - sentView) < 24) return;
  sentView = w;
  try { await post("/api/view", form({ w })); pollSoon(); } catch (e) { showError(e); }
}
let resizeTimer = null;
addEventListener("resize", () => { clearTimeout(resizeTimer); resizeTimer = setTimeout(() => sendView(false), 400); });

// ------------------------------------------------------------ image: mouse modes
let mode = "walk";
for (const b of document.querySelectorAll("[data-mode]")) b.addEventListener("click", () => {
  mode = b.dataset.mode;
  for (const x of document.querySelectorAll("[data-mode]")) x.classList.toggle("down", x === b);
  $("view").className = $("view").className.replace(/\bm-\w+/g, "").trim() + " m-" + mode;
});
const img = $("img"), view = $("view");
function imgRect() {
  const b = img.getBoundingClientRect();
  const nw = img.naturalWidth || 1, nh = img.naturalHeight || 1;
  const s = Math.min(b.width / nw, b.height / nh);
  return { left: b.left + (b.width - nw * s) / 2, top: b.top + (b.height - nh * s) / 2, width: nw * s, height: nh * s };
}
const uvOf = (e, r) => ({ u: (e.clientX - r.left) / r.width, v: (e.clientY - r.top) / r.height });
let drag = null;
img.addEventListener("contextmenu", e => e.preventDefault());
img.addEventListener("mousedown", e => {
  view.focus();
  e.preventDefault();
  const r = imgRect();
  drag = { x: e.clientX, y: e.clientY, r, button: e.button, maxw: 0 };
});
addEventListener("mousemove", e => {
  if (!drag) return;
  const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
  const vb = view.getBoundingClientRect();
  if ((mode === "zoom" || recalcOn) && !pickCb && drag.button === 0) {
    const w = Math.abs(dx), h = w * drag.r.height / drag.r.width;
    drag.maxw = Math.max(drag.maxw, w);
    const s = $("sel");
    s.style.display = w > 3 ? "block" : "none";
    s.style.left = (Math.min(drag.x, e.clientX) - vb.left + view.scrollLeft) + "px";
    s.style.top = ((dy > 0 ? drag.y : drag.y - h) - vb.top + view.scrollTop) + "px";
    s.style.width = w + "px"; s.style.height = h + "px";
    drag.sel = { cx: Math.min(drag.x, e.clientX) + w / 2, cy: (dy > 0 ? drag.y + h / 2 : drag.y - h / 2), w };
  } else if (mode === "xy" && !pickCb) {
    img.style.transform = `translate(${dx}px, ${dy}px)`;
  } else if (mode === "z" && !pickCb) {
    $("hint").style.display = "block"; $("hint").textContent = `Z: ${-dy}`;
  }
});
addEventListener("mouseup", async e => {
  if (!drag) return;
  const d = drag; drag = null;
  const dx = e.clientX - d.x, dy = e.clientY - d.y, r = d.r;
  const W = gnum("width"), H = gnum("height");
  const k = W / r.width; // screen px -> image px
  if (!recalcOn) $("sel").style.display = "none";
  img.style.transform = "";
  if (recalcOn && !pickCb) {
    // "Recalculate a selection": the marked rectangle stays visible
    if (d.sel && d.sel.w >= 4) {
      const h = d.sel.w * r.height / r.width;
      recalcSel = { u0: (d.sel.cx - d.sel.w / 2 - r.left) / r.width, v0: (d.sel.cy - h / 2 - r.top) / r.height,
        u1: (d.sel.cx + d.sel.w / 2 - r.left) / r.width, v1: (d.sel.cy + h / 2 - r.top) / r.height };
      $("sel").style.display = "block";
      renderWin("postpro");
    }
    return;
  }
  if (pickCb) {
    const { u, v } = uvOf(e, r);
    const cb = pickCb; endPick();
    if (u < 0 || u > 1 || v < 0 || v > 1) return;
    try { await cb(u, v); } catch (x) { showError(x); }
    return;
  }
  if (mode === "zoom") {
    if (d.sel && d.sel.w >= 8) {
      nav({ op: "pan", dx: (d.sel.cx - r.left) * k - W / 2, dy: (d.sel.cy - r.top) * k - H / 2, dz: r.width / d.sel.w });
    } else if (d.maxw < 8) {
      const { u, v } = uvOf(e, r);
      nav({ op: "pan", dx: (u - 0.5) * W, dy: (v - 0.5) * H, dz: d.button === 2 ? 1 / 1.4 : 1.4 });
    }
  } else if (mode === "xy") {
    if (Math.hypot(dx, dy) > 2 && d.button === 0) nav({ op: "pan", dx: -dx * k, dy: -dy * k });
  } else if (mode === "z") {
    $("hint").style.display = "none";
    if (Math.abs(dy) > 1) nav({ op: "pan", zt: -dy * k });
  } else {
    // walk (navigator)
    if (Math.hypot(dx, dy) > 4) {
      const kk = gnum("fov", 30) / r.height;
      if (Math.abs(dx) > 2) nav({ op: "rotate", axis: 1, deg: -dx * kk });
      if (Math.abs(dy) > 2) nav({ op: "rotate", axis: 0, deg: dy * kk });
    } else {
      const { u, v } = uvOf(e, r);
      if (u < 0 || u > 1 || v < 0 || v > 1) return;
      const m = e.shiftKey ? "look" : naviCfg.click;
      nav({ op: m, u, v, amount: stepFrac(e.altKey) });
    }
  }
});
let wheelAcc = 0, wheelTimer = null;
view.addEventListener("wheel", e => {
  if (mode !== "walk" || +$("viewScale").value !== 0 && !e.ctrlKey && view.scrollHeight > view.clientHeight + 2) return;
  e.preventDefault();
  wheelAcc += e.deltaY < 0 ? 1 : -1;
  clearTimeout(wheelTimer);
  wheelTimer = setTimeout(() => { const n = wheelAcc; wheelAcc = 0; nav({ op: "move", axis: 2, amount: n * stepFrac(e.shiftKey) * 0.5 }); }, 120);
}, { passive: false });

// ------------------------------------------------------------ keyboard (navigator keys)
const naviCfg = Object.assign({ step: 50, angle: 10, click: "fly", adj: 10 }, store.get("navi", {}));
const saveNavi = () => store.set("navi", naviCfg);
function stepFrac(fine) { return (+naviCfg.step || 50) / 100 * (fine ? 0.25 : 1); }
function angleDeg(fine) { return (+naviCfg.angle || 10) * (fine ? 0.25 : 1); }
function doNav(op, axis, sign, fine) {
  if (op === "move") nav({ op, axis, amount: sign * stepFrac(fine) });
  else if (op === "rotate") nav({ op, axis, deg: sign * angleDeg(fine) });
  else if (op === "zoom") nav({ op, factor: sign > 0 ? (fine ? 1.1 : 1.5) : (fine ? 1 / 1.1 : 1 / 1.5) });
}
const KEYS = { w: ["move", 2, 1], s: ["move", 2, -1], a: ["move", 0, -1], d: ["move", 0, 1], r: ["move", 1, -1], f: ["move", 1, 1],
  arrowleft: ["rotate", 1, -1], arrowright: ["rotate", 1, 1], arrowup: ["rotate", 0, 1], arrowdown: ["rotate", 0, -1],
  q: ["rotate", 2, -1], e: ["rotate", 2, 1], "+": ["zoom", 0, 1], "=": ["zoom", 0, 1], "-": ["zoom", 0, -1] };
document.addEventListener("keydown", e => {
  const t = e.target, typing = ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName);
  if (e.key === "Escape" && pickCb) { endPick(); return; }
  if ((e.ctrlKey || e.metaKey) && !typing) {
    if (e.key === "z") { e.preventDefault(); undo(); return; }
    if (e.key === "y") { e.preventDefault(); redo(); return; }
  }
  if (typing || e.ctrlKey || e.metaKey || e.altKey) return;
  if (t !== view && !(isOpen("navi") && $("w_navi").contains(t))) return;
  const k = KEYS[e.key.toLowerCase()];
  if (k) { e.preventDefault(); doNav(k[0], k[1], k[2], e.shiftKey); }
});

// ------------------------------------------------------------ bottom bar: rotation buttons
for (const b of document.querySelectorAll("[data-rot]")) {
  const [axis, sign] = b.dataset.rot.split(",").map(Number);
  b.addEventListener("contextmenu", e => e.preventDefault());
  b.addEventListener("mouseup", e => nav({ op: "rotmid", axis, deg: sign * (parseFloat($("rotDeg").value) || 5), obj: e.button === 2 ? 1 : 0 }));
}

// ------------------------------------------------------------ right column
const folds = store.get("folds", {});
for (const b of document.querySelectorAll("[data-fold]")) {
  const p = $(b.dataset.fold);
  const set = () => { p.hidden = !!folds[b.dataset.fold]; b.classList.toggle("closed", p.hidden); };
  set();
  b.onclick = () => { folds[b.dataset.fold] = !folds[b.dataset.fold]; store.set("folds", folds); set(); };
}
$("bUndo").onclick = undo;
$("bUndo").addEventListener("contextmenu", e => { e.preventDefault(); redo(); });
$("bRedo").onclick = redo;
$("bCalcMinus").onclick = async () => { await post("/api/refresh", ""); pollSoon(); };
async function calc3D() {
  await navChain;
  try { await post("/api/render", form({ aa: 1 })); msg("Calculating the image…", "n"); pollSoon(); } catch (e) { showError(e); }
}
$("bCalc3D").onclick = calc3D;
// MB3D's quick quality presets (AccPreset)
const ACC = [[0, 1.0, 0.5, 6, 480, 1, 1], [1, 1.0, 0.4, 8, 640, 1, 0.75], [2, 1.2, 0.3, 10, 1600, 2, 0.5], [3, 1.2, 0.25, 12, 3072, 3, 0.3]];
for (const b of document.querySelectorAll("[data-q]")) b.onclick = () => {
  const [sn, de, rm, bs, iw, sc, rl] = ACC[+b.dataset.q];
  const w = gnum("width", 640), h = gnum("height", 480);
  $("viewScale").value = String(sc); store.set("viewScale", sc);
  setG({ smooth_normals: sn, de_stop: de, raystep: rm, bin_search: bs, raystep_limiter: rl, width: iw, height: Math.round(h * iw / w), tiles: null, tile: null });
};

function renderRight() {
  rerender($("posPnl"), p => {
    p.append(lab("X mid:"), fVec("pX", "mid", 0, 3), lab("Y mid:"), fVec("pY", "mid", 1, 3));
    const sl = (t, at, tip) => btn(t, async () => { await post("/api/slice", form({ at })); pollSoon(); }, { class: "lab", title: tip });
    p.append(sl("Z mid", 2, "Quick 2D calculation of the plane at the middle"), fVec("pZ", "mid", 2, 3));
    p.append(sl("Z start", 1, "Quick 2D calculation of the plane at z start (the camera)"), fG("pZs", "z_start"));
    p.append(sl("Z end", 3, "Quick 2D calculation of the plane at z end"), fG("pZe", "z_end"));
    p.append(el("div", { class: "full row2" },
      btn("get midpoint", () => pickFromImage("Get midpoint", (u, v) => nav({ op: "midpoint", u, v })), { title: "Click on the object to make that point the middle" }),
      btn("reset", () => nav({ op: "reset_pos" }), { title: "Reset position, zoom and rotation" })));
    p.append(lab("Zoom:"), fG("pZoom", "zoom"));
  });
  rerender($("rotPnl"), p => {
    const e = euler || [];
    const f = (id, i) => el("input", { type: "text", id, value: e.length ? +(+e[i]).toFixed(6) : "?" });
    p.append(el("div", { class: "full dim" }, "Euler angles:"), lab("X:"), f("eX", 0), lab("Y:"), f("eY", 1), lab("Z:"), f("eZ", 2),
      el("div", { class: "full hint" }, "Apply your changed values, the fields only show the angles."),
      el("div", { class: "full row2" }, btn("Apply to image", () => nav({ op: "euler", x: parseFloat($("eX").value) || 0, y: parseFloat($("eY").value) || 0, z: parseFloat($("eZ").value) || 0 })),
        btn("Reset 0", () => nav({ op: "euler", x: 0, y: 0, z: 0 }))));
  });
  const box = $("rtabs");
  box.textContent = "";
  for (const n of Object.keys(RTABS)) box.append(el("button", { class: n === rtab ? "on" : "", onclick: () => { rtab = n; store.set("rtab", n); renderRight(); } }, n));
  rerender($("rpage"), p => RTABS[rtab](p));
}

// the PageControl of the main window
let rtab = store.get("rtab", "Calculation");
const cutMem = store.get("cutMem", {});
const RTABS = {
  "Calculation": p => p.append(el("div", { class: "pnl" },
    lab("Smooth normals:", "0..8: normals from the neighbours"), fG("cSN", "smooth_normals"),
    lab("DE stop:", "Surface detail: distance estimate where the ray stops, in pixels"), fG("cDE", "de_stop"),
    lab("Raystep multiplier:"), fG("cRM", "raystep"),
    lab("Stepcount for binary search:"), fG("cBS", "bin_search"),
    lab("Stepwidth limiter:"), fG("cRL", "raystep_limiter"),
    el("div", { class: "full" }, cG("cVD", "vary_de_stop", "Vary DEstop on FOV")),
    el("div", { class: "full" }, cG("cND", "normals_on_de", "Normals on DE")),
    el("div", { class: "full" }, cG("cFR", "first_step_random", "First step random")),
    el("div", { class: "full" }, el("label", { class: "dim", title: "Not ported" }, el("input", { type: "checkbox", disabled: true }), "Shortdistance check DE")),
    el("div", { class: "full" }, cG("cSS", "step_sub_de_stop", "Raystep sub DEstop")))),
  "Internal": p => p.append(el("div", { class: "pnl" },
    lab("Threadcount in calculations:", "0 = all cores of the server"), fG("iThr", "threads"),
    el("div", { class: "full hint" }, "0 uses all cores of the machine that runs mb3d gui."))),
  "Infos": p => {
    const i = lastInfo || {};
    const t = v => v === undefined ? "–" : v < 1 ? (v * 1000).toFixed(0) + " ms" : v.toFixed(2) + " s";
    p.append(el("div", { class: "pnl" },
      lab("Image:"), el("span", {}, i.w ? `${i.w} × ${i.h}${i.full ? "" : " (preview)"}` : "–"),
      lab("Avg. raysteps:"), el("span", {}, i.steps !== undefined ? i.steps : "–"),
      lab("Object pixels:"), el("span", {}, i.hits !== undefined ? i.hits + " %" : "–"),
      lab("Main calc time:"), el("span", {}, t(i.calc)),
      lab("HS+AO time:", "Post calculations: normals on the z-buffer, hard shadows, ambient shadows"), el("span", {}, t(i.post)),
      lab("Paint time:", "Painting incl. reflections and depth of field"), el("span", {}, t(i.paint)),
      lab("Max. iterations:"), el("span", {}, gv("iterations"))));
    if (notes.length) p.append(el("div", { class: "sect" }, "Notes from loading"), ...notes.map(n => el("div", { class: "hint" }, n)));
  },
  "Cutting": p => {
    const row = (axis, i) => {
      const k = "cut_" + axis, on = gv(k) !== "off";
      const v = on ? gv(k) : (cutMem[k] ?? "0");
      return [lab(axis.toUpperCase() + ":"), el("div", { class: "row2" },
        field("ct" + axis, v, x => { cutMem[k] = x; store.set("cutMem", cutMem); if (on) setG({ [k]: x }); }),
        check("cc" + axis, on, c => { cutMem[k] = $("ct" + axis).value; store.set("cutMem", cutMem); setG({ [k]: c ? $("ct" + axis).value || "0" : "off" }); }))];
    };
    p.append(el("div", { class: "pnl" }, ...row("z"), ...row("x"), ...row("y"),
      el("div", { class: "full hint" }, "The side away from the camera is kept."),
      el("div", { class: "full" }, btn("Insert mid values", () => { const m = gvec("mid", 3); setG({ cut_x: m[0], cut_y: m[1], cut_z: m[2] }); })),
      el("div", { class: "full" }, btn("Get values from image", () => pickFromImage("Cutting position", async (u, v) => {
        const r = await pickAt(u, v); if (!r.pos) throw new Error("background picked");
        setG({ cut_x: r.pos[0], cut_y: r.pos[1], cut_z: r.pos[2] });
      })))));
  },
  "Julia Off": p => p.append(el("div", { class: "pnl" },
    el("div", { class: "full" }, cG("jOn", "julia", "Calculate Julia")),
    lab("X:"), fVec("jX", "julia_c", 0, 4), lab("Y:"), fVec("jY", "julia_c", 1, 4), lab("Z:"), fVec("jZ", "julia_c", 2, 4), lab("W:"), fVec("jW", "julia_c", 3, 4),
    el("div", { class: "full" }, btn("Insert mid values", () => { const m = gvec("mid", 3); setG({ julia_c: `${m[0]}, ${m[1]}, ${m[2]}, ${gvec("julia_c", 4)[3]}`, julia: true }); })),
    el("div", { class: "full" }, btn("Get values from image", () => pickFromImage("Julia values", async (u, v) => {
      const r = await pickAt(u, v); if (!r.pos) throw new Error("background picked");
      setG({ julia_c: `${r.pos[0]}, ${r.pos[1]}, ${r.pos[2]}, 0`, julia: true });
    }))))),
  "Camera": p => p.append(el("div", { class: "pnl" },
    lab("FOVy:", "Field of view in degrees"), fG("caF", "fov"),
    el("div", { class: "full dim" }, "Camera lense:"),
    el("div", { class: "full" }, radios("optic", gv("optic", "0"), [["0", "Common"], ["1", "Rectilinear"], ["2", "360 Panorama"]], v => setG({ optic: v }))))),
  "Coloring": p => {
    const vol = gv("vol_light") !== "off";
    p.append(el("div", { class: "pnl" },
      lab("Lli multiplier:", "Multiplier for the 'Last length increase' option"), fG("coM", "color_mul"),
      lab("Color on Iterat.:", "Colouring after this number of iterations instead of at the bailout (-1 off)"), fG("coI", "color_on_iteration"),
      el("div", { class: "full dim" }, "Mode for 2. color choice:"),
      el("div", { class: "full" }, radios("copt", gv("color_option", "0"), [["0", "Orbit trap (mindist. to 0)"], ["1", "Last length increase"], ["2", "Rout angle of X, Y"],
        ["3", "Rout angle of X, Z"], ["4", "Rout angle of Y, Z"], ["5", "Map on output vector"]], v => setG({ color_option: v }))),
      btn(vol ? "Volume light nr:" : "Dyn. fog on It.:", () => setG(vol ? { vol_light: "off" } : { vol_light: 1 }), { class: "lab", title: "Switch between dynamic fog on iterations and volumetric light of a light" }),
      vol ? fG("coV", "vol_light") : fG("coD", "dfog_on_it"),
      vol ? lab("Map size:", "Size of the volumetric light map in 20 % steps") : null, vol ? fG("coVS", "vol_light_map_size") : null));
  },
  "Stereo": p => {
    const eye = (m, t) => btn(t, async () => { await setG({ stereo: m }); calc3D(); });
    p.append(el("div", { class: "pnl" },
      lab("Image width:", "Width of the screen the image is shown on"), fVec("sW", "stereo_screen", 0, 3),
      lab("Screen distance:"), fVec("sD", "stereo_screen", 1, 3),
      lab("Minimal distance:", "Distance of the nearest object part"), fVec("sM", "stereo_screen", 2, 3),
      el("div", { class: "full hint" }, "All realworld units in meter."),
      lab("Eye:"), select("sEye", gv("stereo"), ["off", "left", "right", "very_left"], v => setG({ stereo: v })),
      el("div", { class: "full" }, eye("very_left", "Calc very left from midpos")),
      el("div", { class: "full" }, eye("left", "Calculate left eye image")),
      el("div", { class: "full" }, eye("right", "Calculate right eye image"))));
  },
};
