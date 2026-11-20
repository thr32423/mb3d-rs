import CoreGraphics
import Foundation
import Observation

/// What the app shows: the open scene, its picture and the render running.
@MainActor
@Observable
final class Studio {
    var examples: [URL] = Engine.examples()
    var selected: URL?
    var image: CGImage?
    var status = ""
    var progress: Double?
    var error: String?

    // the settings the prototype shows (read from the scene when it opens)
    var zoom = 1.0
    var fov = 30.0
    var iterations = 30.0
    var detail = 1.0

    @ObservationIgnored private var scene: FractalScene?
    private var job: RenderJob?

    /// The longest side of a render (the prototype keeps renders quick).
    let renderSide = 1024

    func open(_ url: URL) {
        selected = url
        error = nil
        do {
            let s = try FractalScene.open(url)
            let v = s.values
            zoom = Double(v["zoom"] ?? "") ?? 1
            fov = Double(v["fov"] ?? "") ?? 30
            iterations = Double(v["iterations"] ?? "") ?? 30
            detail = Double(v["de_stop"] ?? "") ?? 1
            let (w, h) = s.size
            let k = Double(renderSide) / Double(max(w, h, 1))
            s.resize(width: max(1, Int(Double(w) * k)), height: max(1, Int(Double(h) * k)))
            scene = s
            image = nil
            render()
        } catch {
            self.error = "\(error)"
        }
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
            render()
        } catch {
            self.error = "\(error)"
        }
    }

    func render() {
        guard let scene, let copy = try? scene.copy() else { return }
        job?.stop()
        let job = RenderJob { stage, part in
            Task { @MainActor [weak self] in
                guard let self, self.job != nil else { return }
                self.status = stage
                self.progress = part
            }
        }
        self.job = job
        progress = 0
        status = "Starting"
        Task.detached(priority: .userInitiated) {
            let result = Result { try job.render(copy) }
            await MainActor.run { [weak self] in
                guard let self, self.job === job else { return }
                self.job = nil
                self.progress = nil
                switch result {
                case .success(let r):
                    self.image = r.image
                    self.status = String(format: "%.2f s", r.seconds)
                case .failure(let e):
                    self.status = ""
                    if !job.isStopped { self.error = "\(e)" }
                }
            }
        }
    }

    func stop() {
        job?.stop()
    }

    var rendering: Bool { job != nil }
}
