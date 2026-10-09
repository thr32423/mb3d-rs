
// ------------------------------------------------------------ start
async function loadFormulas() {
  const f = await api("/api/formulas");
  formulaNames = [...f.builtin, ...f.custom];
  formulaDE = new Map(formulaNames.map((n, i) => [n, f.de[i] ?? 0]));
  let dl = $("formulaList");
  if (!dl) { dl = el("datalist", { id: "formulaList" }); document.body.append(dl); }
  dl.textContent = "";
  for (const n of formulaNames) dl.append(el("option", { value: n }));
  if (isOpen("formulas")) renderWin("formulas");
}
(async () => {
  if (store.get("classic", false)) document.documentElement.classList.add("classic");
  try {
    await loadFormulas();
    applyState(await api("/api/state"), true);
    for (const id of store.get("open", [])) if (WINDOWS[id]) openWin(id);
    await sendView(true);
    msg("mb3d-rs editor. Open parameters (top left), edit them in the Formulas, Lighting and Post processing windows, and press \"Calculate 3D\".", "n");
  } catch (e) { showError(e); }
  poll();
})();
</script>
</body>
</html>
