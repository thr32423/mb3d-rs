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
            status = "Saved as \(dest.lastPathComponent)"
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
        pending?.cancel()
        pending = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(500))
            guard !Task.isCancelled else { return }
            self?.commit()
        }
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
            set: { self.doc[key] = $0 ? onValue : "off"; self.commitSoon() }
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
            if scene != nil { status = "Switch on depth of field (Effects) to focus with a tap" }
            return
        }
        let (w, h) = scene.size
        let k = min(size.width / Double(w), size.height / Double(h))
        guard k > 0 else { return }
        let x = Int(((point.x - (size.width - Double(w) * k) / 2) / k).rounded(.down))
        let y = Int(((point.y - (size.height - Double(h) * k) / 2) / k).rounded(.down))
        guard x >= 0, y >= 0, x < w, y < h else { return }
        guard let z = scene.focus(x: x, y: y) else {
            status = "Nothing there to focus on"
            return
        }
        doc["dof_focus"] = String(z)
        commit()
        status = "Focus set"
    }

    /// The parameters as Mandelbulb3D text on the clipboard.
    func copyParameters() {
        guard let s = try? sceneToSave(), let t = s.mb3dText(title: title) else { return }
        UIPasteboard.general.string = t
        status = "Parameters copied"
    }

    /// Parameters from the clipboard (MB3D text or a scene text) become a
    /// new scene among the user's.
    func pasteParameters() {
        guard let t = UIPasteboard.general.string, !t.isEmpty else {
            error = "The clipboard holds no text."
            return
        }
        do {
            let s = try FractalScene(text: t)
            let dest = Library.freeName("Pasted", "m3p")
            try s.saveM3P(to: dest)
            myScenes = Library.scenes()
            open(dest)
        } catch {
            self.error = "The clipboard holds no parameters: \(error)"
        }
    }

    /// The last full picture into the Photos library.
    func saveToPhotos() {
        guard let image else { return }
        UIImageWriteToSavedPhotosAlbum(UIImage(cgImage: image), nil, nil, nil)
        status = "Saved to Photos"
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
                self.status = stage
                self.progress = part
            }
        }
        self.job = job
        jobQuick = quick
        if !quick {
            progress = 0
            status = "Starting"
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
            status = wasQuick ? "" : String(format: "%.2f s", r.seconds)
            if !wasQuick {
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
            if dirty {
                renderQuick()
            } else if !moving {
                render()
            }
        }
    }

    func stop() {
        job?.stop()
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
