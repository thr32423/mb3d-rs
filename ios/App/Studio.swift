import CoreGraphics
import Foundation
import ImageIO
import Observation
import SwiftUI
import UIKit

/// What the app shows: the open scene, its picture and the render running.
///
/// The camera is moved by gestures through a `Navigator`. While a finger
/// moves, quick small pictures follow it (one at a time, the newest view);
/// when the fingers lift, the full picture of the view is rendered.
@MainActor
@Observable
final class Studio {
    var examples: [URL] = Engine.examples()
    /// the user's own scenes (the app's Documents folder)
    var myScenes: [URL] = Library.scenes()
    /// the last full picture as a PNG file, for Share
    var pictureURL: URL?
    var selected: URL?
    var image: CGImage?
    var status = ""
    var progress: Double?
    var error: String?

    /// The settings of the current view, as the inspector edits them.
    var doc = SceneText("")
    /// a colour or a switch changed: the render waits for the next change
    @ObservationIgnored private var pending: Task<Void, Never>?

    /// The size of the file as opened (Save keeps it; the app renders
    /// smaller).
    private var fileSize = (width: 0, height: 0)
    /// The scene as it was opened (double tap goes back to its view).
    @ObservationIgnored private var opened: FractalScene?
    /// The scene of the current view, at the full render size.
    @ObservationIgnored private var scene: FractalScene?
    @ObservationIgnored private var navigator: Navigator?
    private var job: RenderJob?
    /// The job is a quick one (a moving camera's picture).
    @ObservationIgnored private var jobQuick = false
    /// The view changed since the quick picture running was started.
    @ObservationIgnored private var dirty = false
    /// A finger is on the picture.
    @ObservationIgnored private var moving = false
    /// A setting is being changed (a slider held, a colour chosen): quick
    /// pictures of the settings as edited follow it until `commit`.
    @ObservationIgnored private var previewing = false
    @ObservationIgnored private var peekNext: FractalScene?

    /// The value a finger is changing right now: the panels fade and the
    /// picture shows only its name and number.
    struct Adjusting {
        var title: LocalizedStringKey
        var value: String
        /// its place 0...1 on a slider (nil for a free number)
        var fraction: Double?
    }
    var adjusting: Adjusting? {
        didSet {
            // (a control that never reports its end must not leave the
            // panels faded: they come back after a few seconds of rest)
            fading?.cancel()
            guard adjusting != nil else { return }
            fading = Task { [weak self] in
                try? await Task.sleep(for: .seconds(4))
                guard !Task.isCancelled else { return }
                self?.adjusting = nil
            }
        }
    }
    @ObservationIgnored private var fading: Task<Void, Never>?

    /// The longest side of a full render (Quality; `0` the file's size).
    var renderSide = 1024 {
        didSet { if renderSide != oldValue { resizeForRender() } }
    }
    /// Steps back and forward (scene texts): every change and move.
    @ObservationIgnored private var history: [String] = []
    @ObservationIgnored private var future: [String] = []
    var canUndo = false
    var canRedo = false
    /// The longest side of the pictures while the camera moves.
    let quickSide = 320

    var rendering: Bool { job != nil }

    // MARK: the interface

    /// What the panel beside the picture shows.
    enum Panel: Hashable {
        case settings(SettingsPage)
        case animation
        case model
    }

    /// (`-page NAME` opens a settings group at the start: the build
    /// machine's screenshots of the interface)
    var panel: Panel? = Tour.page.map(Panel.settings)
    var showScenes = false
    var breeding = false
    var showHelp = false
    /// Nothing but the picture (the Pencil's double tap, Picture only).
    var pictureOnly = false
    /// The fingers look at the finished picture (zoom, move) and leave
    /// the camera alone.
    var inspecting = false
    /// A finger rests on the picture: the first picture since the scene
    /// was opened shows, to compare.
    var comparing = false
    /// Where the Pencil or the pointer hovers over the picture.
    var hover: CGPoint?
    /// The picture shown is a quick small one.
    var showsPreview = false
    /// Counts the full pictures finished and the focus set (for the taps
    /// one feels).
    var finishedCount = 0
    var focusCount = 0
    /// The first full picture since the scene was opened.
    var before: CGImage?

    /// The scene's directions of the picture's right, down and depth (the
    /// compass on the picture), following the camera.
    var axes: [Double] = [1, 0, 0, 0, 1, 0, 0, 0, 1]
    /// When the render's stage running began, for the time left.
    @ObservationIgnored private var stageBegan = Date()
    @ObservationIgnored private var stageName = ""

    /// About how long the stage running still takes (from its part done;
    /// nil while that is too little to tell).
    var timeLeft: Double? {
        guard busy, let p = progress, p > 0.05, p < 1 else { return nil }
        let spent = Date().timeIntervalSince(stageBegan)
        guard spent > 1.5 else { return nil }
        return spent * (1 - p) / p
    }

    /// A formula of the hybrid dragged onto the place of another.
    func placeFormula(_ id: Int, at target: Int) -> Int {
        let f = doc.formulas
        guard let i = f.firstIndex(where: { $0.id == id }), let j = f.firstIndex(where: { $0.id == target }), i != j
        else { return id }
        var entries = f.map(\.entries)
        let moved = entries.remove(at: i)
        entries.insert(moved, at: j)
        for (k, e) in entries.enumerated() {
            var s = f[k]
            s.entries = e
            doc.update(s)
        }
        commit()
        return f[j].id
    }

    /// The scene's text with another formula in a section (its default
    /// values), for the formula list's small pictures.
    func text(with name: String, in id: Int) -> String? {
        var d = doc
        guard var sec = d.sections.first(where: { $0.id == id }) else { return nil }
        let its = sec["iterations"] ?? "1"
        sec.entries = [.init(key: "name", value: name), .init(key: "iterations", value: its)]
            + Engine.formulaSettings(name).filter { $0.key != "iterations" }.map { .init(key: $0.key, value: $0.value) }
        d.update(sec)
        return d.text
    }

    /// A view kept to come back to (until another scene is opened).
    struct SavedView: Identifiable {
        let id = UUID()
        var number: Int
        var text: String
    }
    var views: [SavedView] = []

    func keepView() {
        guard let scene else { return }
        views.append(SavedView(number: (views.map(\.number).max() ?? 0) + 1, text: scene.text))
    }

    /// The scene of `text` (a kept view, a keyframe) becomes the view.
    func go(to text: String) {
        guard scene != nil, let s = try? FractalScene(text: text) else { return }
        remember()
        fitToRender(s)
        take(s)
        render()
    }

    /// The scenes opened last (newest first), of those still there.
    var recents: [URL] {
        let all = myScenes + examples
        return recentNames.compactMap { n in all.first { $0.lastPathComponent == n } }
    }
    private var recentNames: [String] = UserDefaults.standard.stringArray(forKey: "recent") ?? []

    private func noteRecent(_ url: URL) {
        var r = recentNames.filter { $0 != url.lastPathComponent }
        r.insert(url.lastPathComponent, at: 0)
        recentNames = Array(r.prefix(6))
        UserDefaults.standard.set(recentNames, forKey: "recent")
    }

    /// A settings group has something switched on (a dot on the rail).
    func isActive(_ p: SettingsPage) -> Bool {
        func on(_ key: String) -> Bool {
            guard let v = doc[key]?.lowercased() else { return false }
            return !["off", "none", "false", "no"].contains(v)
        }
        switch p {
        case .julia: return doc["julia"] == "true" || on("cut_x") || on("cut_y") || on("cut_z")
        case .fog: return !backgroundName.isEmpty || (doc.number("depth_fog") ?? 0) > 0 || (doc.number("dyn_fog") ?? 53) != 53
        case .effects: return on("dof") || on("shadows") || on("ao") || doc["mc_reflections"] == "true"
        case .finishing: return on("droste") || (doc.number("finish_exposure") ?? 0) != 0 || on("finish_tonemap")
            || (doc.component("glow", 0) ?? 0) > 0 || (doc.number("vignette") ?? 0) > 0
        case .ground: return on("ground") || on("clouds")
        default: return false
        }
    }

    /// A formula one place earlier (`by` -1) or later in the hybrid; gives
    /// the section it is in now.
    func moveFormula(_ id: Int, by: Int) -> Int {
        let f = doc.formulas
        guard let i = f.firstIndex(where: { $0.id == id }), f.indices.contains(i + by) else { return id }
        var (a, b) = (f[i], f[i + by])
        (a.entries, b.entries) = (b.entries, a.entries)
        doc.update(a)
        doc.update(b)
        commit()
        return b.id
    }

    /// A copy of one of the user's scenes beside it.
    func duplicate(_ url: URL) {
        let dest = Library.freeName(url.deletingPathExtension().lastPathComponent, url.pathExtension)
        try? FileManager.default.copyItem(at: url, to: dest)
        myScenes = Library.scenes()
    }

    func rename(_ url: URL, to name: String) {
        let clean = name.trimmingCharacters(in: .whitespacesAndNewlines).replacingOccurrences(of: "/", with: "-")
        guard !clean.isEmpty, clean != url.deletingPathExtension().lastPathComponent else { return }
        let dest = Library.freeName(clean, url.pathExtension)
        do {
            try FileManager.default.moveItem(at: url, to: dest)
            myScenes = Library.scenes()
            if selected == url { selected = dest }
        } catch {
            self.error = error.localizedDescription
        }
    }
    /// A full picture is being rendered (not the quick ones that follow a
    /// finger).
    var busy: Bool { job != nil && !jobQuick }

    func open(_ url: URL) {
        selected = url
        error = nil
        do {
            let s = try FractalScene.open(url)
            let (w, h) = s.size
            fileSize = (w, h)
            pictureURL = nil
            history = []
            future = []
            before = nil
            views = []
            inspecting = false
            noteRecent(url)
            fitToRender(s)
            opened = try s.copy()
            take(s)
            image = nil
            render()
        } catch {
            self.error = "\(error)"
        }
    }

    /// `s` becomes the current view: its values in the controls, a new
    /// navigator at it.
    private func take(_ s: FractalScene) {
        scene = s
        navigator = try? Navigator(s)
        if let navigator { axes = navigator.axes }
        doc = SceneText(s.text)
        canUndo = !history.isEmpty
        canRedo = !future.isEmpty
    }

    /// The render size for the file's size and the Quality.
    private func renderSize() -> (Int, Int) {
        let (w, h) = (max(fileSize.width, 1), max(fileSize.height, 1))
        guard renderSide > 0 else { return (w, h) }
        let k = Double(renderSide) / Double(max(w, h))
        return (max(16, Int((Double(w) * k).rounded())), max(16, Int((Double(h) * k).rounded())))
    }

    private func fitToRender(_ s: FractalScene) {
        let (w, h) = renderSize()
        s.resize(width: w, height: h)
    }

    private func resizeForRender() {
        guard let scene else { return }
        remember()
        fitToRender(scene)
        if let opened { fitToRender(opened) }
        take(scene)
        render()
    }

    /// The picture's shape: width / height (the long side kept).
    var aspect: Double {
        get { Double(max(fileSize.width, 1)) / Double(max(fileSize.height, 1)) }
        set {
            guard let scene, newValue > 0 else { return }
            let long = max(fileSize.width, fileSize.height, 16)
            fileSize = newValue >= 1 ? (long, max(16, Int((Double(long) / newValue).rounded())))
                                     : (max(16, Int((Double(long) * newValue).rounded())), long)
            remember()
            fitToRender(scene)
            if let opened { fitToRender(opened) }
            take(scene)
            render()
        }
    }

    /// The file's own size in pixels (Save keeps it).
    var pictureSize: (width: Int, height: Int) { fileSize }

    /// The scene of the current view (for Variations to breed from).
    var currentScene: FractalScene? { scene }

    /// A variation becomes the scene, at the render size.
    func takeVariation(_ v: FractalScene) {
        guard let s = try? v.copy() else { return }
        remember()
        fitToRender(s)
        take(s)
        render()
    }

    // MARK: undo

    /// Keeps the scene as it is now as a step back (before a change).
    private func remember() {
        guard let scene else { return }
        let t = scene.text
        if history.last != t { history.append(t) }
        if history.count > 60 { history.removeFirst() }
        future = []
        canUndo = true
        canRedo = false
    }

    private func restore(_ text: String) {
        do {
            let s = try FractalScene(text: text)
            take(s)
            render()
        } catch {
            self.error = "\(error)"
        }
    }

    func undo() {
        guard let t = history.popLast(), let scene else { return }
        future.append(scene.text)
        restore(t)
    }

    func redo() {
        guard let t = future.popLast(), let scene else { return }
        history.append(scene.text)
        restore(t)
    }

    var title: String {
        selected?.deletingPathExtension().lastPathComponent ?? "Mandelbulb3D"
    }

    // MARK: files

    /// A file from the Files app (or another app): copied into the app's
    /// own scenes and opened.
    func importFile(_ url: URL) {
        do {
            let u = try Library.bringIn(url)
            myScenes = Library.scenes()
            open(u)
        } catch {
            self.error = "\(error.localizedDescription)"
        }
    }

    /// The current view at the file's size, for saving.
    private func sceneToSave() throws -> FractalScene? {
        guard let scene else { return nil }
        let s = try scene.copy()
        if fileSize.width > 0, fileSize.height > 0 {
            s.resize(width: fileSize.width, height: fileSize.height)
        }
        return s
    }

    /// Saves the view among the user's scenes: over its own .m3p, else
    /// (an example, an .m3i) as a new .m3p of the same name.
    func save() {
        guard let selected else { return }
        do {
            guard let s = try sceneToSave() else { return }
            let own = Library.contains(selected) && selected.pathExtension.lowercased() == "m3p"
            let dest = own ? selected : Library.freeName(title, "m3p")
            try s.saveM3P(to: dest)
            myScenes = Library.scenes()
            self.selected = dest
            status = String(localized: "Saved as \(dest.lastPathComponent)")
        } catch {
            self.error = "\(error)"
        }
    }

    /// The view as an .m3p for Export (the Files app's save dialog).
    func exportDocument() -> M3PDocument? {
        do {
            guard let s = try sceneToSave() else { return nil }
            let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("export.m3p")
            try s.saveM3P(to: tmp)
            defer { try? FileManager.default.removeItem(at: tmp) }
            return M3PDocument(data: try Data(contentsOf: tmp))
        } catch {
            self.error = "\(error)"
            return nil
        }
    }

    func delete(_ url: URL) {
        try? FileManager.default.removeItem(at: url)
        myScenes = Library.scenes()
        if selected == url {
            selected = nil
            scene = nil
            opened = nil
            navigator = nil
            image = nil
            pictureURL = nil
            doc = SceneText("")
        }
    }

    // MARK: the settings

    /// The settings as edited become the scene (from its whole text, so
    /// formula and light sections change as shown), rendered again.
    func commit() {
        pending?.cancel()
        pending = nil
        previewing = false
        guard scene != nil else { return }
        do {
            let s = try FractalScene(text: doc.text)
            remember()
            take(s)
            render()
        } catch {
            self.error = "\(error)"
            if let scene { doc = SceneText(scene.text) }
        }
    }

    /// `commit` a moment after the last of quick changes (a colour wheel).
    func commitSoon() {
        previewSettings()
        pending?.cancel()
        pending = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(500))
            guard !Task.isCancelled else { return }
            self?.commit()
        }
    }

    /// A quick small picture of the settings as edited so far (the scene
    /// itself changes with `commit`): one at a time, the newest values.
    func previewSettings() {
        guard scene != nil, !moving else { return }
        previewing = true
        dirty = true
        if job == nil || !jobQuick {
            renderPreview()
        }
    }

    /// A quick picture of another scene (a frame of the animation while
    /// its strip is scrubbed), the newest asked for; `endPeek` brings the
    /// view's own picture back.
    func peek(_ s: FractalScene) {
        guard scene != nil, !moving else { return }
        let (w, h) = s.size
        let k = min(1, Double(quickSide) / Double(max(w, h, 1)))
        s.resize(width: max(16, Int(Double(w) * k)), height: max(16, Int(Double(h) * k)))
        previewing = true
        if job != nil && jobQuick {
            peekNext = s
        } else {
            start(s, quick: true)
        }
    }

    func endPeek() {
        peekNext = nil
        previewing = false
        dirty = false
        render()
    }

    private func renderPreview() {
        guard let scene, let s = try? FractalScene(text: doc.text) else { return }
        let (w, h) = scene.size
        let k = min(1, Double(quickSide) / Double(max(w, h, 1)))
        s.resize(width: max(16, Int(Double(w) * k)), height: max(16, Int(Double(h) * k)))
        dirty = false
        start(s, quick: true)
    }

    /// A global number (`whole`: written without decimals).
    func number(_ key: String, _ fallback: Double, whole: Bool = false) -> Binding<Double> {
        Binding(
            get: { self.doc.number(key) ?? fallback },
            set: { self.doc[key] = whole ? String(Int($0.rounded())) : String($0) }
        )
    }

    /// The detail (DE stop) at the file's size: the render's is scaled
    /// with its width.
    func detail() -> Binding<Double> {
        Binding(
            get: { (self.doc.number("de_stop") ?? 1) / self.detailScale },
            set: { self.doc["de_stop"] = String($0 * self.detailScale) }
        )
    }

    private var detailScale: Double {
        let (w, _) = renderSize()
        return fileSize.width > 0 ? Double(w) / Double(fileSize.width) : 1
    }

    /// Component `i` of a value like `a, b, c` (`fallback` the whole when
    /// missing).
    func component(_ key: String, _ i: Int, _ fallback: [Double]) -> Binding<Double> {
        Binding(
            get: { self.doc.component(key, i) ?? fallback[i] },
            set: { self.doc.setComponent(key, i, $0, fallback: fallback) }
        )
    }

    /// An effect that is on unless its key is missing or `off`: on writes
    /// `onValue`, applied shortly after.
    func effect(_ key: String, on onValue: String) -> Binding<Bool> {
        Binding(
            get: {
                guard let v = self.doc[key]?.lowercased() else { return false }
                return !["off", "none", "false", "no"].contains(v)
            },
            set: {
                if $0 { self.doc[key] = onValue } else { self.doc.switchOff(key) }
                self.commitSoon()
            }
        )
    }

    /// The formula the hybrid repeats from (1-based; `repeat_from` 0-based).
    func repeatFrom() -> Binding<Double> {
        Binding(
            get: { (self.doc.number("repeat_from") ?? 0) + 1 },
            set: { self.doc["repeat_from"] = String(max(0, Int($0.rounded()) - 1)) }
        )
    }

    /// The kind of surface detail (`detail = kind, strength, size, fineness`).
    func detailKind() -> Binding<String> {
        Binding(
            get: {
                let k = (self.doc["detail"] ?? "off").split(separator: ",").first.map {
                    $0.trimmingCharacters(in: .whitespaces).lowercased()
                } ?? "off"
                return ["rough", "stone", "grain"].contains(k) ? k : "off"
            },
            set: { kind in
                if kind == "off" {
                    self.doc["detail"] = "off"
                } else {
                    var rest = (self.doc["detail"] ?? "").split(separator: ",").dropFirst()
                        .map { $0.trimmingCharacters(in: .whitespaces) }
                    if rest.count != 3 { rest = ["1", "0.05", "0.5"] }
                    self.doc["detail"] = ([kind] + rest).joined(separator: ", ")
                }
                self.commitSoon()
            }
        )
    }

    /// The ground plane's axis (`ground = axis, position`).
    func groundAxis() -> Binding<String> {
        Binding(
            get: {
                (self.doc["ground"] ?? "y").split(separator: ",").first
                    .map { $0.trimmingCharacters(in: .whitespaces).lowercased() } ?? "y"
            },
            set: { axis in
                self.doc["ground"] = "\(axis), \(SceneText.format(self.groundPosition().wrappedValue))"
                self.commitSoon()
            }
        )
    }

    func groundPosition() -> Binding<Double> {
        Binding(
            get: { self.doc.component("ground", 1) ?? 1.2 },
            set: { self.doc["ground"] = "\(self.groundAxis().wrappedValue), \(SceneText.format($0))" }
        )
    }

    /// The cloud layer's heights (`clouds = axis, from, to`).
    func cloudHeight(_ i: Int) -> Binding<Double> {
        Binding(
            get: { self.doc.component("clouds", i + 1) ?? [-1.2, -2.2][i] },
            set: { v in
                let axis = (self.doc["clouds"] ?? "y").split(separator: ",").first
                    .map { $0.trimmingCharacters(in: .whitespaces) } ?? "y"
                var h = [self.doc.component("clouds", 1) ?? -1.2, self.doc.component("clouds", 2) ?? -2.2]
                h[i] = v
                self.doc["clouds"] = "\(axis), \(SceneText.format(h[0])), \(SceneText.format(h[1]))"
            }
        )
    }

    /// A global `true`/`false`, applied shortly after.
    func flag(_ key: String) -> Binding<Bool> {
        Binding(
            get: { self.doc[key] == "true" },
            set: { self.doc[key] = $0 ? "true" : "false"; self.commitSoon() }
        )
    }

    /// A global word from a list, applied shortly after.
    func choice(_ key: String, _ fallback: String) -> Binding<String> {
        Binding(
            get: { self.doc[key] ?? fallback },
            set: { self.doc[key] = $0; self.commitSoon() }
        )
    }

    /// A global `#RRGGBB` colour, applied shortly after.
    func color(_ key: String) -> Binding<CGColor> {
        Binding(
            get: { Self.cgColor(self.doc[key]) },
            set: { if let h = Self.hex($0) { self.doc[key] = h; self.commitSoon() } }
        )
    }

    /// A number of a formula or light section.
    func sectionNumber(_ id: Int, _ key: String) -> Binding<Double> {
        Binding(
            get: { self.doc.sections.first { $0.id == id }?[key].flatMap { Double($0) } ?? 0 },
            set: { v in
                guard var sec = self.doc.sections.first(where: { $0.id == id }) else { return }
                sec[key] = key == "iterations" || key == "slot" ? String(Int(v.rounded())) : String(v)
                self.doc.update(sec)
            }
        )
    }

    /// A switch of a section (missing means `fallback`), applied shortly
    /// after.
    func sectionFlag(_ id: Int, _ key: String, _ fallback: Bool) -> Binding<Bool> {
        Binding(
            get: {
                guard let v = self.doc.sections.first(where: { $0.id == id })?[key] else { return fallback }
                return v == "true"
            },
            set: { on in
                guard var sec = self.doc.sections.first(where: { $0.id == id }) else { return }
                sec[key] = on ? "true" : "false"
                self.doc.update(sec)
                self.commitSoon()
            }
        )
    }

    /// A word of a section, applied shortly after.
    func sectionChoice(_ id: Int, _ key: String, _ fallback: String) -> Binding<String> {
        Binding(
            get: { self.doc.sections.first { $0.id == id }?[key] ?? fallback },
            set: { v in
                guard var sec = self.doc.sections.first(where: { $0.id == id }) else { return }
                sec[key] = v
                self.doc.update(sec)
                self.commitSoon()
            }
        )
    }

    /// Another formula after the last (up to six), an Integer Power to
    /// start with.
    func addFormula() {
        guard doc.formulas.count < 6 else { return }
        let name = "Integer Power"
        doc.insert("formula", [.init(key: "name", value: name), .init(key: "iterations", value: "1")]
            + Engine.formulaSettings(name).filter { $0.key != "iterations" }.map { .init(key: $0.key, value: $0.value) })
        commit()
    }

    /// Another light (up to six) in the first free slot, white from the
    /// upper left.
    func addLight() {
        let used = Set(doc.lights.compactMap { $0["slot"].flatMap { Int($0) } })
        guard let slot = (1...6).first(where: { !used.contains($0) }) else { return }
        doc["lights"] = nil
        doc.insert("light", [
            .init(key: "slot", value: String(slot)), .init(key: "color", value: "#FFFFFF"),
            .init(key: "x_angle", value: "-30"), .init(key: "y_angle", value: "30"),
            .init(key: "amplitude", value: "1"), .init(key: "spec_power", value: "8"),
            .init(key: "diffuse_func", value: "0"), .init(key: "relative_to_object", value: "false"),
        ])
        commit()
    }

    /// Removes a formula (the last stays) or a light (without any, the
    /// scene says `lights = none`).
    func removeSection(_ id: Int) {
        guard let sec = doc.sections.first(where: { $0.id == id }) else { return }
        if sec.kind == "formula" && doc.formulas.count < 2 { return }
        doc.remove(id)
        if sec.kind == "light" && doc.lights.isEmpty { doc["lights"] = "none" }
        commit()
    }

    // MARK: background picture and own formulas

    /// The background picture's file name (in a map folder), "" without.
    var backgroundName: String { doc["background_image"] ?? "" }

    /// A picture (JPEG or PNG data from Photos) as the background.
    func setBackground(imageData: Data) {
        guard let img = UIImage(data: imageData), let jpeg = img.jpegData(compressionQuality: 0.92) else {
            error = String(localized: "The picture could not be read.")
            return
        }
        let name = "Photo \(Self.stamp()).jpg"
        do {
            try jpeg.write(to: Library.backgrounds.appendingPathComponent(name))
            useBackground(name)
        } catch {
            self.error = "\(error.localizedDescription)"
        }
    }

    /// A picture file (.jpg, .png, .hdr from the Files app) as the background.
    func setBackground(file: URL) {
        do {
            guard let dest = try Library.copy([file], into: Library.backgrounds).first else { return }
            useBackground(dest.lastPathComponent)
        } catch {
            self.error = "\(error.localizedDescription)"
        }
    }

    private func useBackground(_ name: String) {
        doc["background_image"] = name
        if doc["background_direct"] == nil { doc["background_direct"] = "true" }
        if doc["background_brightness"] == nil { doc["background_brightness"] = "40" }
        commit()
    }

    func removeBackground() {
        for k in ["background_image", "background_rotation", "background_direct", "background_brightness",
                  "background_add_light", "background_smooth", "background_geographic", "background_fit_borders",
                  "background_ambient"] {
            doc[k] = nil
        }
        commit()
    }

    /// "As full background sphere" (`background_direct = false`).
    func backgroundSphere() -> Binding<Bool> {
        Binding(
            get: { self.doc["background_direct"] == "false" },
            set: { self.doc["background_direct"] = $0 ? "false" : "true"; self.commitSoon() }
        )
    }

    /// The user's formula files.
    var ownFormulas: [String] = Library.ownFormulas()

    /// Formula files (.m3f) into the user's formula folder.
    func importFormulas(_ urls: [URL]) {
        do {
            let done = try Library.copy(urls, into: Library.formulas)
            Engine.reloadFormulas()
            ownFormulas = Library.ownFormulas()
            status = done.count == 1 ? String(localized: "1 formula added") : String(localized: "\(done.count) formulas added")
        } catch {
            self.error = "\(error.localizedDescription)"
        }
    }

    func removeFormula(_ name: String) {
        try? FileManager.default.removeItem(at: Library.formulas.appendingPathComponent(name).appendingPathExtension("m3f"))
        Engine.reloadFormulas()
        ownFormulas = Library.ownFormulas()
    }

    private static func stamp() -> String {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd HH.mm.ss"
        return f.string(from: Date())
    }

    // MARK: the palette

    struct Stop: Identifiable {
        let id: Int
        var position: Int
        var diffuse: String
        var specular: String
    }

    /// The colour stops of `palette_full` (`position:#diffuse:#specular`).
    var stops: [Stop] {
        (doc["palette_full"] ?? "").split(separator: ",").enumerated().compactMap { i, part in
            let f = part.trimmingCharacters(in: .whitespaces).split(separator: ":")
            guard f.count == 3, let p = Int(f[0]) else { return nil }
            return Stop(id: i, position: p, diffuse: String(f[1]), specular: String(f[2]))
        }
    }

    private func setStops(_ s: [Stop]) {
        doc["palette_full"] = s.map { "\($0.position):\($0.diffuse):\($0.specular)" }.joined(separator: ", ")
    }

    /// A stop dragged along the palette: kept between its neighbours.
    func moveStop(_ id: Int, to position: Int, limit: Int) {
        var all = stops
        guard let i = all.firstIndex(where: { $0.id == id }) else { return }
        let low = i > 0 ? all[i - 1].position : 0
        let high = i + 1 < all.count ? all[i + 1].position : max(limit, low)
        let p = min(max(position, low), max(high, low))
        guard p != all[i].position else { return }
        all[i].position = p
        setStops(all)
        previewSettings()
    }

    func stopColor(_ id: Int, specular: Bool) -> Binding<CGColor> {
        Binding(
            get: {
                let s = self.stops.first { $0.id == id }
                return Self.cgColor(specular ? s?.specular : s?.diffuse)
            },
            set: { c in
                guard let h = Self.hex(c) else { return }
                var all = self.stops
                guard let i = all.firstIndex(where: { $0.id == id }) else { return }
                if specular { all[i].specular = h } else { all[i].diffuse = h }
                self.setStops(all)
                self.commitSoon()
            }
        )
    }

    // MARK: the picture

    /// A tap on the picture at `point` of a view `size` big: with depth of
    /// field on, the focus goes to what is there.
    func tap(at point: CGPoint, in size: CGSize) {
        guard let scene, let dof = doc["dof"], dof != "off" else {
            if scene != nil { status = String(localized: "Switch on depth of field (Effects) to focus with a tap") }
            return
        }
        let (w, h) = scene.size
        let k = min(size.width / Double(w), size.height / Double(h))
        guard k > 0 else { return }
        let x = Int(((point.x - (size.width - Double(w) * k) / 2) / k).rounded(.down))
        let y = Int(((point.y - (size.height - Double(h) * k) / 2) / k).rounded(.down))
        guard x >= 0, y >= 0, x < w, y < h else { return }
        guard let z = scene.focus(x: x, y: y) else {
            status = String(localized: "Nothing there to focus on")
            return
        }
        doc["dof_focus"] = String(z)
        commit()
        status = String(localized: "Focus set")
        focusCount += 1
    }

    /// The parameters as Mandelbulb3D text on the clipboard.
    func copyParameters() {
        guard let s = try? sceneToSave(), let t = s.mb3dText(title: title) else { return }
        UIPasteboard.general.string = t
        status = String(localized: "Parameters copied")
    }

    /// Parameters from the clipboard (MB3D text or a scene text) become a
    /// new scene among the user's.
    func pasteParameters() {
        guard let t = UIPasteboard.general.string, !t.isEmpty else {
            error = String(localized: "The clipboard holds no text.")
            return
        }
        do {
            let s = try FractalScene(text: t)
            let dest = Library.freeName("Pasted", "m3p")
            try s.saveM3P(to: dest)
            myScenes = Library.scenes()
            open(dest)
        } catch {
            self.error = String(localized: "The clipboard holds no parameters: \(String(describing: error))")
        }
    }

    /// The last full picture into the Photos library.
    func saveToPhotos() {
        guard let image else { return }
        UIImageWriteToSavedPhotosAlbum(UIImage(cgImage: image), nil, nil, nil)
        status = String(localized: "Saved to Photos")
    }

    /// A colour of a light section, applied shortly after.
    func sectionColor(_ id: Int, _ key: String) -> Binding<CGColor> {
        Binding(
            get: { Self.cgColor(self.doc.sections.first { $0.id == id }?[key]) },
            set: { c in
                guard let h = Self.hex(c), var sec = self.doc.sections.first(where: { $0.id == id }) else { return }
                sec[key] = h
                self.doc.update(sec)
                self.commitSoon()
            }
        )
    }

    /// Another formula in a section: its own default values, the
    /// iterations kept.
    func changeFormula(_ id: Int, to name: String) {
        guard var sec = doc.sections.first(where: { $0.id == id }), sec["name"] != name else { return }
        let its = sec["iterations"] ?? "1"
        sec.entries = [.init(key: "name", value: name), .init(key: "iterations", value: its)]
            + Engine.formulaSettings(name).filter { $0.key != "iterations" }.map { .init(key: $0.key, value: $0.value) }
        doc.update(sec)
        commit()
    }

    /// The formula picker's names (the section's own first if unknown).
    func formulaNames(including current: String?) -> [String] {
        let all = Engine.formulaNames
        if let current, !current.isEmpty, !all.contains(current) { return [current] + all }
        return all
    }

    static func cgColor(_ hex: String?) -> CGColor {
        let (r, g, b) = hex.flatMap(HexColor.rgb) ?? (1, 1, 1)
        return CGColor(srgbRed: r, green: g, blue: b, alpha: 1)
    }

    static func hex(_ c: CGColor) -> String? {
        guard let srgb = CGColorSpace(name: CGColorSpace.sRGB),
              let k = c.converted(to: srgb, intent: .defaultIntent, options: nil)?.components, k.count >= 3
        else { return nil }
        return HexColor.hex(k[0], k[1], k[2])
    }

    // MARK: the camera

    /// A one-finger drag of (dx, dy) points on a picture `height` points
    /// high: the camera looks around (as the desktop Navigator's drag).
    func look(dx: Double, dy: Double, height: Double) {
        guard let navigator, height > 0 else { return }
        let k = navigator.fov / height
        navigator.turn(pitch: -dy * k, yaw: dx * k, roll: 0)
        moved()
    }

    /// A two-finger drag of (dx, dy) points: the camera slides.
    func slide(dx: Double, dy: Double, height: Double) {
        guard let navigator, height > 0 else { return }
        navigator.slide(dx: dx / height, dy: dy / height)
        moved()
    }

    /// A pinch by `scale` (since the last call): forward when spreading.
    func pinch(scale: Double) {
        guard let navigator, scale > 0 else { return }
        let percent = max(-90, min(90, log(scale) * 120))
        if abs(percent) < 0.05 { return }
        navigator.fly(percent: percent)
        moved()
    }

    /// A two-finger twist by `radians` (since the last call), clockwise
    /// positive: the camera rolls with it.
    func twist(radians: Double) {
        guard let navigator else { return }
        navigator.turn(pitch: 0, yaw: 0, roll: -radians * 180 / .pi)
        moved()
    }

    /// The fingers left the picture: the full picture of the view.
    func gestureEnded() {
        moving = false
        guard let navigator, let scene else { return }
        let (w, h) = scene.size
        if let s = try? navigator.scene(quick: false, width: w, height: h) {
            self.scene = s
            doc = SceneText(s.text)
        }
        if job == nil || jobQuick {
            if job == nil { render() }
            // (else the quick picture running starts the full one when done)
        }
    }

    /// A double tap at `point` of a view `size` big: the camera turns to
    /// what is there and flies half the way to it.
    func flyTo(_ point: CGPoint, in size: CGSize) {
        guard let navigator, let scene else { return }
        let (w, h) = scene.size
        let k = min(size.width / Double(w), size.height / Double(h))
        guard k > 0 else { return }
        remember()
        // (as a drag that brings the point to the middle)
        let d = navigator.fov / (Double(h) * k)
        navigator.turn(pitch: (point.y - size.height / 2) * d, yaw: -(point.x - size.width / 2) * d, roll: 0)
        navigator.fly(percent: 50)
        axes = navigator.axes
        if let s = try? navigator.scene(quick: false, width: w, height: h) {
            self.scene = s
            doc = SceneText(s.text)
        }
        render()
    }

    /// Back to the view of the file as it was opened.
    func resetView() {
        guard let opened, let s = try? opened.copy() else { return }
        remember()
        take(s)
        render()
    }

    private func moved() {
        if !moving { remember() }
        moving = true
        previewing = false
        if let navigator { axes = navigator.axes }
        dirty = true
        if job == nil || !jobQuick {
            renderQuick()
        }
    }

    // MARK: rendering

    private func renderQuick() {
        guard let navigator, let scene else { return }
        let (w, h) = scene.size
        let k = Double(quickSide) / Double(max(w, h, 1))
        guard let s = try? navigator.scene(quick: true, width: max(16, Int(Double(w) * k)), height: max(16, Int(Double(h) * k)))
        else { return }
        dirty = false
        start(s, quick: true)
    }

    func render() {
        guard let scene, let copy = try? scene.copy() else { return }
        start(copy, quick: false)
    }

    private func start(_ s: FractalScene, quick: Bool) {
        job?.stop()
        let job = RenderJob { stage, part in
            Task { @MainActor [weak self] in
                guard let self, self.job != nil, !self.jobQuick else { return }
                if stage != self.stageName {
                    self.stageName = stage
                    self.stageBegan = Date()
                }
                self.status = stage
                self.progress = part
            }
        }
        self.job = job
        jobQuick = quick
        if !quick {
            stageName = ""
            stageBegan = Date()
            progress = 0
            status = String(localized: "Starting")
        } else {
            progress = nil
        }
        Task.detached(priority: .userInitiated) {
            let result = Result { try job.render(s) }
            await MainActor.run { [weak self] in
                self?.finished(job, result)
            }
        }
    }

    private func finished(_ done: RenderJob, _ result: Result<(image: CGImage, seconds: Double), Error>) {
        guard job === done else { return }
        let wasQuick = jobQuick
        job = nil
        progress = nil
        switch result {
        case .success(let r):
            image = r.image
            showsPreview = wasQuick
            status = wasQuick ? "" : "\(r.image.width) × \(r.image.height) · " + String(format: "%.2f s", r.seconds)
            if !wasQuick {
                if before == nil { before = r.image }
                finishedCount += 1
                Snapshots.save(r.image)
                let url = FileManager.default.temporaryDirectory.appendingPathComponent(title).appendingPathExtension("png")
                pictureURL = PNG.write(r.image, to: url) ? url : nil
            }
        case .failure(let e):
            status = ""
            if !done.isStopped { error = "\(e)" }
        }
        if wasQuick {
            // the newest view while moving, the full picture once the
            // fingers are up
            if let next = peekNext {
                peekNext = nil
                start(next, quick: true)
            } else if dirty {
                if previewing { renderPreview() } else { renderQuick() }
            } else if !moving && !previewing {
                render()
            }
        }
    }

    func stop() {
        job?.stop()
    }

    /// The animation's keyframes and settings (kept while the app runs).
    let director = Director()
    /// The 3D model export's settings and result.
    let sculptor = Sculptor()

    /// Stereo pairs side by side for cross-eyed viewing (else parallel).
    var crossEyed = true

    /// The two eyes' pictures (the scene's `stereo` key), side by side.
    func renderStereoPair() {
        guard let scene, let l = try? FractalScene(text: scene.text), let r = try? FractalScene(text: scene.text) else { return }
        do {
            try l.set("stereo = left")
            try r.set("stereo = right")
        } catch {
            self.error = "\(error)"
            return
        }
        job?.stop()
        let job = RenderJob { stage, part in
            Task { @MainActor [weak self] in
                guard let self, self.job != nil else { return }
                self.status = stage
                self.progress = part
            }
        }
        self.job = job
        jobQuick = false
        progress = 0
        status = String(localized: "Left eye")
        let cross = crossEyed
        Task.detached(priority: .userInitiated) {
            let result: Result<CGImage, Error>
            do {
                let left = try job.render(l).image
                await MainActor.run { [weak self] in self?.status = String(localized: "Right eye") }
                let right = try job.render(r).image
                // cross-eyed: the right eye's picture on the left
                result = .success(try Studio.sideBySide(cross ? right : left, cross ? left : right))
            } catch {
                result = .failure(error)
            }
            await MainActor.run { [weak self] in
                guard let self, self.job === job else { return }
                self.job = nil
                self.progress = nil
                switch result {
                case .success(let img):
                    self.image = img
                    self.status = cross ? "Stereo pair (cross-eyed)" : "Stereo pair (parallel)"
                    Snapshots.save(img, name: "stereo")
                    let url = FileManager.default.temporaryDirectory
                        .appendingPathComponent("\(self.title) stereo").appendingPathExtension("png")
                    self.pictureURL = PNG.write(img, to: url) ? url : nil
                case .failure(let e):
                    self.status = ""
                    if !job.isStopped { self.error = "\(e)" }
                }
            }
        }
    }

    nonisolated private static func sideBySide(_ a: CGImage, _ b: CGImage) throws -> CGImage {
        let (w, h) = (a.width + b.width, max(a.height, b.height))
        guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)
        else { throw EngineError(String(localized: "the pair could not be made")) }
        ctx.draw(a, in: CGRect(x: 0, y: 0, width: a.width, height: a.height))
        ctx.draw(b, in: CGRect(x: a.width, y: 0, width: b.width, height: b.height))
        guard let img = ctx.makeImage() else { throw EngineError(String(localized: "the pair could not be made")) }
        return img
    }

    /// The view as a 360° panorama: the panorama lens, a 2:1 picture.
    func makePanorama() {
        doc["optic"] = "2"
        commit()
        aspect = 2
    }

    /// The last picture as a 360° photo (JPEG with the tags Facebook and
    /// Google Photos look for) for Share.
    func panoramaPhoto() -> URL? {
        guard let image, image.width == 2 * image.height else {
            error = String(localized: "Make a 360° panorama first (Picture ▸ 360° panorama): its picture is twice as wide as high.")
            return nil
        }
        let data = NSMutableData()
        guard let dest = CGImageDestinationCreateWithData(data, "public.jpeg" as CFString, 1, nil) else { return nil }
        CGImageDestinationAddImage(dest, image, [kCGImageDestinationLossyCompressionQuality: 0.92] as CFDictionary)
        guard CGImageDestinationFinalize(dest) else { return nil }
        var n = 0
        let tagged: Data? = (data as Data).withUnsafeBytes { raw in
            guard let p = raw.bindMemory(to: UInt8.self).baseAddress,
                  let out = mb3d_jpeg_360(p, raw.count, UInt32(image.width), UInt32(image.height), &n)
            else { return nil }
            defer { mb3d_bytes_free(out, n) }
            return Data(bytes: out, count: n)
        }
        guard let tagged else {
            error = Engine.lastError()
            return nil
        }
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("\(title)_360").appendingPathExtension("jpg")
        do {
            try tagged.write(to: url)
            return url
        } catch {
            self.error = "\(error.localizedDescription)"
            return nil
        }
    }

    /// A Monte Carlo render of the view: pass after pass (up to
    /// `maxPasses`), each picture shown, until Stop.
    func renderMonteCarlo(maxPasses: Int = 64) {
        guard let scene, let s = try? scene.copy() else { return }
        job?.stop()
        let job = RenderJob { stage, part in
            Task { @MainActor [weak self] in
                guard let self, self.job != nil else { return }
                self.status = String(localized: "Monte Carlo, \(stage.lowercased())")
                self.progress = part
            }
        }
        self.job = job
        jobQuick = false
        progress = 0
        status = String(localized: "Monte Carlo")
        Task.detached(priority: .userInitiated) {
            var failure: Error?
            do {
                let mc = try MonteCarloRender(s)
                for _ in 0..<maxPasses {
                    if job.isStopped { break }
                    try mc.pass(job)
                    let img = try mc.image()
                    let n = mc.passes
                    await MainActor.run { [weak self] in
                        guard let self, self.job === job else { return }
                        self.image = img
                        self.status = n == 1 ? String(localized: "Monte Carlo, 1 pass") : String(localized: "Monte Carlo, \(n) passes")
                    }
                }
            } catch {
                failure = error
            }
            let error = failure
            await MainActor.run { [weak self] in
                guard let self, self.job === job else { return }
                self.job = nil
                self.progress = nil
                if let error, !job.isStopped { self.error = "\(error)" }
                if let img = self.image {
                    Snapshots.save(img, name: "montecarlo")
                    let url = FileManager.default.temporaryDirectory.appendingPathComponent(self.title).appendingPathExtension("png")
                    self.pictureURL = PNG.write(img, to: url) ? url : nil
                }
            }
        }
    }

    // MARK: the build machine's run

    /// `-demo`: after the first picture, moves the camera as gestures
    /// would (fly in, look around, slide, roll), each move followed by its
    /// full picture (saved with `-snapshots`); then changes settings
    /// (colours, fog, the formula) and saves the scene.
    func demo() async {
        let steps: [(String, (Studio) -> Void)] = [
            ("fly in", { $0.pinch(scale: 1.4) }),
            ("fly in", { $0.pinch(scale: 1.4) }),
            ("look right", { $0.look(dx: 120, dy: 0, height: 768) }),
            ("look up", { $0.look(dx: 0, dy: -80, height: 768) }),
            ("slide", { $0.slide(dx: 100, dy: 40, height: 768) }),
            ("roll", { $0.twist(radians: 0.3) }),
            ("fly back", { $0.pinch(scale: 0.6) }),
        ]
        await waitIdle()
        for (name, step) in steps {
            status = name
            step(self)
            gestureEnded()
            try? await Task.sleep(for: .milliseconds(300))
            await waitIdle()
        }
        // the settings: light colour, background and fog, then another
        // formula (its defaults), then Save
        let settings: [(String, (Studio) -> Void)] = [
            ("colours and fog", { s in
                s.doc["depth_color"] = "#E07020"
                s.doc["depth_color2"] = "#301040"
                s.doc["depth_fog"] = "40"
                if var l = s.doc.lights.first {
                    l["color"] = "#80C0FF"
                    s.doc.update(l)
                }
                s.commit()
            }),
            ("formula", { s in
                if let f = s.doc.formulas.first { s.changeFormula(f.id, to: "Integer Power") }
            }),
        ]
        for (name, step) in settings {
            status = name
            step(self)
            try? await Task.sleep(for: .milliseconds(300))
            await waitIdle()
        }
        save()
        // phase 1: effects, a tap to focus, undo, another shape, the
        // clipboard (pictures 10 to 14)
        let more: [(String, (Studio) -> Void)] = [
            ("effects", { s in
                s.undo()  // back to the box (before the formula swap)
            }),
            ("ground and finishing", { s in
                s.doc["ground"] = "y, 1.2"
                s.doc["ground_reflect"] = "0.4"
                s.doc["glow"] = "0.6, 0.02, 0.8"
                s.doc["vignette"] = "0.4"
                s.doc["dof"] = "sorted"
                s.doc["dof_aperture"] = "0.2"
                s.commit()
            }),
            ("focus", { s in
                guard let (w, h) = s.scene?.size else { return }
                s.tap(at: CGPoint(x: Double(w) * 0.3, y: Double(h) * 0.3), in: CGSize(width: w, height: h))
            }),
            ("16:9", { s in s.aspect = 16.0 / 9 }),
            ("paste", { s in
                s.copyParameters()
                s.pasteParameters()
            }),
        ]
        for (name, step) in more {
            status = name
            step(self)
            try? await Task.sleep(for: .milliseconds(300))
            await waitIdle()
        }
        // (depth of field and ground off again: the variations sharp)
        doc.switchOff("dof")
        doc.switchOff("ground")
        commit()
        try? await Task.sleep(for: .milliseconds(300))
        await waitIdle()
        // phase 2: a generation of variations, the third taken (picture 15)
        status = String(localized: "variations")
        let breeder = Breeder()
        if let s = scene { breeder.breed(from: s) }
        try? await Task.sleep(for: .milliseconds(300))
        while breeder.busy { try? await Task.sleep(for: .milliseconds(300)) }
        print("MB3D demo: \(breeder.members.count) variations, status '\(breeder.status)'")
        for m in breeder.members {
            if let img = m.image { Snapshots.save(img, name: "variation-\(m.label)") }
        }
        if breeder.members.count > 2 { takeVariation(breeder.members[2].scene) }
        try? await Task.sleep(for: .milliseconds(300))
        await waitIdle()
        // phase 5: Droste, a stereo pair, a 360 photo, a turntable video,
        // a 3D model (their files copied to Documents for the workflow)
        let keep = { (u: URL?) in
            guard let u else { return }
            let dest = Library.folder.appendingPathComponent("demo-" + u.lastPathComponent)
            try? FileManager.default.removeItem(at: dest)
            try? FileManager.default.copyItem(at: u, to: dest)
        }
        status = String(localized: "Droste")
        doc["droste"] = "0.5, 0.5, 0.15, 0.45"
        doc["droste_twist"] = "1"
        commit()
        try? await Task.sleep(for: .milliseconds(300))
        await waitIdle()
        if let img = image { Snapshots.save(img, name: "droste") }
        doc.switchOff("droste")
        commit()
        try? await Task.sleep(for: .milliseconds(300))
        await waitIdle()
        status = String(localized: "stereo")
        renderStereoPair()
        try? await Task.sleep(for: .milliseconds(300))
        await waitIdle()
        status = String(localized: "360")
        makePanorama()
        try? await Task.sleep(for: .milliseconds(300))
        await waitIdle()
        keep(panoramaPhoto())
        status = String(localized: "turntable")
        director.turntable = true
        director.turntableSeconds = 2
        director.fps = 12
        director.side = 320
        director.render(turntableOf: scene)
        try? await Task.sleep(for: .milliseconds(300))
        while director.busy { try? await Task.sleep(for: .milliseconds(300)) }
        print("MB3D demo: video \(director.video?.lastPathComponent ?? "none"), \(director.status) \(director.error ?? "")")
        keep(director.video)
        // keyframes: the view and a closer one, the second's zoom changed
        // on its curve, rendered with a motion blur of 4 pictures
        status = String(localized: "motion blur")
        if let s = scene {
            director.turntable = false
            director.keys = []
            director.add(s, thumbnail: image)
            var t = SceneText(s.text)
            t["zoom"] = SceneText.format((t.number("zoom") ?? 1) * 2)
            if let closer = try? FractalScene(text: t.text) { director.add(closer, thumbnail: image) }
            director.keys[0].frames = 6
            if let film = try? director.film(), let zi = Film.curveNames.firstIndex(of: "zoom"),
               let before = film.curve(zi)?.keys.last?.value {
                film.setCurveKey(zi, key: 1, value: before * 2)
                if let k = try? film.keyframe(1) { director.keys[1].text = k.text }
                let after = (try? director.film())?.curve(zi)?.keys.last?.value
                print("MB3D demo: curve zoom of keyframe 2 \(before) -> \(after ?? .nan)")
            }
            director.motionBlur = 4
            director.side = 240
            director.render(turntableOf: nil)
            try? await Task.sleep(for: .milliseconds(300))
            while director.busy { try? await Task.sleep(for: .milliseconds(300)) }
            print("MB3D demo: blurred video \(director.video?.lastPathComponent ?? "none"), \(director.status) \(director.error ?? "")")
            if let v = director.video {
                let dest = Library.folder.appendingPathComponent("demo-blur.mp4")
                try? FileManager.default.removeItem(at: dest)
                try? FileManager.default.copyItem(at: v, to: dest)
            }
            director.motionBlur = 1
        }
        status = String(localized: "3D model")
        sculptor.resolution = 64
        if let s = scene { sculptor.trace(s, name: "model") }
        try? await Task.sleep(for: .milliseconds(300))
        while sculptor.busy { try? await Task.sleep(for: .milliseconds(300)) }
        print("MB3D demo: mesh \(sculptor.status) \(sculptor.error ?? ""), usdz \(sculptor.usdz != nil)")
        keep(sculptor.usdz)
        keep(sculptor.files.first)
        // phase 4: a Monte Carlo render of 3 passes (montecarlo.png)
        status = String(localized: "Monte Carlo")
        renderMonteCarlo(maxPasses: 3)
        try? await Task.sleep(for: .milliseconds(300))
        await waitIdle()
        print("MB3D demo: done, status '\(status)', undo \(canUndo), scenes \(myScenes.map(\.lastPathComponent))")
    }

    private func waitIdle() async {
        while job != nil {
            try? await Task.sleep(for: .milliseconds(200))
        }
    }
}

/// `-snapshots`: every full picture is also written to the app's Documents
/// folder (the build machine's run collects them).
enum Snapshots {
    static let on = ProcessInfo.processInfo.arguments.contains("-snapshots")
    @MainActor private static var count = 0

    @MainActor
    static func save(_ image: CGImage) {
        guard on else { return }
        _ = PNG.write(image, to: Library.folder.appendingPathComponent(String(format: "picture-%02d.png", count)))
        count += 1
    }

    /// A picture under its own name (not counted).
    @MainActor
    static func save(_ image: CGImage, name: String) {
        guard on else { return }
        _ = PNG.write(image, to: Library.folder.appendingPathComponent(name).appendingPathExtension("png"))
    }
}

enum PNG {
    static func write(_ image: CGImage, to url: URL) -> Bool {
        guard let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil) else {
            return false
        }
        CGImageDestinationAddImage(dest, image, nil)
        return CGImageDestinationFinalize(dest)
    }
}
