
// ============================================================ Post processing window (PostProcessForm)
const ppOpen = store.get("ppOpen", { hs: true });
defWin("postpro", "Post processing", 380, p => {
  const sec = (key, title, build) => {
    const b = el("button", { class: "ppbtn" + (ppOpen[key] ? " open" : ""), title: "Click to hide/show", onclick: () => { ppOpen[key] = !ppOpen[key]; store.set("ppOpen", ppOpen); renderWin("postpro"); } }, title);
    p.append(b);
    if (ppOpen[key]) { const d = el("div", { class: "pppnl" }); build(d); p.append(d); }
  };
  p.append(el("div", { class: "hint" }, imgFull ? "Changes are applied to the calculated image right away (the G-buffer is kept)." : "The settings apply to the previews and to \"Calculate 3D\"."));
  sec("nz", "Normals on Z-buffer", d => d.append(cG("ppNz", "normals_on_zbuf", "Do this function automatically"),
    el("div", { class: "hint" }, "Normals from the positions of the neighbour pixels; only if the normals on the surface look bad.")));
  sec("hs", "Hard Shadows", d => {
    const on = gv("shadows") !== "off" && gv("shadows") !== "none";
    const set = new Set(gv("shadows") === "all" ? [1, 2, 3, 4, 5, 6] : on ? gv("shadows").split(",").map(x => +x.trim()) : []);
    const emit = () => setG({ shadows: set.size ? [...set].sort().join(",") : "off" });
    d.append(el("div", { class: "btns" }, ...[1, 2, 3, 4, 5, 6].map(i => check("ppL" + i, set.has(i), c => { c ? set.add(i) : set.delete(i); emit(); }, "Light " + i, lightSec(i) ? "" : "this light is off"))),
      el("div", { class: "grid" },
        el("div", { class: "full" }, cG("ppSoft", "shadow_soft", "Softer H.S.", "Better soft shadow, only with 1 light selected")),
        lab("Radius:", "Bigger radius means softer shadows"), fG("ppRad", "shadow_soft_radius", { class: "n" }),
        lab("Max. length calc.:", "Multiplier of the maximal shadow length"), fG("ppMl", "shadow_max_len", { class: "n" }),
        el("div", { class: "full" }, cG("ppCos", "shadow_set_cos", "Set lightfunc to cosine"))));
  });
  sec("as", "Ambient Shadows", d => {
    const ao = gv("ao", "ssao24");
    const t = ao === "off" ? "off" : ao === "deao" ? "DEAO" : ao.startsWith("ssao15") ? "SSAO15" : +gv("ao_random") > 0 ? "SSAO24r" : "SSAO24";
    const t0 = ao.endsWith("t0");
    d.append(check("ppAo", ao !== "off", c => setG({ ao: c ? "ssao24" : "off" }), "Calculate A.S. automatically"));
    if (ao === "off") return;
    d.append(tabsRow(["SSAO15", "SSAO24", "SSAO24r", "DEAO"], t, n => {
      if (n === "SSAO15") setG({ ao: t0 ? "ssao15t0" : "ssao15", ao_random: null });
      if (n === "SSAO24") setG({ ao: t0 ? "ssao24t0" : "ssao24", ao_random: null });
      if (n === "SSAO24r") setG({ ao: t0 ? "ssao24t0" : "ssao24", ao_random: Math.max(1, +gv("ao_random")) });
      if (n === "DEAO") setG({ ao: "deao", ao_random: null });
    }));
    const g = el("div", { class: "grid page2" });
    if (t === "DEAO") g.append(lab("Rays:"), select("ppDq", gv("deao_quality"), [["0", "3"], ["1", "7"], ["2", "17"], ["3", "33"]], v => setG({ deao_quality: v })),
      lab("MaxL:", "Scales the maximum length of the ambient rays"), fG("ppDm", "deao_max_len", { class: "n" }),
      lab("Dithering for scale:"), select("ppDd", gv("deao_dither"), [["0", "none"], ["1", "2x2 (1:2)"], ["2", "3x3 (1:3)"]], v => setG({ deao_dither: v })));
    else {
      g.append(lab("Z/R Threshold:", "Up to which angle the shadowing is calculated"), fG("ppTh", "ao_threshold", { class: "n" }),
        el("div", { class: "full" }, check("ppT0", t0, c => setG({ ao: (t === "SSAO15" ? "ssao15" : "ssao24") + (c ? "t0" : "") }), "Threshold to 0")),
        lab("Border size:", "Extends the calculation outside the image with mirrored parts"), fG("ppBo", "ao_border", { class: "n" }));
      if (t === "SSAO24r") g.append(lab("Calculation count:", "Calculate n times to reduce the noise"), fG("ppRn", "ao_random", { class: "n" }));
    }
    d.append(g);
  });
  sec("rt", "Reflections + Transparency", d => d.append(el("div", { class: "grid" },
    el("div", { class: "full" }, cG("ppR", "mc_reflections", "Calculate R. (+ T.) automatically")),
    lab("Depth:"), fG("ppRd", "mc_reflection_depth", { class: "n" }),
    lab("Amount:", "Total amount of reflectivity + transparency, times the specular colours"), fG("ppRa", "mc_reflection_amount", { class: "n" }),
    el("div", { class: "full" }, cG("ppT", "mc_transparency", "Calculate transparency", "The specular amount is split into a ray going inside and the reflection")),
    el("div", { class: "full" }, cG("ppTd", "mc_only_difs", "Only dIFS")),
    lab("Absorption:"), fG("ppAb", "mc_absorption", { class: "n" }),
    lab("Refractive index:", "air ~1, water ~1.33, glass ~1.5, diamond ~2.4"), fG("ppRi", "mc_refraction_index", { class: "n" }),
    lab("Light scattering:"), fG("ppLs", "mc_scattering", { class: "n" }))));
  sec("dof", "Depth of Field", d => {
    const on = gv("dof") !== "off";
    const get = k => pickFromImage("Sharp point", async (u, v) => { const r = await pickAt(u, v); if (!r.pos) throw new Error("background picked"); setG({ [k]: +r.z.toFixed(6), dof: gv("dof") === "off" ? "sorted" : gv("dof") }); });
    d.append(check("ppDof", on, c => setG({ dof: c ? "sorted" : "off" }), "Calculate DoF automatically"),
      el("div", { class: "grid" },
        lab("Z1 sharp:", "Focus point: distance from the camera relative to the image width"), el("div", { class: "row2" }, fG("ppZ1", "dof_focus"), btn("Get Z1", () => get("dof_focus"), { style: "flex:none" })),
        lab("Z2 sharp:", "Second focus point for a focus line"), el("div", { class: "row2" }, fG("ppZ2", "dof_focus2"), btn("Get Z2", () => get("dof_focus2"), { style: "flex:none" }),
          btn("<-", () => setG({ dof_focus2: gv("dof_focus") }), { style: "flex:none", title: "Copy Z1 to Z2 (focus point)" })),
        lab("ClippingR:", "The blur radius is limited to this"), fG("ppCr", "dof_max_radius", { class: "n" }),
        lab("Aperture:"), fG("ppAp", "dof_aperture", { class: "n" }),
        lab("Passes:"), select("ppPa", gv("dof_passes"), ["1", "2", "3"], v => setG({ dof_passes: v })),
        lab("Calculation:"), select("ppCa", on ? gv("dof") : "sorted", [["sorted", "Sorted"], ["forward", "Forward"]], v => setG({ dof: v }))));
  });
  sec("rs", "Recalculate a Selection", d => recalcPanel(d));
  sec("ds", "Double imagesize", d => doublePanel(d));
});

// ============================================================ Navigator window
let naviPanel = store.get("naviPanel", "Misc"), naviF = 0;
defWin("navi", "Navigator", 360, p => {
  const b = (t, op, axis, sign, title) => el("button", { title, onclick: e => doNav(op, axis, sign, e.shiftKey) }, t);
  p.append(el("div", { class: "hint" }, "Walk in the main image (mode \"walk\"): click flies towards a point, drag turns, wheel moves. Keys in the image: W/S A/D R/F, arrows, Q/E, +/−; Shift for finer steps."));
  p.append(el("div", { class: "grid page2" },
    lab("Walking"), el("div", { class: "btns" }, b("fwd (w)", "move", 2, 1), b("back (s)", "move", 2, -1)),
    lab("Sliding"), el("div", { class: "btns" }, b("◀ (a)", "move", 0, -1), b("▶ (d)", "move", 0, 1), b("▲ (r)", "move", 1, -1), b("▼ (f)", "move", 1, 1)),
    lab("Looking"), el("div", { class: "btns" }, b("←", "rotate", 1, -1), b("→", "rotate", 1, 1), b("↑", "rotate", 0, 1), b("↓", "rotate", 0, -1)),
    lab("Rolling"), el("div", { class: "btns" }, b("↺ (q)", "rotate", 2, -1), b("↻ (e)", "rotate", 2, 1), b("zoom +", "zoom", 0, 1), b("zoom −", "zoom", 0, -1)),
    lab("Step (% of DE):", "Moves are a percentage of the distance estimate at the camera"), field("nvStep", naviCfg.step, v => { naviCfg.step = v; saveNavi(); }, { class: "n" }),
    lab("Angle (°):"), field("nvAng", naviCfg.angle, v => { naviCfg.angle = v; saveNavi(); }, { class: "n" }),
    lab("Click in image:"), select("nvClick", naviCfg.click, [["fly", "flies towards the point"], ["look", "turns to the point"], ["focus", "sets the DOF focus"]], v => { naviCfg.click = v; saveNavi(); })));
  // the parameter sliders (Panel3 buttons)
  p.append(tabsRow(["Julia", "Formula values", "4d rotation", "Misc"], naviPanel, n => { naviPanel = n; store.set("naviPanel", n); renderWin("navi"); }));
  const pg = el("div", { class: "page2" });
  pg.append(el("div", { class: "btns" }, lab("Adjustments:"), select("nvAdj", String(naviCfg.adj), [["1", "fine (1 %)"], ["10", "medium (10 %)"], ["100", "coarse (100 %)"]], v => { naviCfg.adj = +v; saveNavi(); })));
  // relative slider: dragging changes the value by up to ±adj %, released it snaps back
  const rel = (id, label, get, set, unit = 1) => {
    const r = el("input", { type: "range", id, min: -100, max: 100, value: 0 });
    const base = get();
    const t = el("span", { class: "mono" }, String(+base.toPrecision(8)));
    const val = () => base + (+r.value / 100) * (naviCfg.adj / 100) * (unit === 1 ? (Math.abs(base) > 1e-3 ? Math.abs(base) : 1) : unit);
    r.addEventListener("input", () => { t.textContent = String(+val().toPrecision(8)); });
    r.addEventListener("change", () => set(+val().toPrecision(12)));
    return [lab(label), r, t];
  };
  const tb = el("div", { class: "tb" });
  if (naviPanel === "Julia") {
    const jc = () => gvec("julia_c", 4).map(Number);
    ["x", "y", "z"].forEach((n, i) => tb.append(...rel("nvJ" + i, n, () => jc()[i], v => { const c = jc(); c[i] = v; setG({ julia_c: c.join(", "), julia: true }); })));
    pg.append(cG("nvJm", "julia", "Julia mode"), tb);
  } else if (naviPanel === "Formula values") {
    const secs = formulaSecs();
    if (naviF >= secs.length) naviF = 0;
    const s = secs[naviF];
    pg.append(el("div", { class: "btns" }, lab("F.nr:"), select("nvF", String(naviF), secs.map((x, i) => [String(i), `${i + 1}: ${sv(x, "name")}`]), v => { naviF = +v; renderWin("navi"); })));
    if (s) s.lines.forEach((l, i) => {
      if (l.k === "name" || isNaN(parseFloat(l.v))) return;
      tb.append(...rel("nvO" + i, /^option\d+$/.test(l.k) && l.c ? l.c : l.k, () => parseFloat(l.v), v => { l.v = String(/^-?\d+$/.test(l.v) && l.k === "iterations" ? Math.round(v) : v); commit(); }));
    });
    pg.append(tb);
  } else if (naviPanel === "4d rotation") {
    const r4 = () => gvec("rotation_4d", 3).map(Number);
    ["xw", "yw", "zw"].forEach((n, i) => tb.append(...rel("nv4" + i, n, () => r4()[i], v => { const c = r4(); c[i] = v; setG({ rotation_4d: c.join(", ") }); }, 180)));
    pg.append(tb);
  } else {
    const num = (k, unit) => rel("nvM" + k, k, () => gnum(k), v => setG({ [k]: k.includes("iter") || k === "dfog_on_it" ? Math.max(0, Math.round(v)) : v }), unit);
    tb.append(...num("iterations", 20), ...num("rstop"), ...num("decomb_smooth"), ...num("de_stop"), ...num("dfog_on_it", 20));
    pg.append(tb);
  }
  p.append(pg);
});

// ============================================================ smaller windows
defWin("text", "Parameter text", 520, p => {
  const ta = el("textarea", { id: "txtArea", rows: 28, spellcheck: "false" });
  ta.value = sceneText;
  p.append(el("div", { class: "hint" }, "All parameters as .m3s text (the keys are listed in the README). Edit and apply."), ta,
    el("div", { class: "btns" }, btn("Apply", () => sendText(ta.value), { class: "on" }), btn("Reload", () => renderWin("text"))));
}, { selfRefresh: true });

let dirsState = null;
defWin("dirs", "Ini Dirs", 460, p => {
  if (!dirsState) { p.append(el("div", { class: "hint" }, "loading…")); api("/api/dirs").then(d => { dirsState = d; renderWin("dirs"); }).catch(showError); return; }
  const list = (t, a) => [el("div", { class: "sect" }, t), ...a.map(d => el("div", { class: "mono" }, d))];
  const inp = el("input", { type: "text", id: "dirIn", placeholder: "folder on the server, e.g. /home/me/mb3d/M3Formulas" });
  const add = kind => post("/api/dirs", form({ kind, dir: inp.value })).then(d => { dirsState = d; renderWin("dirs"); loadFormulas(); msg(`${kind} folder added: ${inp.value}`); }).catch(showError);
  p.append(el("div", { class: "hint" }, "Folders searched by the server (first match wins). Also: mb3d gui --formulas DIR --maps DIR, or MB3D_FORMULAS / MB3D_MAPS."),
    ...list("Formulas (.m3f)", dirsState.formulas), ...list("Maps", dirsState.maps), ...list("Work folders (as in MB3D)", dirsState.work || []),
    el("div", { class: "sect" }, "Add a folder"), inp, el("div", { class: "btns" }, btn("Add formula folder", () => add("formulas")), btn("Add map folder", () => add("maps"))),
    el("div", { class: "hint mono" }, "server working folder: " + dirsState.cwd));
}, { onopen: () => { dirsState = null; renderWin("dirs"); } });

defWin("zbuf", "ZBuf16Bit", 330, p => p.append(
  el("div", { class: "hint" }, "Saves the z-buffer of the image on screen as a 16 bit grey PNG (near = bright, background = black), e.g. for depth effects in other programs."),
  el("div", { class: "btns" }, btn("Save Z-buffer (16 bit PNG)", () => location.href = "/api/zbuf.png", { class: "on" })),
  el("div", { class: "hint" }, "For the full resolution press \"Calculate 3D\" first. Animations can write depth images per frame too (Animation maker).")));

// ---- Big renders (Tiling)
const bigCfg = Object.assign({ factor: "2", tx: "2", ty: "2", ds: "1", scaleDe: true, col: "1", row: "1" }, store.get("big", {}));
defWin("tiling", "Big renders", 380, p => {
  const f = +bigCfg.factor || 1, W = Math.round(gnum("width") * f), H = Math.round(gnum("height") * f), ds = +bigCfg.ds || 1;
  const inp = (k, title) => field("bg_" + k, bigCfg[k], v => { bigCfg[k] = v; store.set("big", bigCfg); renderWin("tiling"); }, { class: "n", title: title || "" });
  const q = () => ({ factor: bigCfg.factor, scale_de: bigCfg.scaleDe, tiles: `${bigCfg.tx}x${bigCfg.ty}`, downscale: bigCfg.ds });
  p.append(el("div", { class: "grid" },
    lab("Original size:"), el("span", {}, `${gv("width")} × ${gv("height")}`),
    lab("Size factor:"), inp("factor"),
    el("div", { class: "full" }, check("bgDe", bigCfg.scaleDe, c => { bigCfg.scaleDe = c; store.set("big", bigCfg); }, "Scale DEstop to keep color & detail level")),
    lab("Tile count, horizontal:"), inp("tx"), lab("vertical:"), inp("ty"),
    lab("Tile downscale, anti aliasing:"), select("bgDs", bigCfg.ds, ["1", "2", "3"], v => { bigCfg.ds = v; store.set("big", bigCfg); renderWin("tiling"); }),
    lab("Big size:"), el("span", {}, `${W} × ${H}` + (ds > 1 ? ` (calculated ${W * ds} × ${H * ds})` : "")),
    lab("Tile size:"), el("span", {}, `${Math.ceil(W * ds / (+bigCfg.tx || 1))} × ${Math.ceil(H * ds / (+bigCfg.ty || 1))}`)));
  p.append(el("div", { class: "btns" }, btn("Render all tiles", async () => {
    try { await post("/api/render", form({ aa: 1, ...q() })); msg(`Big render ${W}x${H} started.`, "n"); pollSoon(); } catch (e) { showError(e); }
  }, { class: "on", title: "One tile after another, stitched; then save it with Save pic" }), btn("Stop", () => post("/api/cancel", ""))));
  p.append(el("div", { class: "sect" }, "Tile parameter files"), el("div", { class: "hint" }, "One MB3D tile file per machine or process (also: mb3d file.m3p --tiles 4x4 --tile 2,3)."),
    el("div", { class: "btns" }, lab("Tile column"), inp("col"), lab("row"), inp("row"),
      btn("Save tile .m3p", () => location.href = "/api/tile.m3p?" + form({ ...q(), col: bigCfg.col, row: bigCfg.row }))));
});

// ---- Batch processing (m3p -> m3i)
let batch = { files: [], running: false, i: 0, log: [] };
const batchCfg = Object.assign({ folder: "batch", m3i: true, scale: "1" }, store.get("batch", {}));
defWin("batch", "Batch processing", 420, p => {
  const fi = el("input", { type: "file", multiple: true, accept: ".m3p,.m3i,.m3s,.txt", hidden: true });
  fi.onchange = () => { batch.files.push(...fi.files); renderWin("batch"); };
  p.append(el("div", { class: "hint" }, "Calculates the parameter files one after another with \"Calculate 3D\" and stores PNG (and .m3i) files in a folder of the server. The editor shows each file while it is calculated."), fi,
    el("div", { class: "btns" }, btn("Open files", () => fi.click()), btn("Clear", () => { batch.files = []; batch.log = []; renderWin("batch"); }, { disabled: batch.running })),
    el("div", { class: "page2", style: "max-height:180px;overflow:auto" }, ...(batch.files.length ? batch.files.map((f, i) => el("div", { class: i < batch.i && batch.running ? "dim" : "" }, `${i + 1}. ${f.name}` + (batch.log[i] ? " — " + batch.log[i] : ""))) : [el("div", { class: "hint" }, "no files")])),
    el("div", { class: "grid" }, lab("Folder:"), field("btF", batchCfg.folder, v => { batchCfg.folder = v; store.set("batch", batchCfg); }),
      lab("Save scale 1:"), select("btS", batchCfg.scale, ["1", "2", "3"], v => { batchCfg.scale = v; store.set("batch", batchCfg); }),
      el("div", { class: "full" }, check("btM", batchCfg.m3i, c => { batchCfg.m3i = c; store.set("batch", batchCfg); }, "Store M3I files too"))),
    el("div", { class: "btns" }, btn("Start batch rendering", runBatch, { class: "on", disabled: batch.running || !batch.files.length }), btn("Stop", () => { batch.stop = true; post("/api/cancel", ""); }, { disabled: !batch.running })));
});
async function runBatch() {
  batch.running = true; batch.stop = false; batch.log = [];
  for (batch.i = 0; batch.i < batch.files.length && !batch.stop; batch.i++) {
    const f = batch.files[batch.i];
    renderWin("batch");
    try {
      applyState(await post("/api/open?name=" + encodeURIComponent(f.name), await f.arrayBuffer()), true);
      const done = waitFinal();
      await calc3D();
      if (!await done) throw new Error("calculation failed");
      const r = await post("/api/batch/store", form({ folder: batchCfg.folder, name: f.name.replace(/\.[^.]+$/, ""), scale: batchCfg.scale, m3i: batchCfg.m3i }));
      batch.log[batch.i] = "ok"; msg("stored " + r.file, "n");
    } catch (e) { batch.log[batch.i] = "error: " + e.message; showError(e); }
  }
  batch.running = false; renderWin("batch");
}
