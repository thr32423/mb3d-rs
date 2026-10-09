
// ============================================================ Formulas window (FormulaGUI)
let fwSlot = 0;
const FCATS = [["3D", "escapetime 3D formulas"], ["3Da", "escapetime 3D formulas with analytic (faster) DE"], ["4D", "escapetime 4D formulas"],
  ["4Da", "escapetime 4D formulas with analytic DE"], ["Ads", "transformations only, use with escapetime formulas"], ["dIFS", "dIFS shapes"], ["dIFS t", "dIFS transformations"]];
function fcat(de) {
  if (de === 2 || de === 11) return "3Da";
  if (de === 4) return "4D";
  if (de === 5 || de === 6) return "4Da";
  if (de === -1 || de === -2) return "Ads";
  if (de === 20) return "dIFS";
  if (de === 21 || de === 22) return "dIFS t";
  return "3D";
}
let flistEl = null;
function closeFlist() { if (flistEl) { flistEl.remove(); flistEl = null; } }
addEventListener("mousedown", e => { if (flistEl && !flistEl.contains(e.target)) closeFlist(); }, true);
function showFlist(anchor, names, onpick) {
  closeFlist();
  const b = anchor.getBoundingClientRect();
  const rows = Math.max(8, Math.min(32, Math.ceil(names.length / Math.max(1, Math.floor((innerWidth - 40) / 150)))));
  flistEl = el("div", { class: "flist" }, ...names.map(n => el("div", { onclick: () => { closeFlist(); onpick(n); } }, n)));
  flistEl.style.setProperty("--rows", rows);
  flistEl.style.left = Math.min(b.left, innerWidth - 200) + "px";
  flistEl.style.top = (b.bottom + 2) + "px";
  document.body.append(flistEl);
  const r = flistEl.getBoundingClientRect();
  if (r.right > innerWidth - 4) flistEl.style.left = Math.max(4, innerWidth - 4 - r.width) + "px";
  if (r.bottom > innerHeight - 4) flistEl.style.top = Math.max(4, b.top - r.height - 2) + "px";
}
function setFormula(slot, name) {
  const secs = formulaSecs();
  if (!name) { if (secs[slot]) model.sections.splice(model.sections.indexOf(secs[slot]), 1); return commit(); }
  if (secs[slot]) {
    const its = sv(secs[slot], "iterations", "1");
    secs[slot].lines = [{ k: "name", v: name, c: "" }, { k: "iterations", v: its, c: "" }];
  } else {
    // a new formula: the server fills in its default options
    const after = secs.length ? model.sections.indexOf(secs[secs.length - 1]) + 1 : 0;
    model.sections.splice(after, 0, { type: "formula", lines: [{ k: "name", v: name, c: "" }, { k: "iterations", v: "1", c: "" }] });
    fwSlot = secs.length;
  }
  return commit();
}
function swapFormula(i, j) {
  const secs = formulaSecs();
  if (j < 0 || j >= secs.length) return;
  const a = model.sections.indexOf(secs[i]), b = model.sections.indexOf(secs[j]);
  [model.sections[a], model.sections[b]] = [model.sections[b], model.sections[a]];
  fwSlot = j;
  commit();
}
defWin("formulas", "Formulas", 480, p => {
  const secs = formulaSecs();
  if (fwSlot > secs.length) fwSlot = secs.length;
  p.append(tabsRow([0, 1, 2, 3, 4, 5].map(i => [i, (i === 0 ? "Formula1" : "F" + (i + 1)) + (secs[i] && +sv(secs[i], "iterations", "0") > 0 ? " •" : "")]),
    fwSlot, i => { if (i <= secs.length) { fwSlot = i; renderWin("formulas"); } }));
  const s = secs[fwSlot];
  const pg = el("div", { class: "page2" });
  const byCat = new Map(FCATS.map(c => [c[0], []]));
  for (const n of formulaNames) byCat.get(fcat(formulaDE.get(n) ?? 0)).push(n);
  const nameIn = el("input", { type: "text", id: "fwName", value: s ? sv(s, "name") : "", list: "formulaList", placeholder: "formula name", style: "flex:1" });
  nameIn.addEventListener("change", () => setFormula(fwSlot, nameIn.value.trim()));
  pg.append(el("div", { class: "btns" }, ...FCATS.map(([c, t]) => {
    const b = el("button", { title: t + ` (${byCat.get(c).length})` }, c);
    b.onclick = () => showFlist(b, byCat.get(c), n => setFormula(fwSlot, n));
    return b;
  }), btn("JIT", () => openJit(s ? sv(s, "name") : ""), { title: "Create or edit a JIT-compiled formula" })));
  pg.append(el("div", { class: "btns" }, nameIn,
    btn("◀", () => swapFormula(fwSlot, fwSlot - 1), { title: "Exchange this formula with the previous one", disabled: !s || fwSlot === 0 }),
    btn("▶", () => swapFormula(fwSlot, fwSlot + 1), { title: "Exchange this formula with the next one", disabled: !s || fwSlot >= secs.length - 1 }),
    btn("✕", () => setFormula(fwSlot, ""), { title: "Remove this formula", disabled: !s })));
  if (s) {
    const g = el("div", { class: "grid" });
    const ipol = gv("interpolation") !== "off";
    g.append(lab(ipol && fwSlot < 2 ? "Weight (interpolation)" : "Iterationcount"),
      field("fwIts", sv(s, "iterations", "1"), v => { putS(s, { iterations: v || "0" }); commit(); }, { class: "n" }));
    s.lines.forEach((l, i) => {
      if (l.k === "name" || l.k === "iterations") return;
      const name = /^option\d+$/.test(l.k) && l.c ? l.c : l.k;
      const f = field(`fwO${i}`, l.v, v => { l.v = v; commit(); }, { class: "n" });
      if (/normal/i.test(name)) {
        // "N": normalise the 3D vector of the next three options
        g.append(lab(name, l.k), el("div", { class: "row2" }, f, btn("N", () => {
          const vs = [0, 1, 2].map(j => s.lines[i + j]).filter(Boolean);
          const len = Math.hypot(...vs.map(x => parseFloat(x.v) || 0)) || 1;
          vs.forEach(x => { x.v = String((parseFloat(x.v) || 0) / len); });
          commit();
        }, { title: "Normalise the 3D vector length to 1", style: "flex:none" })));
      } else g.append(lab(name, l.k), f);
    });
    g.append(el("div", { class: "full" }, check("fwRep", +gv("repeat_from", "0") === fwSlot, c => setG({ repeat_from: c ? fwSlot : 0 }), "Repeat from here",
      "Where the iteration starts again after the last formula")));
    pg.append(g);
  } else pg.append(el("div", { class: "hint" }, "Empty slot: choose a formula from the lists or type its name."));
  p.append(pg);

  // hybrid type
  const ht = gv("interpolation") !== "off" ? "Interpolate" : gv("de_combination") !== "off" ? "DEcombinate" : "Alternate";
  p.append(tabsRow(["Alternate", "Interpolate", "DEcombinate"], ht, t => {
    if (t === "Alternate") setG({ interpolation: "off", de_combination: "off" });
    else if (t === "Interpolate") setG({ de_combination: null, interpolation: "1, 1" });
    else setG({ interpolation: null, de_combination: "min" });
  }));
  const hp = el("div", { class: "page2 grid" });
  if (ht === "Alternate") hp.append(el("div", { class: "full hint" }, "Formulas 1 to 6 alternated (each its iteration count)."));
  if (ht === "Interpolate") hp.append(el("div", { class: "full hint" }, "Formula 1 interpolated in each iteration with formula 2."),
    lab("Weight formula 1:"), fVec("fwW1", "interpolation", 0, 2, { class: "n" }), lab("Weight formula 2:"), fVec("fwW2", "interpolation", 1, 2, { class: "n" }));
  if (ht === "DEcombinate") hp.append(
    lab("2nd part starts at:", "Formula nr where the 2nd part of the hybrid starts"), select("fwS2", gv("decomb_start2"), ["2", "3", "4", "5", "6"], v => setG({ decomb_start2: v, decomb_end1: v - 1 })),
    lab("DE comb.:"), select("fwDC", gv("de_combination"), [["min", "Min"], ["max", "Max"], ["max_inverted", "Inv max"], ["smooth_linear", "Min lin"], ["smooth", "Min nlin"], ["mix", "Mix"]], v => setG({ de_combination: v })),
    lab("Ds:", "Smoothing of the DE combination"), fG("fwDs", "decomb_smooth", { class: "n" }),
    lab("Fpow:", "Mix power"), fG("fwFp", "decomb_mix_pow", { class: "n" }),
    lab("Maxits hybrid part2:"), fG("fwM2", "decomb_iterations2", { class: "n" }));
  p.append(hp);
  p.append(el("div", { class: "grid page2" },
    lab("R bailout:"), fG("fwRb", "rstop", { class: "n" }),
    lab("Max. iterations:"), fG("fwMax", "iterations", { class: "n" }),
    lab("Min. iterations:"), fG("fwMin", "min_iterations", { class: "n" }),
    el("div", { class: "full" }, cG("fwDA", "disable_analytic_de", "Disable analytical DE")),
    lab("Render:", "Outside: default; inside: raymarching on the inside with max. its; in and outside: both"),
    select("fwIn", gv("inside"), [["outside", "Outside render"], ["inside", "Inside render"], ["both", "In and outside"]], v => setG({ inside: v })),
    el("div", { class: "full dim" }, "4D rotation of the whole bulb:"),
    lab("XW / YW / ZW:"), el("div", { class: "row2" }, fVec("fwXW", "rotation_4d", 0, 3), fVec("fwYW", "rotation_4d", 1, 3), fVec("fwZW", "rotation_4d", 2, 3))));
  p.append(el("div", { class: "btns" }, btn("Reset", async () => {
    if (!confirm("Reset the formulas to the default Integer Power (P8) bulb?")) return;
    model.sections = model.sections.filter(x => x.type !== "formula");
    model.sections.unshift({ type: "formula", lines: [{ k: "name", v: "Integer Power", c: "" }, { k: "iterations", v: "1", c: "" }] });
    putG({ interpolation: null, de_combination: null, repeat_from: 0 });
    fwSlot = 0; commit();
  }, { title: "Resets the formulas to the default P8 bulb" }), el("span", { class: "hint" }, `${formulaNames.length} formulas`)));
});

// ============================================================ Lighting window (LightAdjust)
let lwSlot = 1, lwTab2 = store.get("lwTab2", "Object"), palEdit = null;
const lightMem = {};
function lightOn(slot, on) {
  const s = lightSec(slot);
  if (!on && s) { lightMem[slot] = JSON.parse(JSON.stringify(s)); model.sections.splice(model.sections.indexOf(s), 1); if (!lightSecs().length) putG({ lights: "none" }); }
  if (on && !s) {
    const n = lightMem[slot] || { type: "light", lines: [{ k: "slot", v: String(slot), c: "" }, { k: "color", v: "#FFFFFF", c: "" }, { k: "x_angle", v: "30", c: "" },
      { k: "y_angle", v: "30", c: "" }, { k: "amplitude", v: "1", c: "" }, { k: "spec_power", v: "8", c: "" }, { k: "diffuse_func", v: "0", c: "" }] };
    putS(n, { on: null });
    const others = lightSecs().filter(x => +sv(x, "slot") < slot);
    const at = others.length ? model.sections.indexOf(others[others.length - 1]) + 1 : model.sections.length;
    model.sections.splice(at, 0, n);
    putG({ lights: null });
  }
  return commit();
}
function lightType(s) { return !s ? "Global light" : +sv(s, "map", "0") > 0 ? "Lightmap" : sv(s, "position") ? "Positional light" : "Global light"; }
function setLightType(s, t) {
  if (t === "Global light") putS(s, { position: null, map: null, map_rotation: null, x_angle: sv(s, "x_angle", "30"), y_angle: sv(s, "y_angle", "30") });
  if (t === "Positional light") putS(s, { map: null, map_rotation: null, x_angle: null, y_angle: null, position: gv("mid") });
  if (t === "Lightmap") putS(s, { position: null, x_angle: null, y_angle: null, map: 1, map_rotation: "128, 128, 128" });
  commit();
}
// palette as CSS gradient (positions 0..32767)
function palGradient(entries, pick) {
  const e = entries.filter(x => x.length >= 2).map(x => [+x[0], pick(x)]).sort((a, b) => a[0] - b[0]);
  if (!e.length) return "#000";
  return `linear-gradient(90deg, ${e.map(([p, c]) => `${c} ${(p / 32767 * 100).toFixed(1)}%`).join(", ")})`;
}
const palItems = k => gv(k).split(",").map(s => s.trim().split(":")).filter(x => x.length >= 2);
function palEditor(kind) {
  if (kind === "interior") {
    const items = palItems("interior_colors");
    const emit = () => setG({ interior_colors: items.map(i => i.join(":")).join(", ") });
    return el("div", { class: "pal" }, el("span", { class: "dim" }, "position"), el("span", { class: "dim" }, "colour"), el("span", { class: "dim" }, "spec."), el("span"),
      ...items.flatMap((it, i) => [field(`ic${i}p`, it[0], v => { it[0] = v; emit(); }), colorIn(`ic${i}c`, it[1], v => { it[1] = v; emit(); }),
        field(`ic${i}s`, it[2] ?? "160", v => { it[2] = v; emit(); }), el("span")]));
  }
  const items = palItems("palette_full");
  const alpha = gv("palette_alpha", "").split(",").map(s => s.trim()).filter(Boolean);
  const emit = () => setG({ palette_full: items.map(i => i.join(":")).join(", "), palette_alpha: alpha.length ? alpha.join(", ") : null });
  return el("div", { class: "pal" }, el("span", { class: "dim" }, "position"), el("span", { class: "dim" }, "diffuse"), el("span", { class: "dim" }, "specular"),
    el("span", { class: "dim", title: "Transparency (alpha of the specular colour) for reflections + transparency" }, "alpha"),
    ...items.flatMap((it, i) => [field(`pc${i}p`, it[0], v => { it[0] = v; emit(); }), colorIn(`pc${i}d`, it[1], v => { it[1] = v; emit(); }),
      colorIn(`pc${i}s`, it[2], v => { it[2] = v; emit(); }),
      field(`pc${i}a`, alpha[i] ?? "", v => { while (alpha.length < items.length) alpha.push("255"); alpha[i] = v || "255"; emit(); }, { placeholder: "255" })]));
}
defWin("lighting", "Lighting", 430, p => {
  // the light tabs 1..6
  p.append(tabsRow([1, 2, 3, 4, 5, 6].map(i => [i, String(i) + (lightSec(i) ? " •" : "")]), lwSlot, i => { lwSlot = i; renderWin("lighting"); }));
  const s = lightSec(lwSlot);
  const lp = el("div", { class: "page2" });
  const specs = ["2", "4", "8", "16", "32", "64", "128", "256"];
  lp.append(el("div", { class: "btns" },
    s ? colorIn("lwCol", sv(s, "color", "#FFFFFF"), v => { putS(s, { color: v }); commit(); }, "Light colour") : null,
    lab("Diff:"), select("lwDf", s ? sv(s, "diffuse_func", "0") : "0", [["0", "Cos"], ["1", "Cos^2"], ["2", "Cos/2+"], ["3", "(Cos/2+)²"]], v => { putS(s, { diffuse_func: v }); commit(); },
      { disabled: !s, title: "Diffuse function, Cos or Cos^2 for hard shadows" }),
    lab("Spec:"), select("lwSp", s ? sv(s, "spec_power", "8") : "8", specs, v => { putS(s, { spec_power: v }); commit(); }, { disabled: !s, title: "Specular power" }),
    check("lwOn", !!s, c => lightOn(lwSlot, c), "On"),
    s ? check("lwHS", sv(s, "shadow", "true") !== "false", c => { putS(s, { shadow: c ? null : "false" }); commit(); }, "HS", "Use the hard shadow of this light, if calculated") : null));
  if (s) {
    lp.append(el("div", { class: "grid" }, lab("Intensity:", "Light amplitude, e.g. 1.4e-2"), field("lwAmp", sv(s, "amplitude", "1"), v => { putS(s, { amplitude: v }); commit(); }, { class: "n" })));
    const lt = lightType(s);
    lp.append(tabsRow(["Global light", "Positional light", "Lightmap"], lt, t => t !== lt && setLightType(s, t)));
    const tp = el("div", { class: "page2" });
    const vis = id => select(id, sv(s, "visible", "0"), ["0", "1", "2", "3", "4"], v => { putS(s, { visible: v === "0" ? null : v }); commit(); }, { title: "Visible light source and its shape" });
    if (lt === "Global light") {
      tp.append(el("div", { class: "tb" },
        ...trackbar("lwY", "Light Yangle", Math.round(+sv(s, "y_angle", "0")), -180, 180, v => { putS(s, { y_angle: v }); commit(); }, { def: 0 }),
        ...trackbar("lwX", "Light Xangle", Math.round(+sv(s, "x_angle", "0")), -180, 180, v => { putS(s, { x_angle: v }); commit(); }, { def: 0 })),
        el("div", { class: "btns" }, lab("Visible:"), vis("lwVis"),
          check("lwRel", sv(s, "relative_to_object") === "true", c => { putS(s, { relative_to_object: c }); commit(); }, "Rel. to object", "Fix the light to the scenery (animations)")));
    } else if (lt === "Positional light") {
      const pos = sv(s, "position", "0, 0, 0").split(",").map(x => x.trim());
      const pf = i => field("lwP" + i, pos[i] ?? "0", v => { pos[i] = v || "0"; putS(s, { position: pos.join(", ") }); commit(); });
      tp.append(el("div", { class: "grid" }, lab("Xpos"), pf(0), lab("Ypos"), pf(1), lab("Zpos"), pf(2)),
        el("div", { class: "btns" }, btn("mid", () => pickFromImage("Light position", async (u, v) => {
          const r = await pickAt(u, v); if (!r.pos) throw new Error("background picked");
          putS(s, { position: r.pos.join(", ") }); commit();
        }), { title: "Click on the object in the image to put the light there" }), lab("Visible:"), vis("lwVis2")));
    } else {
      const rot = sv(s, "map_rotation", "128, 128, 128").split(",").map(x => +x.trim());
      tp.append(el("div", { class: "grid" }, lab("Map number:"), field("lwMap", sv(s, "map", "1"), v => { putS(s, { map: v }); commit(); }, { class: "n" })),
        el("div", { class: "tb" }, ...["Xrot", "Yrot", "Zrot"].flatMap((n, i) => trackbar("lwMr" + i, n, rot[i] || 0, 0, 255, v => { rot[i] = v; putS(s, { map_rotation: rot.join(", ") }); commit(); }, { def: 128 }))),
        check("lwMRel", sv(s, "relative_to_object") === "true", c => { putS(s, { relative_to_object: c }); commit(); }, "M.rel.obj.", "Map orientation relative to the object"));
    }
    lp.append(tp);
    lp.append(el("div", { class: "btns" }, select("lwCopy", "", [["", "Copy this light to…"], ...[1, 2, 3, 4, 5, 6].filter(i => i !== lwSlot).map(i => [i, "Light " + i])], v => {
      if (!v) return;
      const t = lightSec(+v);
      if (t) model.sections.splice(model.sections.indexOf(t), 1);
      const c = JSON.parse(JSON.stringify(s)); putS(c, { slot: v });
      model.sections.splice(model.sections.indexOf(s) + 1, 0, c);
      commit();
    })));
  } else lp.append(el("div", { class: "hint" }, "This light is off."));
  p.append(lp);
  // gamma
  p.append(el("div", { class: "tb" }, ...tbG("lwGam", "Gamma", "gamma", 0, 63, { def: 32, title: "0.5 .. 2, 32 = 1", fmt: v => (Math.pow(2, (v - 32) / 32)).toFixed(2), parse: v => Math.round(32 + 32 * Math.log2(+v || 1)) }),
    el("span"), cG("lwI2", "internal_gamma2", "I2", "Internal gamma of 2 for the light calculations"), el("span")));
  // PageControl2
  p.append(tabsRow(["Object", "Ambient", "d.Fog", "Back pic"], lwTab2, t => { lwTab2 = t; store.set("lwTab2", t); renderWin("lighting"); }));
  const pg = el("div", { class: "page2" });
  if (lwTab2 === "Object") {
    const pal = palItems("palette_full");
    pg.append(el("div", { class: "tb" }, ...tbG("lwDif", "Diffuse", "diffuse", 0, 250, { def: 50 }), ...tbG("lwSpe", "Specular", "specular", 0, 350, { def: 50 })));
    pg.append(el("div", { class: "grid" },
      btn("Diff", () => { palEdit = palEdit === "pal" ? null : "pal"; renderWin("lighting"); }, { class: "lab", title: "Diffuse object colours, click to change them" }),
      el("div", { class: "palbar", style: `background:${palGradient(pal, x => x[1])}`, onclick: () => { palEdit = palEdit === "pal" ? null : "pal"; renderWin("lighting"); } }),
      btn("Spec", () => { palEdit = palEdit === "pal" ? null : "pal"; renderWin("lighting"); }, { class: "lab", title: "Specular object colours" }),
      el("div", { class: "palbar", style: `background:${palGradient(pal, x => x[2] || "#000")}`, onclick: () => { palEdit = palEdit === "pal" ? null : "pal"; renderWin("lighting"); } })));
    if (palEdit === "pal") pg.append(palEditor("pal"));
    pg.append(el("div", { class: "tb" }, ...tbG("lwCs", "Color start", "color_start", -30, 90, { def: 68 }), ...tbG("lwCe", "Color end", "color_end", -30, 90, { def: 72 }),
      ...tbG("lwCz", "Col. var. on Z", "color_var_z", -120, 360, { def: 0 })));
    pg.append(el("div", { class: "btns" }, cG("lwCc", "color_cycling", "Col cycling"), cG("lwC2", "color_on_otrap", "2. choice", "Second colour choice (see the main window's Coloring tab)"),
      check("lwNi", gv("color_interpolation") === "false", c => setG({ color_interpolation: c ? false : null }), "No ipol")));
    const ic = palItems("interior_colors");
    pg.append(el("div", { class: "grid" }, btn("Cuts", () => { palEdit = palEdit === "int" ? null : "int"; renderWin("lighting"); }, { class: "lab", title: "Object colours on cuts and 2D inside" }),
      el("div", { class: "palbar", style: `background:${palGradient(ic, x => x[1])}`, onclick: () => { palEdit = palEdit === "int" ? null : "int"; renderWin("lighting"); } })));
    if (palEdit === "int") pg.append(palEditor("interior"));
    pg.append(el("div", { class: "tb" }, ...tbG("lwIs", "Cuts start", "interior_start", 0, 120), ...tbG("lwIe", "Cuts end", "interior_end", 0, 120)));
    const dm = +gv("diffuse_map", "0") > 0;
    pg.append(check("lwDm", dm, c => setG({ diffuse_map: c ? 1 : null }), "Use a map for the diffuse color"));
    if (dm) {
      const off = gvec("diffuse_map_offset", 2).map(Number);
      pg.append(el("div", { class: "grid" }, lab("Map number:"), fG("lwDmN", "diffuse_map", { class: "n" })),
        el("div", { class: "tb" },
          ...trackbar("lwDmX", "Offset X", off[0], 0, 255, v => setG({ diffuse_map_offset: `${v}, ${off[1]}` }), { def: 128 }),
          ...trackbar("lwDmY", "Offset Y", off[1], 0, 255, v => setG({ diffuse_map_offset: `${off[0]}, ${v}` }), { def: 128 }),
          ...tbG("lwDmR", "Rotation", "diffuse_map_rotation", 0, 255, { def: 128 }), ...tbG("lwDmS", "Scale", "diffuse_map_scale", 0, 255, { def: 30 })),
        el("div", { class: "grid" }, lab("Mode:"), select("lwDmM", gv("diffuse_map_mode"), [["iterations_otrap", "its.trap"], ["normals", "norms"], ["wrap_sine", "wrap1"], ["wrap", "wrap2"]], v => setG({ diffuse_map_mode: v }))),
        cG("lwDmY2", "diffuse_map_brightness_only", "Combine map Y with diffuse colors"));
    }
  } else if (lwTab2 === "Ambient") {
    const lm = +gv("light_mode", "0");
    pg.append(el("div", { class: "grid" },
      lab("Amb (top, bottom):", "Ambient colours"), el("div", { class: "swatches" }, colG("lwAt", "ambient_top", "Ambient top colour"), colG("lwAb", "ambient_bottom", "Ambient bottom colour")),
      lab("Depth (top, bottom):", "Background / depth fog colours"), el("div", { class: "swatches" }, colG("lwDt", "depth_color", "Background top colour"), colG("lwDb", "depth_color2", "Background bottom colour")),
      lab("Depth gradient:", "Depth gradient function, only if not 'Relative to object'"), select("lwDg", gv("depth_func", "0"), [["0", "vertical"], ["1", "function 2"], ["2", "function 3"]], v => setG({ depth_func: v }))));
    pg.append(el("div", { class: "tb" },
      ...tbG("lwAm", "Ambient", "ambient", 0, 270, { def: 90 }), ...tbG("lwDe", "Depth", "depth_fog", 0, 240, { def: 27 }),
      ...tbG("lwAs", "Ambient shadow", "ambient_shadow", 0, 106, { def: 53 }), ...tbG("lwIl", "2nd reflection", "indirect_light", 0, 106, { def: 53 }),
      ...tbG("lwRo", "Roughness", "roughness", 0, 255, { def: 0, title: "Only working if 'Smooth normals' was bigger than zero" }),
      ...trackbar("lwDs", "Diffuse shadow", Math.round(gnum("diffuse_shadowing") * 256), 0, 255, v => setG({ diffuse_shadowing: +(v / 256).toFixed(4) }), { title: "Decrease also the direct light by the ambient occlusion" })));
    pg.append(el("div", { class: "btns" }, cG("lwFf", "far_fog", "Far depth fog", "Decreases the fog in near parts"),
      cG("lwRel2", "ambient_relative_to_object", "Relative to object"),
      check("lwM2", (lm & 1) !== 0, c => setG({ light_mode: c ? (lm | 1) : (lm & ~1) || null }), "Mode 2", "Different calculation of light combining")));
  } else if (lwTab2 === "d.Fog") {
    const o = +gv("dynfog_options", "0");
    pg.append(el("div", { class: "tb" }, ...tbG("lwFo", "Fog offset", "fog_offset", 0, 256, { def: 128 }), ...tbG("lwDy", "Dyn. fog", "dyn_fog", 0, 159, { def: 53, title: "53 = off" })),
      el("div", { class: "grid" }, lab("Dyn.fog colours:"), el("div", { class: "swatches" }, colG("lwFc", "dynfog_color", "Dynamic fog colour"), colG("lwFc2", "dynfog_color2", "Dynamic fog intense colour"))),
      el("div", { class: "btns" }, btn("set to 0", () => setG({ fog_offset: 0, dyn_fog: 53 }), { title: "Set the dynamic fog sliders to zero" }),
        check("lwBl", (o & 1) !== 0, c => setG({ dynfog_options: (c ? o | 1 : o & ~1) || null }), "Blend dFog", "Blend the dynamic fog instead of adding the light"),
        check("lwOa", (o & 2) !== 0, c => setG({ dynfog_options: (c ? o | 2 : o & ~2) || null }), "Only add light")));
  } else {
    const has = gv("background_image") !== "";
    const rot = gvec("background_rotation", 3).map(Number);
    pg.append(el("div", { class: "grid" }, lab("Background image:", "File name or map number in the maps folder"),
      field("lwBg", gv("background_image"), v => setG(v ? { background_image: v } : { background_image: null, background_rotation: null, background_direct: null, background_brightness: null, background_add_light: null }))));
    if (has) pg.append(el("div", { class: "tb" }, ...["Axis X", "Axis Y", "Axis Z"].flatMap((n, i) => trackbar("lwBr" + i, n, rot[i] || 0, 0, 255, v => { rot[i] = v; setG({ background_rotation: rot.join(", ") }); }, { def: 128 })),
      ...tbG("lwBi", "Int.", "background_brightness", 0, 255, { def: 40, title: "Intensity of the image" })),
      el("div", { class: "btns" }, check("lwBs", !gbool("background_direct"), c => setG({ background_direct: !c }), "As full background sphere"),
        cG("lwBa", "background_add_light", "Add to background depth (not blend)"), cG("lwBam", "background_ambient", "Use a small image as ambient color")));
  }
  p.append(pg);
});
