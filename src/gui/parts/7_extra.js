
// ============================================================ Recalculate a selection, double imagesize
let recalcOn = false, recalcSel = null;
const recalcCfg = Object.assign({ div: "1", nearer: false }, store.get("recalc", {}));
function recalcPanel(d) {
  d.append(el("div", { class: "hint" }, "Enable, mark a rectangle in the image, then recalculate it with the current parameters (e.g. a smaller raystep against overstepping). Uses the last \"Calculate 3D\" image."),
    el("div", { class: "grid" },
      el("div", { class: "full" }, check("rcOn", recalcOn, c => { recalcOn = c; if (!c) { recalcSel = null; $("sel").style.display = "none"; } renderWin("postpro"); }, "Enable (mark the selection with the mouse)")),
      lab("Raystep divisor:"), field("rcDiv", recalcCfg.div, v => { recalcCfg.div = v; store.set("recalc", recalcCfg); }, { class: "n" }),
      el("div", { class: "full" }, check("rcNear", recalcCfg.nearer, c => { recalcCfg.nearer = c; store.set("recalc", recalcCfg); }, "Keep only nearer parts", "To reduce overstepping"))),
    el("div", { class: "btns" }, btn("Calculate", async () => {
      if (!recalcSel) return showError("Mark a selection in the image first.");
      try { await navChain; await post("/api/recalc", form({ ...recalcSel, div: recalcCfg.div, nearer: recalcCfg.nearer })); pollSoon(); } catch (e) { showError(e); }
    }, { class: "on", disabled: !recalcOn }), el("span", { class: "hint" }, recalcSel ? `selection ${Math.round((recalcSel.u1 - recalcSel.u0) * gnum("width"))} × ${Math.round((recalcSel.v1 - recalcSel.v0) * gnum("height"))} px` : "")));
}
function doublePanel(d) {
  d.append(el("div", { class: "hint" }, `Doubles the image size to ${gnum("width") * 2} × ${gnum("height") * 2} (with the DE stop and DOF radius) and calculates it again.`),
    el("div", { class: "btns" }, btn("Calculate now", async () => { await nav({ op: "scale", f: 2, destop: 1 }); calc3D(); })));
}

// ============================================================ JIT formula editor (JITFormulaEditGUI)
const JIT_TEMPLATE = `[OPTIONS]
.Version = 9
.DEscale = 1
.SIPow = 2
.Double Scale = 1
[CONSTANTS]
[SOURCE]
procedure NewFormula(var x, y, z, w: Double; PIteration3D: TPIteration3D);
var
  r: Double;
begin
  r := x*x - y*y - z*z;
  y := 2*x*y*Scale + PIteration3D^.J2;
  z := 2*x*z*Scale + PIteration3D^.J3;
  x := r*Scale + PIteration3D^.J1;
end;
[END]

Description:
A new JIT formula (Mandelbrot-like power 2 in 3D).
`;
let jit = null;
function splitM3f(text) {
  const out = { options: [], constants: [], code: [], desc: [] };
  let cur = null;
  for (const line of text.replace(/\r\n/g, "\n").split("\n")) {
    const m = line.trim().match(/^\[(OPTIONS|CONSTANTS|SOURCE|END)\]$/i);
    if (m) { cur = { OPTIONS: "options", CONSTANTS: "constants", SOURCE: "code", END: "desc" }[m[1].toUpperCase()]; continue; }
    if (cur) out[cur].push(line);
  }
  const j = k => out[k].join("\n").replace(/^\n+|\s+$/g, "");
  return { options: j("options"), constants: j("constants"), code: j("code"), desc: j("desc").replace(/^Description:\s*/i, "") };
}
const joinM3f = j => `[OPTIONS]\n${j.options.trim()}\n[CONSTANTS]\n${j.constants.trim()}${j.constants.trim() ? "\n" : ""}[SOURCE]\n${j.code.replace(/\s+$/, "")}\n[END]\n\nDescription:\n${j.desc.trim()}\n`;
async function openJit(name) {
  jit = null;
  if (name && !formulaNames.slice(0, 8).includes(name)) {
    try {
      const r = await api("/api/jit?name=" + encodeURIComponent(name));
      if (/\[SOURCE\]/i.test(r.text)) jit = { name, file: r.file, ...splitM3f(r.text) };
      else msg(`${name} is a machine code formula; only JIT formulas (with Pascal source) can be edited. Starting a new one.`, "n");
    } catch (e) { /* new formula */ }
  }
  if (!jit) jit = { name: "", file: "", ...splitM3f(JIT_TEMPLATE) };
  jit.tab = "Code"; jit.status = "";
  openWin("jit");
}
defWin("jit", "JIT-compiled formula", 640, p => {
  if (!jit) { p.append(el("div", { class: "hint" }, "Use the JIT button of the Formulas window.")); return; }
  const ta = (k, rows) => { const t = el("textarea", { id: "jit_" + k, rows, spellcheck: "false" }); t.value = jit[k]; t.addEventListener("input", () => { jit[k] = t.value; }); return t; };
  p.append(el("div", { class: "grid" }, lab("Formula name:"), field("jitName", jit.name, v => { jit.name = v; }, { placeholder: "e.g. JITMyBulb" })),
    jit.file ? el("div", { class: "hint mono" }, jit.file) : null);
  p.append(el("div", { style: "display:grid;grid-template-columns:1fr 1fr;gap:6px" },
    el("div", {}, el("div", { class: "sect" }, "Options", el("span", { class: "hint" }, "  .Double Name = default, .Integer …")), ta("options", 6)),
    el("div", {}, el("div", { class: "sect" }, "Constants"), ta("constants", 6))));
  p.append(tabsRow(["Code", "Description", "Supported functions"], jit.tab, t => { jit.tab = t; renderWin("jit"); }));
  const pg = el("div", { class: "page2" });
  if (jit.tab === "Code") pg.append(ta("code", 18));
  else if (jit.tab === "Description") pg.append(ta("desc", 18));
  else pg.append(el("div", { class: "mono" }, "Pascal subset of MB3D's JIT formulas: := + - * / div mod, if/then/else, for/while/repeat, begin/end, var Double/Integer; " +
    "functions abs sqr sqrt sin cos tan arctan arctan2 arcsin arccos exp ln log10 log2 power intpower round trunc frac floor ceil min max sign sinh cosh tanh; " +
    "PIteration3D^.J1..J3 (julia/C values), x, y, z, w; the options and constants by name."));
  p.append(pg);
  const jitCheck = async save => {
    try {
      const r = await post(`/api/jit/${save ? "save" : "compile"}?name=` + encodeURIComponent(jit.name.trim()), joinM3f(jit));
      jit.status = save ? `saved: ${r.file}` : `compiled ok: ${r.options} options`;
      if (save) { jit.file = r.file; await loadFormulas(); msg(`JIT formula ${jit.name} saved to ${r.file}`); }
    } catch (e) { jit.status = "error: " + e.message; }
    renderWin("jit");
  };
  p.append(el("div", { class: "btns" }, btn("Compile", () => jitCheck(false)), btn("Save", () => jitCheck(true), { class: "on", title: "Save the .m3f into the formula folder (overwrites the file it was loaded from)" }),
    btn("Use in formula slot", async () => {
      if (!jit.file) return showError("Save the formula first.");
      await setFormula(fwSlot, jit.name.trim()); openWin("formulas");
    }, { title: "Put it into the selected slot of the Formulas window" }),
    btn("New", () => openJit("")), btn("Load…", e => {
      const jitNames = formulaNames.filter(n => /^jit/i.test(n));
      showFlist(e.target, jitNames.length ? jitNames : formulaNames.slice(8), n => openJit(n));
    })),
    el("div", { class: "hint", style: jit.status.startsWith("error") ? "color:var(--err);white-space:pre-wrap" : "" }, jit.status));
}, { selfRefresh: true });

// ============================================================ Map sequences (MapSequencesGUI)
let mapseq = null;
async function mapseqLoad() { try { mapseq = await api("/api/mapseq"); $("frame").value = mapseq.frame; renderWin("mapseq"); } catch (e) { showError(e); } }
function mapseqText(list) {
  return `Count=${list.length}\r\n` + list.map((q, i) => `DestChannel#${i}=${q.channel}\r\nImageFilename#${i}=${q.filename}\r\nFirstImage#${i}=${q.first}\r\nLastImage#${i}=${q.last}\r\nLoop#${i}=${q.loop ? 1 : 0}\r\nIncrement#${i}=${q.increment}\r\n`).join("");
}
async function mapseqSave(list) { try { mapseq = await post("/api/mapseq", mapseqText(list)); renderWin("mapseq"); msg("Map sequences saved."); } catch (e) { showError(e); } }
defWin("mapseq", "Map Sequences", 560, p => {
  if (!mapseq) { p.append(el("div", { class: "hint" }, "loading…")); return; }
  const list = mapseq.list.map(q => ({ ...q }));
  p.append(el("div", { class: "hint" }, "An image sequence replaces a map number: the frame number (bottom bar, or the animation frame) goes into the digits before the extension, e.g. wave0001.png → wave0012.png. File names are on the server, absolute or relative to the map folders."));
  const g = el("div", { style: "display:grid;grid-template-columns:52px 1fr 48px 48px 40px 36px 24px;gap:3px 4px;align-items:center" },
    ...["Map nr", "First image file", "First", "Last", "Step", "Loop", ""].map(t => el("span", { class: "dim" }, t)));
  list.forEach((q, i) => {
    const f = (k, w) => field(`ms${i}${k}`, q[k], v => { q[k] = k === "filename" ? v : +v || 0; mapseqSave(list); }, w ? { style: `width:${w}px` } : {});
    g.append(f("channel"), el("div", { title: q.current }, f("filename")), f("first"), f("last"), f("increment"),
      check(`ms${i}l`, q.loop, c => { q.loop = c; mapseqSave(list); }), btn("✕", () => { list.splice(i, 1); mapseqSave(list); }, { title: "Delete" }));
  });
  p.append(g, el("div", { class: "btns" }, btn("Add sequence", () => {
    const used = new Set(list.map(q => q.channel)); let n = 1; while (used.has(n)) n++;
    list.push({ channel: n, filename: "sequence0001.png", first: 1, last: 100, increment: 1, loop: true }); mapseqSave(list);
  })), el("div", { class: "hint" }, `Frame ${mapseq.frame}` + (mapseq.list.length ? ": " + mapseq.list.map(q => `map ${q.channel} = ${q.current || "?"}`).join(", ") : "")),
    el("div", { class: "hint mono" }, "stored in " + mapseq.file));
}, { selfRefresh: true, onopen: mapseqLoad });
$("frame").onchange = async () => {
  try { mapseq = await post("/api/frame", form({ frame: $("frame").value })); renderWin("mapseq"); pollSoon(); } catch (e) { showError(e); }
};
