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
        // the user's own (folders in the app's Documents, seen in the Files
        // app): formulas first, so they win over the app's of the same name
        for dir in [Library.formulas, Library.backgrounds] {
            try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        }
        _ = Library.backgrounds.path.withCString { mb3d_add_map_folder($0) }
        _ = Library.formulas.path.withCString { mb3d_add_formula_folder($0) }
    }

    /// The example files that come with the app.
    static func examples() -> [URL] {
        guard let dir = Bundle.main.url(forResource: "Data", withExtension: nil)?.appendingPathComponent("M3Parameter"),
              let files = try? FileManager.default.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil)
        else { return [] }
        return files.filter { $0.pathExtension.lowercased() == "m3p" }
            .sorted { $0.lastPathComponent.localizedStandardCompare($1.lastPathComponent) == .orderedAscending }
    }

    /// A picture of the engine as an image.
    static func image(of pic: OpaquePointer) throws -> CGImage {
        let w = Int(mb3d_picture_width(pic))
        let h = Int(mb3d_picture_height(pic))
        guard let rgb = mb3d_picture_rgb(pic) else { throw EngineError(String(localized: "no picture")) }
        return try image(rgb: Data(bytes: rgb, count: w * h * 3), width: w, height: h)
    }

    /// An image of `width` x `height` RGB bytes.
    static func image(rgb data: Data, width w: Int, height h: Int) throws -> CGImage {
        guard let provider = CGDataProvider(data: data as CFData),
              let image = CGImage(
                  width: w, height: h, bitsPerComponent: 8, bitsPerPixel: 24, bytesPerRow: w * 3,
                  space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.none.rawValue),
                  provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent)
        else { throw EngineError(String(localized: "the picture could not be made")) }
        return image
    }

    static func lastError() -> String {
        String(cString: mb3d_last_error())
    }

    static var version: String {
        String(cString: mb3d_engine_version())
    }

    /// All formula names (built in, then the formula folders').
    @MainActor static private(set) var formulaNames: [String] = readFormulaNames()

    /// After formula files came in: the names again.
    @MainActor static func reloadFormulas() {
        formulaNames = readFormulaNames()
        formulaBook = readFormulaBook()
    }

    /// The Formula book in short: a formula's kind (0 to 6: 3D, 3D with
    /// analytic DE, 4D, 4D with analytic DE, transformation, dIFS shape,
    /// dIFS transformation) and its author's description.
    @MainActor static private(set) var formulaBook: [String: (kind: Int, about: String)] = readFormulaBook()

    private static func readFormulaBook() -> [String: (kind: Int, about: String)] {
        guard let t = mb3d_formula_book() else { return [:] }
        defer { mb3d_string_free(t) }
        var book: [String: (kind: Int, about: String)] = [:]
        for line in String(cString: t).split(separator: "\n") {
            let f = line.split(separator: "\t", maxSplits: 2, omittingEmptySubsequences: false)
            guard f.count == 3 else { continue }
            book[String(f[0])] = (Int(f[1]) ?? 0, f[2].replacingOccurrences(of: "\u{1}", with: "\n"))
        }
        return book
    }

    private static func readFormulaNames() -> [String] {
        guard let t = mb3d_formula_names() else { return [] }
        defer { mb3d_string_free(t) }
        return String(cString: t).split(separator: "\n").map(String.init)
    }

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

    /// The parameters as Mandelbulb3D text (`Mandelbulb3Dv18{...}`).
    func mb3dText(title: String) -> String? {
        guard let t = title.withCString({ mb3d_scene_mb3d_text(ptr, $0) }) else { return nil }
        defer { mb3d_string_free(t) }
        return String(cString: t)
    }

    /// The depth at a pixel as `dof_focus` takes it; nil on the background.
    func focus(x: Int, y: Int) -> Double? {
        guard x >= 0, y >= 0 else { return nil }
        var f = 0.0
        return mb3d_scene_pick(ptr, UInt32(x), UInt32(y), &f) == 0 ? f : nil
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
        return (try Engine.image(of: pic), mb3d_picture_seconds(pic))
    }

    /// The scenes rendered and averaged (a motion blur's pictures; all of
    /// one size).
    func renderAverage(_ scenes: [FractalScene]) throws -> CGImage {
        let me = Unmanaged.passUnretained(self).toOpaque()
        let callback: mb3d_progress = { user, stage, done, total in
            guard let user else { return 0 }
            let job = Unmanaged<RenderJob>.fromOpaque(user).takeUnretainedValue()
            let name = stage.map { String(cString: $0) } ?? ""
            job.progress(name, total > 0 ? Double(done) / Double(total) : 0)
            return job.isStopped ? 1 : 0
        }
        var sum: [UInt32] = []
        var (w, h) = (0, 0)
        for scene in scenes {
            guard let pic = mb3d_render(scene.ptr, callback, me) else { throw EngineError(Engine.lastError()) }
            defer { mb3d_picture_free(pic) }
            let pw = Int(mb3d_picture_width(pic))
            let ph = Int(mb3d_picture_height(pic))
            guard let rgb = mb3d_picture_rgb(pic) else { throw EngineError(String(localized: "no picture")) }
            if sum.isEmpty {
                (w, h) = (pw, ph)
                sum = [UInt32](repeating: 0, count: w * h * 3)
            }
            guard (pw, ph) == (w, h) else { throw EngineError("the pictures differ in size") }
            for i in 0..<sum.count { sum[i] += UInt32(rgb[i]) }
        }
        let n = UInt32(max(scenes.count, 1))
        let bytes = Data(sum.map { UInt8(min(255, ($0 + n / 2) / n)) })
        return try Engine.image(rgb: bytes, width: w, height: h)
    }

    /// MutaGen's 15 variations of `scene` (the first is the scene), on
    /// this thread.
    func variations(_ scene: FractalScene, seed: UInt64, mutation: [Double]) throws -> [FractalScene] {
        precondition(mutation.count == 7)
        let me = Unmanaged.passUnretained(self).toOpaque()
        let callback: mb3d_progress = { user, stage, done, total in
            guard let user else { return 0 }
            let job = Unmanaged<RenderJob>.fromOpaque(user).takeUnretainedValue()
            let name = stage.map { String(cString: $0) } ?? ""
            job.progress(name, total > 0 ? Double(done) / Double(total) : 0)
            return job.isStopped ? 1 : 0
        }
        var out = [OpaquePointer?](repeating: nil, count: 15)
        if mb3d_variations_mutation(scene.ptr, seed, mutation, &out, callback, me) != 0 {
            throw EngineError(Engine.lastError())
        }
        return out.compactMap { $0.map(FractalScene.init) }
    }
}

/// A Monte Carlo render (soft light by random rays), pass after pass.
final class MonteCarloRender: @unchecked Sendable {
    let ptr: OpaquePointer

    init(_ scene: FractalScene) throws {
        guard let p = mb3d_montecarlo_new(scene.ptr) else { throw EngineError(Engine.lastError()) }
        ptr = p
    }

    deinit {
        mb3d_montecarlo_free(ptr)
    }

    /// One more pass on this thread; `job` reports and stops it.
    func pass(_ job: RenderJob) throws {
        let me = Unmanaged.passUnretained(job).toOpaque()
        let callback: mb3d_progress = { user, stage, done, total in
            guard let user else { return 0 }
            let job = Unmanaged<RenderJob>.fromOpaque(user).takeUnretainedValue()
            let name = stage.map { String(cString: $0) } ?? ""
            job.progress(name, total > 0 ? Double(done) / Double(total) : 0)
            return job.isStopped ? 1 : 0
        }
        if mb3d_montecarlo_pass(ptr, callback, me) != 0 { throw EngineError(Engine.lastError()) }
    }

    var passes: Int { Int(mb3d_montecarlo_passes(ptr)) }

    /// The picture so far.
    func image() throws -> CGImage {
        guard let pic = mb3d_montecarlo_picture(ptr) else { throw EngineError(Engine.lastError()) }
        defer { mb3d_picture_free(pic) }
        return try Engine.image(of: pic)
    }
}

/// An animation: keyframes (scenes of one size) and the frames between
/// them, interpolated as the desktop's Animation window does.
final class Film: @unchecked Sendable {
    let ptr: OpaquePointer

    /// `smoothness`: 0 linear, 1 Bezier (MB3D), 2 Catmull-Rom (through every
    /// keyframe).
    init(looped: Bool, smoothness: Int) throws {
        guard let p = mb3d_film_new(looped ? 1 : 0, Int32(smoothness)) else { throw EngineError(Engine.lastError()) }
        ptr = p
    }

    /// A turntable: the camera around the object in `steps` keyframes of
    /// `frames` frames each.
    init(orbitOf scene: FractalScene, steps: Int, frames: Int) throws {
        guard let p = mb3d_film_orbit(scene.ptr, UInt32(steps), UInt32(frames)) else {
            throw EngineError(Engine.lastError())
        }
        ptr = p
    }

    deinit {
        mb3d_film_free(ptr)
    }

    func add(_ scene: FractalScene, frames: Int) throws {
        if mb3d_film_add(ptr, scene.ptr, UInt32(max(1, frames))) != 0 { throw EngineError(Engine.lastError()) }
    }

    var keyframeCount: Int { Int(mb3d_film_keyframes(ptr)) }

    func keyframe(_ i: Int) throws -> FractalScene {
        guard let p = mb3d_film_keyframe(ptr, UInt32(i)) else { throw EngineError(Engine.lastError()) }
        return FractalScene(p)
    }

    var frameCount: Int { Int(mb3d_film_frames(ptr)) }

    /// The scene of frame `i` (from 0).
    func frame(_ i: Int) throws -> FractalScene {
        guard let p = mb3d_film_frame(ptr, UInt32(i)) else { throw EngineError(Engine.lastError()) }
        return FractalScene(p)
    }

    /// The scene `offset` (0..1) of the way to the next frame: the pictures
    /// a motion blur averages.
    func frame(_ i: Int, offset: Double) throws -> FractalScene {
        guard let p = mb3d_film_frame_at(ptr, UInt32(i), offset) else { throw EngineError(Engine.lastError()) }
        return FractalScene(p)
    }

    /// The short names of the values the curves show (zoom, x, ...).
    static let curveNames: [String] = {
        guard let t = mb3d_curve_names() else { return [] }
        defer { mb3d_string_free(t) }
        return String(cString: t).split(separator: "\n").map(String.init)
    }()

    /// Curve `param`: its value at every frame (NaN where a frame has none)
    /// and the keyframes' frames and values.
    func curve(_ param: Int) -> (values: [Double], keys: [(frame: Int, value: Double)])? {
        let n = frameCount
        let k = keyframeCount
        var values = [Double](repeating: .nan, count: max(n, 1))
        var frames = [UInt32](repeating: 0, count: max(k, 1))
        var kv = [Double](repeating: .nan, count: max(k, 1))
        guard mb3d_film_curve(ptr, UInt32(param), &values, &frames, &kv) == 0 else { return nil }
        return (Array(values.prefix(n)), (0..<k).map { (Int(frames[$0]), kv[$0]) })
    }

    /// Sets value `param` of keyframe `key` (read it back with `keyframe`).
    func setCurveKey(_ param: Int, key: Int, value: Double) {
        _ = mb3d_film_set_curve_key(ptr, UInt32(param), UInt32(key), value)
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

    /// The scene's directions of the picture's right, down and depth (9
    /// numbers, row after row), for the compass.
    var axes: [Double] {
        var m = [Double](repeating: 0, count: 9)
        mb3d_navigator_axes(ptr, &m)
        return m
    }

    /// The view as a scene: quick (simpler light) or full.
    func scene(quick: Bool, width: Int, height: Int) throws -> FractalScene {
        guard let p = mb3d_navigator_scene(ptr, quick ? 1 : 0, UInt32(width), UInt32(height)) else {
            throw EngineError(Engine.lastError())
        }
        return FractalScene(p)
    }
}
