import QuickLook
import SceneKit
import SwiftUI

/// The BulbTracer: the object as a triangle mesh, seen in AR (USDZ in
/// Quick Look) or shared as OBJ, PLY or STL for 3D printing and other
/// programs.
@MainActor
@Observable
final class Sculptor {
    var resolution = 192
    var colours = true
    var smoothing = 2
    /// the model's size in AR, in metres
    var size = 0.3

    var progress: Double?
    var status = ""
    var error: String?
    var usdz: URL?
    var files: [URL] = []
    @ObservationIgnored private var job: RenderJob?

    var busy: Bool { job != nil }

    func trace(_ scene: FractalScene, name: String) {
        guard !busy, let s = try? scene.copy() else { return }
        let job = RenderJob { stage, part in
            Task { @MainActor [weak self] in
                guard let self, self.job != nil else { return }
                self.progress = part
                self.status = stage
            }
        }
        self.job = job
        usdz = nil
        files = []
        progress = 0
        status = String(localized: "Tracing")
        let (res, col, smooth, size) = (resolution, colours, smoothing, size)
        Task.detached(priority: .userInitiated) {
            do {
                let mesh = try Mesh(tracing: s, resolution: res, colours: col, smoothing: smooth, size: size, job: job)
                guard mesh.faceCount > 0 else { throw EngineError(String(localized: "No surface found: the object may be bigger than the view.")) }
                let dir = FileManager.default.temporaryDirectory.appendingPathComponent("mesh-\(UUID().uuidString)")
                try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
                var out: [URL] = []
                for ext in ["obj", "ply", "stl"] {
                    let u = dir.appendingPathComponent(name).appendingPathExtension(ext)
                    try mesh.save(to: u)
                    out.append(u)
                }
                let u = dir.appendingPathComponent(name).appendingPathExtension("usdz")
                try mesh.scene().write(to: u, options: nil, delegate: nil, progressHandler: nil)
                let faces = mesh.faceCount
                let ready = out
                await MainActor.run { [weak self] in
                    guard let self, self.job === job else { return }
                    self.job = nil
                    self.progress = nil
                    self.status = String(localized: "\(faces.formatted()) triangles")
                    self.files = ready
                    self.usdz = FileManager.default.fileExists(atPath: u.path) ? u : nil
                }
            } catch {
                await MainActor.run { [weak self] in
                    guard let self, self.job === job else { return }
                    self.job = nil
                    self.progress = nil
                    self.status = ""
                    if !job.isStopped { self.error = "\(error)" }
                }
            }
        }
    }

    func stop() {
        job?.stop()
        job = nil
        progress = nil
        status = ""
    }
}

/// A traced mesh of the engine.
final class Mesh: @unchecked Sendable {
    let ptr: OpaquePointer

    init(tracing scene: FractalScene, resolution: Int, colours: Bool, smoothing: Int, size: Double, job: RenderJob) throws {
        let me = Unmanaged.passUnretained(job).toOpaque()
        let callback: mb3d_progress = { user, stage, done, total in
            guard let user else { return 0 }
            let job = Unmanaged<RenderJob>.fromOpaque(user).takeUnretainedValue()
            let name = stage.map { String(cString: $0) } ?? ""
            job.progress(name, total > 0 ? Double(done) / Double(total) : 0)
            return job.isStopped ? 1 : 0
        }
        guard let p = mb3d_mesh_trace(scene.ptr, UInt32(resolution), colours ? 1 : 0, UInt32(smoothing), size, callback, me)
        else { throw EngineError(Engine.lastError()) }
        ptr = p
    }

    deinit {
        mb3d_mesh_free(ptr)
    }

    var vertexCount: Int { Int(mb3d_mesh_vertex_count(ptr)) }
    var faceCount: Int { Int(mb3d_mesh_face_count(ptr)) }

    /// OBJ, PLY or STL by the file's extension.
    func save(to url: URL) throws {
        if url.path.withCString({ mb3d_mesh_save(ptr, $0) }) != 0 { throw EngineError(Engine.lastError()) }
    }

    /// A SceneKit scene of the mesh (vertex colours when traced with them).
    func scene() -> SCNScene {
        let n = vertexCount
        let f = faceCount
        let v = Data(bytes: mb3d_mesh_vertices(ptr), count: n * 3 * 4)
        let nm = Data(bytes: mb3d_mesh_normals(ptr), count: n * 3 * 4)
        let idx = Data(bytes: mb3d_mesh_faces(ptr), count: f * 3 * 4)
        var sources = [
            SCNGeometrySource(data: v, semantic: .vertex, vectorCount: n, usesFloatComponents: true,
                              componentsPerVector: 3, bytesPerComponent: 4, dataOffset: 0, dataStride: 12),
            SCNGeometrySource(data: nm, semantic: .normal, vectorCount: n, usesFloatComponents: true,
                              componentsPerVector: 3, bytesPerComponent: 4, dataOffset: 0, dataStride: 12),
        ]
        if let c = mb3d_mesh_colors(ptr) {
            // red, green, blue, palette position: the first three
            sources.append(SCNGeometrySource(data: Data(bytes: c, count: n * 4 * 4), semantic: .color, vectorCount: n,
                                             usesFloatComponents: true, componentsPerVector: 3, bytesPerComponent: 4,
                                             dataOffset: 0, dataStride: 16))
        }
        let element = SCNGeometryElement(data: idx, primitiveType: .triangles, primitiveCount: f, bytesPerIndex: 4)
        let geometry = SCNGeometry(sources: sources, elements: [element])
        let m = SCNMaterial()
        m.lightingModel = .physicallyBased
        m.diffuse.contents = mb3d_mesh_colors(ptr) == nil ? UIColor(white: 0.8, alpha: 1) : UIColor.white
        m.roughness.contents = 0.5
        m.isDoubleSided = true
        geometry.materials = [m]
        let scene = SCNScene()
        scene.rootNode.addChildNode(SCNNode(geometry: geometry))
        return scene
    }
}

struct SculptureView: View {
    let studio: Studio
    @Bindable var sculptor: Sculptor
    /// (in the panel beside the picture)
    let close: () -> Void
    @State private var preview: URL?

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Picker("Detail", selection: $sculptor.resolution) {
                        Text("Low (128)").tag(128)
                        Text("Medium (192)").tag(192)
                        Text("High (320)").tag(320)
                        Text("Very high (512)").tag(512)
                    }
                    Toggle("Colours from the palette", isOn: $sculptor.colours)
                    Stepper("Smoothing: \(sculptor.smoothing)", value: $sculptor.smoothing, in: 0...20)
                    Picker("Size in AR", selection: $sculptor.size) {
                        Text("10 cm").tag(0.1)
                        Text("30 cm").tag(0.3)
                        Text("1 m").tag(1.0)
                        Text("3 m").tag(3.0)
                    }
                } footer: {
                    Text("The object in a cube around the scene's centre, as the desktop's BulbTracer traces it. More detail takes longer and makes bigger files.")
                }
                Section {
                    if sculptor.busy {
                        ProgressView(value: sculptor.progress ?? 0)
                        Text(sculptor.status).font(.caption).foregroundStyle(.secondary)
                        Button("Stop", systemImage: "stop.fill", role: .destructive) { sculptor.stop() }
                    } else {
                        Button("Trace the object", systemImage: "cube.transparent") {
                            if let s = studio.currentScene { sculptor.trace(s, name: studio.title) }
                        }
                        if !sculptor.status.isEmpty {
                            Text(sculptor.status).font(.caption).foregroundStyle(.secondary)
                        }
                    }
                }
                if !sculptor.busy, sculptor.usdz != nil || !sculptor.files.isEmpty {
                    Section("The model") {
                        if let u = sculptor.usdz {
                            Button("See it in AR", systemImage: "arkit") { preview = u }
                            ShareLink(item: u) { Label("Share as USDZ", systemImage: "square.and.arrow.up") }
                        }
                        ForEach(sculptor.files, id: \.self) { f in
                            ShareLink(item: f) {
                                Label("Share as \(f.pathExtension.uppercased())", systemImage: "square.and.arrow.up")
                            }
                        }
                    }
                }
            }
            .navigationTitle("3D model")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close", systemImage: "xmark", action: close)
                }
            }
            .quickLookPreview($preview)
            .alert("3D model", isPresented: Binding(
                get: { sculptor.error != nil },
                set: { if !$0 { sculptor.error = nil } }
            )) {
                Button("OK") {}
            } message: {
                Text(sculptor.error ?? "")
            }
        }
    }
}
