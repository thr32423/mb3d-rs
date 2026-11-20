import SwiftUI

struct ContentView: View {
    @State private var studio = Studio()
    @State private var showSettings = true

    var body: some View {
        NavigationSplitView {
            List(studio.examples, id: \.self, selection: Binding(
                get: { studio.selected },
                set: { if let url = $0 { studio.open(url) } }
            )) { url in
                Text(url.deletingPathExtension().lastPathComponent)
                    .lineLimit(1)
            }
            .navigationTitle("Examples")
        } detail: {
            PictureView(studio: studio)
                .navigationTitle(studio.selected?.deletingPathExtension().lastPathComponent ?? "Mandelbulb3D")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .primaryAction) {
                        if studio.rendering {
                            Button("Stop", systemImage: "stop.fill") { studio.stop() }
                        } else {
                            Button("Render", systemImage: "arrow.clockwise") { studio.render() }
                                .disabled(studio.selected == nil)
                        }
                    }
                    ToolbarItem(placement: .primaryAction) {
                        Button("Settings", systemImage: "slider.horizontal.3") { showSettings.toggle() }
                    }
                }
                .inspector(isPresented: $showSettings) {
                    SettingsView(studio: studio)
                        .inspectorColumnWidth(min: 260, ideal: 300, max: 380)
                }
        }
        .onAppear(perform: autoOpen)
    }

    /// `-open NAME` (the build machine's screenshot run) opens that example.
    private func autoOpen() {
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-open"), i + 1 < args.count,
           let url = studio.examples.first(where: { $0.deletingPathExtension().lastPathComponent == args[i + 1] }) {
            studio.open(url)
        }
    }
}

struct PictureView: View {
    let studio: Studio

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            if let image = studio.image {
                Image(decorative: image, scale: 1)
                    .resizable()
                    .interpolation(.high)
                    .aspectRatio(contentMode: .fit)
            } else if studio.selected == nil {
                ContentUnavailableView("Choose an example", systemImage: "cube.transparent",
                                       description: Text("Pick a scene from the list."))
            }
            VStack {
                Spacer()
                if let p = studio.progress {
                    VStack(spacing: 6) {
                        ProgressView(value: p)
                        Text(studio.status).font(.caption).foregroundStyle(.secondary)
                    }
                    .padding()
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
                    .padding()
                } else if !studio.status.isEmpty {
                    Text(studio.status)
                        .font(.caption.monospacedDigit())
                        .padding(8)
                        .background(.regularMaterial, in: Capsule())
                        .padding()
                }
            }
        }
        .alert("Could not render", isPresented: Binding(
            get: { studio.error != nil },
            set: { if !$0 { studio.error = nil } }
        )) {
            Button("OK") {}
        } message: {
            Text(studio.error ?? "")
        }
    }
}

struct SettingsView: View {
    @Bindable var studio: Studio

    var body: some View {
        Form {
            Section("Camera") {
                LabeledSlider(title: "Zoom", value: $studio.zoom, range: 0.01...20, logarithmic: true, format: "%.3f",
                              done: studio.apply)
                LabeledSlider(title: "Field of view", value: $studio.fov, range: 5...150, format: "%.0f°",
                              done: studio.apply)
            }
            Section("Fractal") {
                LabeledSlider(title: "Iterations", value: $studio.iterations, range: 1...250, format: "%.0f",
                              done: studio.apply)
                LabeledSlider(title: "Detail (DE stop)", value: $studio.detail, range: 0.05...8, logarithmic: true,
                              format: "%.2f", done: studio.apply)
            }
            Section {
                LabeledContent("Engine", value: Engine.version)
            }
        }
        .disabled(studio.selected == nil)
    }
}

/// A slider that renders again when the finger lifts.
struct LabeledSlider: View {
    let title: String
    @Binding var value: Double
    let range: ClosedRange<Double>
    var logarithmic = false
    let format: String
    let done: () -> Void

    var body: some View {
        VStack(alignment: .leading) {
            LabeledContent(title, value: String(format: format, value))
            Slider(value: position, in: 0...1) { editing in
                if !editing { done() }
            }
        }
    }

    /// The slider's place 0...1 (on a log scale for wide ranges).
    private var position: Binding<Double> {
        Binding(
            get: {
                let v = min(max(value, range.lowerBound), range.upperBound)
                return logarithmic
                    ? log(v / range.lowerBound) / log(range.upperBound / range.lowerBound)
                    : (v - range.lowerBound) / (range.upperBound - range.lowerBound)
            },
            set: { p in
                value = logarithmic
                    ? range.lowerBound * pow(range.upperBound / range.lowerBound, p)
                    : range.lowerBound + p * (range.upperBound - range.lowerBound)
            }
        )
    }
}
