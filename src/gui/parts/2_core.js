<script>
"use strict";
const $ = id => document.getElementById(id);
const el = (tag, attrs = {}, ...kids) => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v === undefined || v === null || v === false) continue;
    if (k === "class") e.className = v;
    else if (k === "style") e.style.cssText = v;
    else if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
    else if (v === true) e.setAttribute(k, "");
    else e.setAttribute(k, v);
  }
  for (const k of kids) if (k !== null && k !== undefined && k !== false) e.append(k);
  return e;
};
const store = {
  get(k, d) { try { const v = localStorage.getItem("mb3d." + k); return v === null ? d : JSON.parse(v); } catch (e) { return d; } },
  set(k, v) { try { localStorage.setItem("mb3d." + k, JSON.stringify(v)); } catch (e) { /* private mode */ } },
};

// ------------------------------------------------------------ scene model
// The scene is .m3s text: global "key = value" lines, then [formula] and
// [light] sections.  Every control edits this list and sends the whole text
// back; the server answers with the normalised text.
let model = { global: [], sections: [] };
// a window popped out into its own browser window: index.html?win=<id>
const SOLO = new URLSearchParams(location.search).get("win");
const chan = "BroadcastChannel" in window ? new BroadcastChannel("mb3d-editor") : null;
let sceneHash = "", stateFetch = null;
let sceneText = "", euler = null, notes = [];
let formulaNames = [], formulaDE = new Map();
let undoStack = [], redoStack = [];

function parseM3s(text) {
  const m = { global: [], sections: [] };
  let cur = m.global;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith("#") || line.startsWith("//")) continue;
    const sec = line.match(/^\[(\w+)\]$/);
    if (sec) { const s = { type: sec[1].toLowerCase(), lines: [] }; m.sections.push(s); cur = s.lines; continue; }
    const semi = line.indexOf(";");
    const body = semi >= 0 ? line.slice(0, semi) : line;
    const comment = semi >= 0 ? line.slice(semi + 1).trim() : "";
    const eq = body.indexOf("=");
    if (eq < 0) continue;
    cur.push({ k: body.slice(0, eq).trim(), v: body.slice(eq + 1).trim(), c: comment });
  }
  return m;
}
const lineText = l => `${l.k} = ${l.v}` + (l.c ? ` ; ${l.c}` : "");
function toText(m) {
  let t = "# mb3d scene\n" + m.global.map(lineText).join("\n") + "\n";
  for (const s of m.sections) t += `\n[${s.type}]\n` + s.lines.map(lineText).join("\n") + "\n";
  return t;
}

// defaults of keys the scene text leaves out
const DEF = {
  cut_x: "off", cut_y: "off", cut_z: "off", inside: "outside", interpolation: "off", de_combination: "off",
  decomb_end1: "1", decomb_start2: "2", decomb_end2: "6", decomb_repeat2: "2", decomb_iterations2: "60", decomb_smooth: "0.5", decomb_mix_pow: "1",
  slice_2d: "off", stereo: "off", stereo_screen: "1, 2, 1.5", color_on_iteration: "-1", shadows: "off", shadow_soft: "false",
  shadow_soft_radius: "1", shadow_max_len: "1", shadow_set_cos: "true", ao_random: "0", deao_quality: "1", deao_dither: "0", deao_max_len: "1",
  dof: "off", dof_focus: "0", dof_focus2: "0", dof_aperture: "0.02", dof_max_radius: "25", dof_passes: "1", vol_light: "off", vol_light_map_size: "0",
  normals_on_zbuf: "false", internal_gamma2: "false", color_interpolation: "true", ambient_relative_to_object: "false", dynfog_options: "0",
  light_mode: "0", diffuse_map: "0", diffuse_map_mode: "iterations_otrap", diffuse_map_offset: "128, 128", diffuse_map_rotation: "128",
  diffuse_map_scale: "30", diffuse_map_brightness_only: "false", background_image: "", background_rotation: "128, 128, 128",
  background_direct: "false", background_brightness: "40", background_add_light: "false", background_ambient: "false", rstop: "16",
  mc_depth: "3", mc_reflections: "false", mc_reflection_depth: "1", mc_reflection_amount: "0.5", mc_transparency: "false", mc_only_difs: "false",
  mc_diffuse_reflects: "0", mc_soft_shadow_radius: "1", mc_exposure: "128", mc_saturation: "32", mc_options: "2", mc_refraction_index: "1.5",
  mc_absorption: "0.2", mc_scattering: "1", repeat_from: "0", tiles: "", tile_downscale: "1",
};
function gv(k, d) { const l = model.global.find(x => x.k === k); return l ? l.v : (k in DEF ? DEF[k] : (d ?? "")); }
function gnum(k, d = 0) { const v = parseFloat(gv(k)); return isFinite(v) ? v : d; }
function gvec(k, n) { const p = gv(k).split(",").map(s => s.trim()); while (p.length < n) p.push("0"); return p; }
function gbool(k) { return gv(k) === "true"; }
function putG(o) {
  for (const [k, v] of Object.entries(o)) {
    const i = model.global.findIndex(x => x.k === k);
    if (v === null || v === undefined) { if (i >= 0) model.global.splice(i, 1); }
    else if (i >= 0) model.global[i].v = String(v);
    else model.global.push({ k, v: String(v), c: "" });
  }
}
function setG(o) { putG(o); return commit(); }
const formulaSecs = () => model.sections.filter(s => s.type === "formula");
const lightSecs = () => model.sections.filter(s => s.type === "light");
const sv = (sec, k, d = "") => { const l = sec && sec.lines.find(x => x.k === k); return l ? l.v : d; };
function putS(sec, o) {
  for (const [k, v] of Object.entries(o)) {
    const i = sec.lines.findIndex(x => x.k === k);
    if (v === null || v === undefined) { if (i >= 0) sec.lines.splice(i, 1); }
    else if (i >= 0) sec.lines[i].v = String(v);
    else sec.lines.push({ k, v: String(v), c: "" });
  }
}
function lightSec(slot) { return lightSecs().find(s => +sv(s, "slot", "0") === slot); }

// ------------------------------------------------------------ server calls
async function api(path, opts = {}) {
  const r = await fetch(path, opts);
  const j = await r.json().catch(() => ({ error: r.ok ? "bad response" : r.statusText }));
  if (!r.ok || j.error) throw new Error(j.error || r.statusText);
  return j;
}
const post = (path, body) => api(path, { method: "POST", body });
const form = o => new URLSearchParams(o).toString();

function msg(t, cls) {
  const m = $("memo");
  m.append(el("div", { class: cls || "" }, t));
  while (m.childNodes.length > 200) m.firstChild.remove();
  m.scrollTop = m.scrollHeight;
}
let toastTimer = null;
function showError(e) {
  const t = String(e && e.message || e);
  msg(t, "e");
  const ts = $("toast"); ts.textContent = t; ts.style.display = "block";
  clearTimeout(toastTimer); toastTimer = setTimeout(() => ts.style.display = "none", 6000);
}
$("toast").onclick = () => $("toast").style.display = "none";

function applyState(st, fromUndo) {
  if (!fromUndo && sceneText && st.text !== sceneText) { undoStack.push(sceneText); if (undoStack.length > 200) undoStack.shift(); redoStack = []; }
  sceneText = st.text;
  if (st.scene) sceneHash = st.scene;
  model = parseM3s(st.text);
  euler = st.euler;
  if (st.title !== undefined) { title = st.title; document.title = `Mandelbulb 3D — ${title}`; }
  if (st.notes) notes = st.notes;
  refreshAll();
  pollSoon();
  sendView(false);
}
let title = "";

async function commit(m = model) {
  try { applyState(await post("/api/scene", toText(m))); }
  catch (e) { showError(e); model = parseM3s(sceneText); refreshAll(); }
}
async function sendText(text, fromUndo) {
  try { applyState(await post("/api/scene", text), fromUndo); } catch (e) { showError(e); }
}
let navChain = Promise.resolve();
function nav(params) {
  navChain = navChain.then(async () => {
    try { applyState(await post("/api/nav", form(params))); } catch (e) { showError(e); }
  });
  return navChain;
}
function undo() { if (!undoStack.length) return; redoStack.push(sceneText); sendText(undoStack.pop(), true); }
function redo() { if (!redoStack.length) return; undoStack.push(sceneText); sendText(redoStack.pop(), true); }

// ------------------------------------------------------------ status polling
let lastImg = -1, lastFinal = -1, pollTimer = null, rendering = false, lastInfo = null, lastStage = "";
const finalWaiters = [];
async function poll() {
  clearTimeout(pollTimer);
  try {
    const s = await api("/api/status");
    rendering = s.rendering;
    // changed in another editor window: reload the parameters
    if (s.scene && sceneHash && s.scene !== sceneHash && !stateFetch) {
      stateFetch = api("/api/state").then(st => { if (st.scene !== sceneHash) applyState(st, true); }).catch(() => {}).finally(() => { stateFetch = null; });
    }
    if (s.img_ver !== lastImg && s.img_ver > 0) { lastImg = s.img_ver; $("img").src = "/api/image?v=" + s.img_ver; imgFull = s.full; }
    $("progress").firstElementChild.style.width = s.rendering ? s.progress + "%" : "0";
    const st = $("stage");
    st.textContent = s.error || ((s.rendering ? `${s.stage} · ${s.progress.toFixed(0)} %` : s.stage) || "");
    st.classList.toggle("err", !!s.error);
    if (s.error && s.error !== lastStage) { msg(s.error, "e"); }
    if (!s.rendering && s.stage !== lastStage && /— [\d.]+ s$/.test(s.stage) && /^calculated|^repainted/.test(s.stage)) msg(s.stage, "n");
    lastStage = s.error || s.stage;
    if (JSON.stringify(s.info) !== JSON.stringify(lastInfo)) { lastInfo = s.info; if (rtab === "Infos") renderRight(); }
    $("status2").textContent = s.w ? `${s.w} × ${s.h}${s.full ? " · calculated image" : " · preview"}` : "";
    if (s.final_ver !== lastFinal) {
      const first = lastFinal < 0; lastFinal = s.final_ver;
      if (!first) while (finalWaiters.length) finalWaiters.shift()(true);
    }
    if (!s.rendering && s.error) while (finalWaiters.length) finalWaiters.shift()(false);
  } catch (e) { $("stage").textContent = "server not reachable"; $("stage").classList.add("err"); }
  pollTimer = setTimeout(poll, rendering ? 150 : 700);
}
function pollSoon() { clearTimeout(pollTimer); pollTimer = setTimeout(poll, 60); }
let imgFull = false;
const waitFinal = () => new Promise(res => finalWaiters.push(res));

// ------------------------------------------------------------ widgets
function nudge(input, dir, big) {
  const v = input.value.trim();
  if (!/^-?\d*\.?\d+(e-?\d+)?$/i.test(v)) return false;
  const n = parseFloat(v), isInt = /^-?\d+$/.test(v);
  const d = isInt ? (big ? 10 : 1) : Math.max(Math.abs(n) * (big ? 0.1 : 0.01), 1e-6);
  const r = n + dir * d;
  input.value = isInt ? String(Math.round(r)) : String(+r.toPrecision(10));
  return true;
}
// a text field that commits on change; ↑/↓ change numbers by 1 % (Shift 10 %)
function field(id, value, onset, attrs = {}) {
  const e = el("input", { type: "text", id, value, ...attrs });
  e.addEventListener("change", () => onset(e.value.trim()));
  e.addEventListener("keydown", ev => {
    if ((ev.key === "ArrowUp" || ev.key === "ArrowDown") && nudge(e, ev.key === "ArrowUp" ? 1 : -1, ev.shiftKey)) { ev.preventDefault(); onset(e.value); }
    if (ev.key === "Enter") e.blur();
  });
  return e;
}
const fG = (id, k, attrs) => field(id, gv(k), v => setG({ [k]: v === "" ? null : v }), attrs);
// one component of a comma list key (mid, julia_c, ...)
function fVec(id, k, i, n, attrs) {
  return field(id, gvec(k, n)[i], v => { const p = gvec(k, n); p[i] = v || "0"; setG({ [k]: p.join(", ") }); }, attrs);
}
function check(id, checked, onset, label, title) {
  const c = el("input", { type: "checkbox", id });
  c.checked = !!checked;
  c.addEventListener("change", () => onset(c.checked));
  return label === undefined ? c : el("label", { title: title || "" }, c, label);
}
const cG = (id, k, label, title) => check(id, gbool(k), v => setG({ [k]: v }), label, title);
function select(id, value, opts, onset, attrs = {}) {
  const s = el("select", { id, ...attrs }, ...opts.map(o => Array.isArray(o) ? el("option", { value: o[0] }, o[1]) : el("option", { value: o }, o)));
  if (![...s.options].some(o => o.value === String(value))) s.prepend(el("option", { value }, value));
  s.value = String(value);
  s.addEventListener("change", () => onset(s.value));
  return s;
}
function radios(name, value, opts, onset) {
  return el("div", { class: "radios" }, ...opts.map(([v, lab]) => {
    const r = el("input", { type: "radio", name, value: v });
    r.checked = String(value) === String(v);
    r.addEventListener("change", () => r.checked && onset(v));
    return el("label", { style: "display:block" }, r, lab);
  }));
}
// MB3D's track bars: a slider with the value next to it
function trackbar(id, label, value, min, max, onset, opts = {}) {
  const r = el("input", { type: "range", id, min, max, step: opts.step || 1, value });
  const t = el("input", { type: "text", value: opts.fmt ? opts.fmt(value) : value, title: opts.title || "" });
  r.addEventListener("input", () => { t.value = opts.fmt ? opts.fmt(r.value) : r.value; });
  r.addEventListener("change", () => onset(r.value));
  r.addEventListener("dblclick", () => { if (opts.def !== undefined) onset(opts.def); });
  t.addEventListener("change", () => onset(opts.parse ? opts.parse(t.value) : t.value));
  return [el("label", { for: id, title: opts.title || "" }, label), r, t];
}
const tbG = (id, label, k, min, max, opts) => trackbar(id, label, gnum(k), min, max, v => setG({ [k]: v }), opts);
function colorIn(id, value, onset, title) {
  const c = el("input", { type: "color", id, value: (value || "#000000").toLowerCase(), title: title || "" });
  c.addEventListener("change", () => onset(c.value.toUpperCase()));
  return c;
}
const colG = (id, k, title) => colorIn(id, gv(k), v => setG({ [k]: v }), title);
const lab = (t, title) => el("label", { title: title || "" }, t);
const btn = (t, onclick, attrs = {}) => el("button", { onclick, ...attrs }, t);
function tabsRow(names, cur, onpick, cls = "tabs2") {
  return el("div", { class: cls }, ...names.map(n => {
    const [key, text] = Array.isArray(n) ? n : [n, n];
    return el("button", { class: key === cur ? "on" : "", onclick: () => onpick(key) }, text);
  }));
}

// re-render a container and keep the focused control and the scroll position
function rerender(box, build) {
  const a = document.activeElement;
  const id = a && box.contains(a) ? a.id : null;
  const sel = id && a.selectionStart !== undefined ? [a.selectionStart, a.selectionEnd] : null;
  const sc = box.scrollTop;
  box.textContent = "";
  build(box);
  box.scrollTop = sc;
  if (id && $(id) && box.contains($(id))) { const e = $(id); e.focus(); if (sel && e.setSelectionRange) try { e.setSelectionRange(...sel); } catch (x) { /* not a text field */ } }
}

// ------------------------------------------------------------ picking points in the image
// MB3D: "click image" buttons (get midpoint, julia/cutting values, light
// position, DOF focus): the next click into the image picks a point.
let pickCb = null;
let pickToken = 0;
const remotePicks = new Map();
function pickFromImage(text, cb) {
  if (SOLO && chan) {
    // the image is in the main window: it picks the point and sends it back
    const token = `${SOLO}-${++pickToken}`;
    remotePicks.set(token, cb);
    chan.postMessage({ type: "pick", text, token });
    msg(text + ": click into the image of the main window.", "n");
    return;
  }
  pickCb = cb;
  $("hint").textContent = text + " — click into the image (Esc cancels)";
  $("hint").style.display = "block";
  $("view").classList.add("m-pick");
}
function endPick() { pickCb = null; $("hint").style.display = "none"; $("view").classList.remove("m-pick"); }
async function pickAt(u, v) { return api("/api/pick?" + form({ u, v })); }
if (chan) chan.onmessage = async e => {
  const d = e.data || {};
  if (d.type === "pick" && !SOLO) pickFromImage(d.text, (u, v) => chan.postMessage({ type: "picked", token: d.token, u, v }));
  if (d.type === "picked" && remotePicks.has(d.token)) {
    const cb = remotePicks.get(d.token); remotePicks.delete(d.token);
    try { await cb(d.u, d.v); } catch (x) { showError(x); }
  }
  if (d.type === "recalcOn" && !SOLO) { recalcOn = d.on; if (!d.on) { recalcSel = null; $("sel").style.display = "none"; } }
  if (d.type === "recalcSel" && SOLO) { recalcSel = d.sel; if (isOpen("postpro")) renderWin("postpro"); }
};
