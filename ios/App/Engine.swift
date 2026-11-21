import CoreGraphics
import Foundation

/// The mb3d-rs engine (engine/include/mb3d.h, linked as libmb3d.a).
enum Engine {
    /// The formula and map folders that come with the app, read once.
    static func start() {
        mb3d_use_gpu(1)
        guard let data = Bundle.main.url(forResource: "Data", withExtension: nil) else { return }
        for folder in ["M3Formulas", "EM_JIT_M3Formulas"] {
            _ = data.appendingPathComponent(folder).path.withCString { mb3d_add_formula_folder($0) }
        }
        _ = data.appendingPathComponent("M3Maps").path.withCString { mb3d_add_map_folder($0) }
    }

    /// The example files that come with the app.
    static func examples() -> [URL] {
        guard let dir = Bundle.main.url(forResource: "Data", withExtension: nil)?.appendingPathComponent("M3Parameter"),
              let files = try? FileManager.default.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil)
        else { return [] }
        return files.filter { $0.pathExtension.lowercased() == "m3p" }
            .sorted { $0.lastPathComponent.localizedStandardCompare($1.lastPathComponent) == .orderedAscending }
    }

    static func lastError() -> String {
        String(cString: mb3d_last_error())
    }

    static var version: String {
        String(cString: mb3d_engine_version())
    }

    /// All formula names (built in, then the formula folders'), read once.
    static let formulaNames: [String] = {
        guard let t = mb3d_formula_names() else { return [] }
        defer { mb3d_string_free(t) }
        return String(cString: t).split(separator: "\n").map(String.init)
    }()

    /// A formula's settings with their default values, in their order.
    static func formulaSettings(_ name: String) -> [(key: String, value: String)] {
        guard let t = name.withCString({ mb3d_formula_settings($0) }) else { return [] }
        defer { mb3d_string_free(t) }
        return String(cString: t).split(separator: "\n").compactMap { line in
            let parts = line.split(separator: "=", maxSplits: 1)
            guard parts.count == 2 else { return nil }
            return (parts[0].trimmingCharacters(in: .whitespaces), parts[1].trimmingCharacters(in: .whitespaces))
        }
    }
}

/// A scene of the engine. One thread at a time: a render works on a copy.
final class FractalScene: @unchecked Sendable {
    let ptr: OpaquePointer

    init(_ ptr: OpaquePointer) {
        self.ptr = ptr
    }

    deinit {
        mb3d_scene_free(ptr)
    }

    static func open(_ url: URL) throws -> FractalScene {
        guard let p = url.path.withCString({ mb3d_open($0) }) else { throw EngineError(Engine.lastError()) }
        return FractalScene(p)
    }

    /// A scene from its whole text (as `text` gives it).
    convenience init(text: String) throws {
        guard let p = text.withCString({ mb3d_open_text($0) }) else { throw EngineError(Engine.lastError()) }
        self.init(p)
    }

    /// Writes the scene as a Mandelbulb3D parameter file (.m3p).
    func saveM3P(to url: URL) throws {
        if url.path.withCString({ mb3d_scene_save_m3p(ptr, $0) }) != 0 { throw EngineError(Engine.lastError()) }
    }

    func copy() throws -> FractalScene {
        guard let p = mb3d_scene_copy(ptr) else { throw EngineError(Engine.lastError()) }
        return FractalScene(p)
    }

    /// The scene as `key = value` lines.
    var text: String {
        guard let t = mb3d_scene_text(ptr) else { return "" }
        defer { mb3d_string_free(t) }
        return String(cString: t)
    }

    /// The settings of `text` (the scene's own lines, not the formulas').
    var values: [String: String] {
        var out: [String: String] = [:]
        for line in text.split(separator: "\n") {
            if line.hasPrefix("[") { break }
            let parts = line.split(separator: "=", maxSplits: 1)
            if parts.count == 2 {
                out[parts[0].trimmingCharacters(in: .whitespaces)] = parts[1].trimmingCharacters(in: .whitespaces)
            }
        }
        return out
    }

    /// Changes settings: lines like `zoom = 0.3`.
    func set(_ changes: String) throws {
        if changes.withCString({ mb3d_scene_set(ptr, $0) }) != 0 { throw EngineError(Engine.lastError()) }
    }

    var size: (width: Int, height: Int) {
        var w: UInt32 = 0
        var h: UInt32 = 0
        _ = mb3d_scene_size(ptr, &w, &h)
        return (Int(w), Int(h))
    }

    func resize(width: Int, height: Int) {
        _ = mb3d_scene_resize(ptr, UInt32(width), UInt32(height))
    }
}

struct EngineError: Error, CustomStringConvertible {
    let description: String
    init(_ d: String) {
        description = d
    }
}

/// A render that can be stopped and tells its progress (from the engine's
/// threads, one call at a time).
final class RenderJob: @unchecked Sendable {
    private let lock = NSLock()
    private var stopped = false
    let progress: @Sendable (String, Double) -> Void

    init(progress: @escaping @Sendable (String, Double) -> Void) {
        self.progress = progress
    }

    func stop() {
        lock.lock()
        stopped = true
        lock.unlock()
    }

    var isStopped: Bool {
        lock.lock()
        defer { lock.unlock() }
        return stopped
    }

    /// Renders `scene` (a copy of its own) on this thread.
    func render(_ scene: FractalScene) throws -> (image: CGImage, seconds: Double) {
        let me = Unmanaged.passUnretained(self).toOpaque()
        let callback: mb3d_progress = { user, stage, done, total in
            guard let user else { return 0 }
            let job = Unmanaged<RenderJob>.fromOpaque(user).takeUnretainedValue()
            let name = stage.map { String(cString: $0) } ?? ""
            job.progress(name, total > 0 ? Double(done) / Double(total) : 0)
            return job.isStopped ? 1 : 0
        }
        guard let pic = mb3d_render(scene.ptr, callback, me) else { throw EngineError(Engine.lastError()) }
        defer { mb3d_picture_free(pic) }
        let w = Int(mb3d_picture_width(pic))
        let h = Int(mb3d_picture_height(pic))
        guard let rgb = mb3d_picture_rgb(pic) else { throw EngineError("no picture") }
        let data = Data(bytes: rgb, count: w * h * 3)
        guard let provider = CGDataProvider(data: data as CFData),
              let image = CGImage(
                  width: w, height: h, bitsPerComponent: 8, bitsPerPixel: 24, bytesPerRow: w * 3,
                  space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.none.rawValue),
                  provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent)
        else { throw EngineError("the picture could not be made") }
        return (image, mb3d_picture_seconds(pic))
    }
}

/// A camera flying through a scene (the engine's navigator): moves are
/// measured with the distance to the surface. Used on the main thread; the
/// scenes it gives are rendered elsewhere.
final class Navigator {
    let ptr: OpaquePointer

    init(_ scene: FractalScene) throws {
        guard let p = mb3d_navigator_new(scene.ptr) else { throw EngineError(Engine.lastError()) }
        ptr = p
    }

    deinit {
        mb3d_navigator_free(ptr)
    }

    /// Degrees: up/down, left/right, about the view axis.
    func turn(pitch: Double, yaw: Double, roll: Double) {
        mb3d_navigator_turn(ptr, pitch, yaw, roll)
    }

    /// A drag of (dx, dy) picture heights.
    func slide(dx: Double, dy: Double) {
        mb3d_navigator_slide(ptr, dx, dy)
    }

    /// Percent of the distance to the surface, negative backward.
    func fly(percent: Double) {
        mb3d_navigator_fly(ptr, percent)
    }

    var fov: Double {
        mb3d_navigator_fov(ptr)
    }

    /// The view as a scene: quick (simpler light) or full.
    func scene(quick: Bool, width: Int, height: Int) throws -> FractalScene {
        guard let p = mb3d_navigator_scene(ptr, quick ? 1 : 0, UInt32(width), UInt32(height)) else {
            throw EngineError(Engine.lastError())
        }
        return FractalScene(p)
    }
}
