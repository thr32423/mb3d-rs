import CoreGraphics
import Foundation
import ImageIO
import Observation

/// What the app shows: the open scene, its picture and the render running.
///
/// The camera is moved by gestures through a `Navigator`. While a finger
/// moves, quick small pictures follow it (one at a time, the newest view);
/// when the fingers lift, the full picture of the view is rendered.
@MainActor
@Observable
final class Studio {
    var examples: [URL] = Engine.examples()
    var selected: URL?
    var image: CGImage?
    var status = ""
    var progress: Double?
    var error: String?

    // the settings the prototype shows (read from the scene)
    var zoom = 1.0
    var fov = 30.0
    var iterations = 30.0
    var detail = 1.0

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
        let v = s.values
        zoom = Double(v["zoom"] ?? "") ?? 1
        fov = Double(v["fov"] ?? "") ?? 30
        iterations = Double(v["iterations"] ?? "") ?? 30
        detail = Double(v["de_stop"] ?? "") ?? 1
    }

    /// Writes the controls' values into the scene and renders again.
    func apply() {
        guard let scene else { return }
        let lines = [
            "zoom = \(zoom)",
            "fov = \(fov)",
            "iterations = \(Int(iterations.rounded()))",
            "de_stop = \(detail)",
        ].joined(separator: "\n")
        do {
            try scene.set(lines)
            take(scene)
            render()
        } catch {
            self.error = "\(error)"
        }
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
            let v = s.values
            zoom = Double(v["zoom"] ?? "") ?? zoom
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
            if !wasQuick { Snapshots.save(r.image) }
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
    /// full picture (saved with `-snapshots`).
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
        guard on, let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first,
              let dest = CGImageDestinationCreateWithURL(
                  dir.appendingPathComponent(String(format: "picture-%02d.png", count)) as CFURL,
                  "public.png" as CFString, 1, nil)
        else { return }
        count += 1
        CGImageDestinationAddImage(dest, image, nil)
        CGImageDestinationFinalize(dest)
    }
}
