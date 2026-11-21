import CoreGraphics
import Foundation
import ImageIO
import Observation
import SwiftUI

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
    @ObservationIgnored private var fileSize = (width: 0, height: 0)
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

    /// The longest side of a full render (the prototype keeps renders quick).
    let renderSide = 1024
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
            let k = Double(renderSide) / Double(max(w, h, 1))
            s.resize(width: max(1, Int(Double(w) * k)), height: max(1, Int(Double(h) * k)))
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

    private static func cgColor(_ hex: String?) -> CGColor {
        let (r, g, b) = hex.flatMap(HexColor.rgb) ?? (1, 1, 1)
        return CGColor(srgbRed: r, green: g, blue: b, alpha: 1)
    }

    private static func hex(_ c: CGColor) -> String? {
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
        take(s)
        render()
    }

    private func moved() {
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
