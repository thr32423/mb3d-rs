import AVFoundation
import CoreGraphics
import UIKit
import SwiftUI

/// The animation (in the panel beside the picture, its keyframes also in
/// a strip under the picture): keyframes taken from the views (or a turntable
/// around the object), the frames between them interpolated, rendered into
/// an MP4 video with the iPad's own encoder.
@MainActor
@Observable
final class Director {
    struct Key: Identifiable {
        let id = UUID()
        var text: String
        let thumbnail: CGImage?
        var frames: Int
    }

    var keys: [Key] = []
    var looped = false
    /// 0 linear, 1 Bezier (MB3D), 2 Catmull-Rom
    var smoothness = 2
    var fps = 30
    /// the video's longest side
    var side = 1280
    /// a turntable instead of the keyframes
    var turntable = false
    var turntableSeconds = 8.0
    /// motion blur: each frame the average of this many pictures within
    /// the shutter time (1: none)
    var motionBlur = 1
    /// the shutter time, a part of the time to the next frame
    var shutter = 0.5

    var progress: Double?
    var status = ""
    var video: URL?
    var error: String?
    @ObservationIgnored private var job: RenderJob?

    var busy: Bool { job != nil }

    var frameCount: Int {
        if turntable { return Int(turntableSeconds * Double(fps)) }
        let n = keys.count
        guard n > 1 else { return n }
        return keys.prefix(looped ? n : n - 1).reduce(0) { $0 + $1.frames } + (looped ? 0 : 1)
    }

    /// The keyframes as the engine's film (for the curves).
    func film() throws -> Film {
        let film = try Film(looped: looped, smoothness: smoothness)
        for key in keys {
            try film.add(FractalScene(text: key.text), frames: key.frames)
        }
        return film
    }

    func add(_ scene: FractalScene, thumbnail: CGImage?) {
        keys.append(Key(text: scene.text, thumbnail: thumbnail, frames: max(1, fps * 2)))
    }

    func remove(at offsets: IndexSet) {
        keys.remove(atOffsets: offsets)
    }

    func move(from: IndexSet, to: Int) {
        keys.move(fromOffsets: from, toOffset: to)
    }

    /// Renders the video from the keyframes (or a turntable of `scene`).
    func render(turntableOf scene: FractalScene?) {
        guard !busy else { return }
        let keys = self.keys
        let (looped, smoothness, fps, side) = (self.looped, self.smoothness, self.fps, self.side)
        let turntable = self.turntable
        let (blur, shutter) = (max(1, motionBlur), shutter)
        let orbitFrames = max(2, Int(turntableSeconds * Double(fps)))
        let base = try? (turntable ? scene : keys.first.flatMap { try? FractalScene(text: $0.text) })?.copy()
        guard let base else {
            error = turntable ? "Open a scene first." : "Add at least two keyframes."
            return
        }
        guard turntable || keys.count >= 2 else {
            error = String(localized: "Add at least two keyframes.")
            return
        }
        // the video's size: the scene's shape, even numbers for H.264
        let (w0, h0) = base.size
        let k = Double(side) / Double(max(w0, h0, 1))
        let w = max(16, Int(Double(w0) * k) / 2 * 2)
        let h = max(16, Int(Double(h0) * k) / 2 * 2)
        let job = RenderJob { stage, part in
            Task { @MainActor [weak self] in
                guard let self, self.job != nil else { return }
                self.status = stage
            }
        }
        self.job = job
        video = nil
        progress = 0
        status = String(localized: "Preparing")
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("Mandelbulb3D \(Self.stamp()).mp4")
        Task.detached(priority: .userInitiated) {
            do {
                let film: Film
                if turntable {
                    // 8 keyframes around, the frames shared out (a whole turn)
                    film = try Film(orbitOf: base, steps: 8, frames: max(1, orbitFrames / 8))
                } else {
                    film = try Film(looped: looped, smoothness: smoothness)
                    for key in keys {
                        let s = try FractalScene(text: key.text)
                        s.resize(width: w, height: h)
                        try film.add(s, frames: key.frames)
                    }
                }
                let n = film.frameCount
                let writer = try VideoWriter(url: url, width: w, height: h, fps: fps)
                for i in 0..<n {
                    if job.isStopped { throw EngineError(String(localized: "stopped")) }
                    // the frame, and with a motion blur the pictures to
                    // the shutter's close averaged
                    var scenes = [try film.frame(i)]
                    for k in 1..<blur {
                        scenes.append(try film.frame(i, offset: shutter * Double(k) / Double(blur)))
                    }
                    for s in scenes where s.size.width != w || s.size.height != h { s.resize(width: w, height: h) }
                    let image = try blur > 1 ? job.renderAverage(scenes) : job.render(scenes[0]).image
                    try await writer.append(image, frame: i)
                    await MainActor.run { [weak self] in
                        self?.progress = Double(i + 1) / Double(n)
                        self?.status = String(localized: "Frame \(i + 1) of \(n)")
                    }
                }
                try await writer.finish()
                await MainActor.run { [weak self] in
                    guard let self, self.job === job else { return }
                    self.job = nil
                    self.progress = nil
                    self.status = String(localized: "\(n) frames, \(String(format: "%.1f", Double(n) / Double(fps))) s")
                    self.video = url
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

    func saveToPhotos() {
        guard let video else { return }
        UISaveVideoAtPathToSavedPhotosAlbum(video.path, nil, nil, nil)
        status = String(localized: "Saved to Photos")
    }

    private static func stamp() -> String {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd HH.mm.ss"
        return f.string(from: Date())
    }
}

/// Frames into an H.264 MP4 (AVAssetWriter).
final class VideoWriter: @unchecked Sendable {
    private let writer: AVAssetWriter
    private let input: AVAssetWriterInput
    private let adaptor: AVAssetWriterInputPixelBufferAdaptor
    private let fps: Int32
    private let width: Int
    private let height: Int

    init(url: URL, width: Int, height: Int, fps: Int) throws {
        try? FileManager.default.removeItem(at: url)
        writer = try AVAssetWriter(outputURL: url, fileType: .mp4)
        input = AVAssetWriterInput(mediaType: .video, outputSettings: [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: width,
            AVVideoHeightKey: height,
            AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: width * height * 8],
        ])
        input.expectsMediaDataInRealTime = false
        adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input, sourcePixelBufferAttributes: [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
            kCVPixelBufferWidthKey as String: width,
            kCVPixelBufferHeightKey as String: height,
        ])
        guard writer.canAdd(input) else { throw EngineError(String(localized: "the video cannot be written")) }
        writer.add(input)
        guard writer.startWriting() else { throw EngineError(writer.error?.localizedDescription ?? "the video cannot be written") }
        writer.startSession(atSourceTime: .zero)
        self.fps = Int32(fps)
        self.width = width
        self.height = height
    }

    func append(_ image: CGImage, frame: Int) async throws {
        while !input.isReadyForMoreMediaData {
            try await Task.sleep(for: .milliseconds(10))
        }
        guard let pool = adaptor.pixelBufferPool else { throw EngineError(String(localized: "no pixel buffers")) }
        var buffer: CVPixelBuffer?
        CVPixelBufferPoolCreatePixelBuffer(nil, pool, &buffer)
        guard let buffer else { throw EngineError(String(localized: "no pixel buffer")) }
        CVPixelBufferLockBaseAddress(buffer, [])
        let ctx = CGContext(data: CVPixelBufferGetBaseAddress(buffer), width: width, height: height,
                            bitsPerComponent: 8, bytesPerRow: CVPixelBufferGetBytesPerRow(buffer),
                            space: CGColorSpace(name: CGColorSpace.sRGB)!,
                            bitmapInfo: CGImageAlphaInfo.noneSkipFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)
        ctx?.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
        CVPixelBufferUnlockBaseAddress(buffer, [])
        guard adaptor.append(buffer, withPresentationTime: CMTime(value: CMTimeValue(frame), timescale: fps)) else {
            throw EngineError(writer.error?.localizedDescription ?? "a frame could not be written")
        }
    }

    func finish() async throws {
        input.markAsFinished()
        await writer.finishWriting()
        if writer.status != .completed {
            throw EngineError(writer.error?.localizedDescription ?? "the video could not be finished")
        }
    }
}

struct AnimationView: View {
    let studio: Studio
    @Bindable var director: Director
    /// (in the panel beside the picture)
    let close: () -> Void
    /// (the screenshot tour's `-sheet curves` opens them at once)
    @State private var showCurves = Tour.sheet == "curves"

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Picker("Animation", selection: $director.turntable) {
                        Text("Keyframes").tag(false)
                        Text("Turntable").tag(true)
                    }
                    .pickerStyle(.segmented)
                }
                if director.turntable {
                    Section {
                        LabeledSlider(title: "Length", value: $director.turntableSeconds, range: 2...30,
                                      format: "%.0f s", done: {})
                    } footer: {
                        Text("The camera goes once around the object.")
                    }
                } else {
                    Section {
                        ForEach($director.keys) { $key in
                            HStack {
                                if let t = key.thumbnail {
                                    Image(decorative: t, scale: 1).resizable().aspectRatio(contentMode: .fit)
                                        .frame(width: 96, height: 64)
                                        .clipShape(RoundedRectangle(cornerRadius: 6))
                                }
                                Stepper("\(key.frames) frames to the next", value: $key.frames, in: 1...1000, step: 5)
                            }
                        }
                        .onDelete { director.remove(at: $0) }
                        .onMove { director.move(from: $0, to: $1) }
                        Button("Add the view as a keyframe", systemImage: "plus") {
                            if let s = studio.currentScene { director.add(s, thumbnail: studio.image) }
                        }
                    } header: {
                        Text("Keyframes")
                    } footer: {
                        Text("Move the camera with your fingers, then add the view. Swipe to remove a keyframe.")
                    }
                    Section {
                        NavigationLink {
                            CurvesView(director: director)
                        } label: {
                            Label("Curves", systemImage: "chart.xyaxis.line")
                        }
                        .disabled(director.keys.count < 2)
                        Toggle("Loop (back to the first)", isOn: $director.looped)
                        Picker("Between keyframes", selection: $director.smoothness) {
                            Text("Straight").tag(0)
                            Text("Smooth (MB3D)").tag(1)
                            Text("Through every keyframe").tag(2)
                        }
                    }
                }
                Section {
                    Picker("Frames per second", selection: $director.fps) {
                        Text("24").tag(24)
                        Text("25").tag(25)
                        Text("30").tag(30)
                        Text("60").tag(60)
                    }
                    Picker("Size", selection: $director.side) {
                        Text("640").tag(640)
                        Text("1280 (HD)").tag(1280)
                        Text("1920 (Full HD)").tag(1920)
                        Text("3840 (4K)").tag(3840)
                    }
                    LabeledContent("Frames", value: "\(director.frameCount)")
                }
                Section {
                    Picker("Motion blur", selection: $director.motionBlur) {
                        Text("Off").tag(1)
                        Text("4 pictures").tag(4)
                        Text("8 pictures").tag(8)
                        Text("16 pictures").tag(16)
                    }
                    if director.motionBlur > 1 {
                        LabeledSlider(title: "Shutter", value: $director.shutter, range: 0.1...1,
                                      format: "%.2f", done: {})
                    }
                } footer: {
                    Text(director.motionBlur > 1
                         ? "Each frame is the average of \(director.motionBlur) pictures on the way to the next frame: \(director.motionBlur) times the render time."
                         : "Moving parts sharp in every frame.")
                }
                Section {
                    if director.busy {
                        ProgressView(value: director.progress ?? 0)
                        Text(director.status).font(.caption).foregroundStyle(.secondary)
                        Button("Stop", systemImage: "stop.fill", role: .destructive) { director.stop() }
                    } else {
                        Button("Render the video", systemImage: "film") {
                            director.render(turntableOf: studio.currentScene)
                        }
                        if !director.status.isEmpty {
                            Text(director.status).font(.caption).foregroundStyle(.secondary)
                        }
                    }
                    if let v = director.video, !director.busy {
                        ShareLink(item: v) { Label("Share the video", systemImage: "square.and.arrow.up") }
                        Button("Save to Photos", systemImage: "photo.badge.plus") { director.saveToPhotos() }
                    }
                }
            }
            .navigationTitle("Animation")
            .navigationDestination(isPresented: $showCurves) {
                CurvesView(director: director)
            }
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close", systemImage: "xmark", action: close)
                }
                if !director.turntable {
                    ToolbarItem(placement: .primaryAction) { EditButton() }
                }
            }
            .alert("Animation", isPresented: Binding(
                get: { director.error != nil },
                set: { if !$0 { director.error = nil } }
            )) {
                Button("OK") {}
            } message: {
                Text(director.error ?? "")
            }
        }
    }
}

/// The keyframes under the picture while the animation's panel is open:
/// a tap goes to a keyframe's view, the plus takes the view as the next
/// keyframe (the camera stays free meanwhile), and the slider runs through
/// the frames in quick small pictures.
struct KeyframeStrip: View {
    let studio: Studio
    @Bindable var director: Director
    @State private var film: Film?
    @State private var frame = 0.0
    @State private var showCurves = false

    var body: some View {
        VStack(spacing: 6) {
            if director.keys.count > 1 {
                HStack(spacing: 10) {
                    Slider(value: $frame, in: 0...Double(max(director.frameCount - 1, 1)), step: 1) { editing in
                        if editing {
                            film = try? director.film()
                        } else {
                            film = nil
                            studio.endPeek()
                        }
                    }
                    Text(verbatim: "\(Int(frame) + 1) / \(director.frameCount)")
                        .font(.caption.monospacedDigit())
                        .foregroundStyle(.secondary)
                        .frame(minWidth: 64, alignment: .trailing)
                    Button("Curves", systemImage: "chart.xyaxis.line") { showCurves = true }
                        .labelStyle(.iconOnly)
                        .popover(isPresented: $showCurves) {
                            NavigationStack {
                                CurvesView(director: director)
                                    .navigationTitle("Curves")
                                    .navigationBarTitleDisplayMode(.inline)
                            }
                            .frame(minWidth: 440, minHeight: 560)
                        }
                }
                .onChange(of: frame) {
                    if let film, let s = try? film.frame(Int(frame)) { studio.peek(s) }
                }
            }
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(Array(director.keys.enumerated()), id: \.element.id) { i, key in
                        Button { studio.go(to: key.text) } label: {
                            ZStack(alignment: .bottomLeading) {
                                Color.black
                                if let t = key.thumbnail {
                                    Image(decorative: t, scale: 1).resizable().aspectRatio(contentMode: .fill)
                                }
                                Text(verbatim: "\(i + 1)")
                                    .font(.caption2.monospacedDigit().weight(.semibold))
                                    .padding(.horizontal, 5)
                                    .padding(.vertical, 2)
                                    .background(.regularMaterial, in: Capsule())
                                    .padding(3)
                            }
                            .frame(width: 84, height: 56)
                            .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                        }
                        .buttonStyle(.plain)
                        .contextMenu {
                            Button("Delete", systemImage: "trash", role: .destructive) {
                                director.remove(at: IndexSet(integer: i))
                            }
                        }
                        .accessibilityLabel("Keyframe \(i + 1)")
                    }
                    Button {
                        if let s = studio.currentScene { director.add(s, thumbnail: studio.image) }
                    } label: {
                        Image(systemName: "plus")
                            .font(.title3)
                            .frame(width: 56, height: 56)
                            .background(Color.accentColor, in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                            .foregroundStyle(Color.white)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Add the view as a keyframe")
                }
            }
        }
        .padding(8)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }
}
