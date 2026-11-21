import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @State private var studio = Studio()
    @State private var showSettings = true
    @State private var importing = false
    @State private var exporting = false
    @State private var exportDoc: M3PDocument?

    var body: some View {
        NavigationSplitView {
            List(selection: Binding(
                get: { studio.selected },
                set: { if let url = $0, url != studio.selected { studio.open(url) } }
            )) {
                if !studio.myScenes.isEmpty {
                    Section("My scenes") {
                        ForEach(studio.myScenes, id: \.self) { url in
                            Text(url.deletingPathExtension().lastPathComponent)
                                .lineLimit(1)
                                .swipeActions {
                                    Button("Delete", systemImage: "trash", role: .destructive) { studio.delete(url) }
                                }
                        }
                    }
                }
                Section("Examples") {
                    ForEach(studio.examples, id: \.self) { url in
                        Text(url.deletingPathExtension().lastPathComponent)
                            .lineLimit(1)
                    }
                }
            }
            .navigationTitle("Mandelbulb3D")
            .toolbar {
                ToolbarItemGroup(placement: .primaryAction) {
                    Button("Paste parameters", systemImage: "doc.on.clipboard") { studio.pasteParameters() }
                    Button("Open", systemImage: "folder") { importing = true }
                }
            }
        } detail: {
            PictureView(studio: studio)
                .navigationTitle(studio.title)
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItemGroup(placement: .secondaryAction) {
                        Button("Undo", systemImage: "arrow.uturn.backward") { studio.undo() }
                            .disabled(!studio.canUndo)
                            .keyboardShortcut("z", modifiers: .command)
                        Button("Redo", systemImage: "arrow.uturn.forward") { studio.redo() }
                            .disabled(!studio.canRedo)
                            .keyboardShortcut("z", modifiers: [.command, .shift])
                    }
                    ToolbarItemGroup(placement: .primaryAction) {
                        if studio.rendering {
                            Button("Stop", systemImage: "stop.fill") { studio.stop() }
                        } else {
                            Button("Render", systemImage: "arrow.clockwise") { studio.render() }
                                .disabled(studio.selected == nil)
                        }
                        if let url = studio.pictureURL {
                            ShareLink(item: url, preview: SharePreview(studio.title))
                        }
                        Menu("File", systemImage: "doc") {
                            Button("Save", systemImage: "square.and.arrow.down") { studio.save() }
                            Button("Export…", systemImage: "square.and.arrow.up.on.square") {
                                exportDoc = studio.exportDocument()
                                exporting = exportDoc != nil
                            }
                            Button("Save to Photos", systemImage: "photo.badge.plus") { studio.saveToPhotos() }
                                .disabled(studio.image == nil)
                            Divider()
                            Button("Copy parameters", systemImage: "doc.on.doc") { studio.copyParameters() }
                            Button("Paste parameters", systemImage: "doc.on.clipboard") { studio.pasteParameters() }
                            Divider()
                            Button("Open from Files…", systemImage: "folder") { importing = true }
                        }
                        .disabled(studio.selected == nil)
                        Button("Settings", systemImage: "slider.horizontal.3") { showSettings.toggle() }
                    }
                }
                .inspector(isPresented: $showSettings) {
                    SettingsView(studio: studio)
                        .inspectorColumnWidth(min: 280, ideal: 320, max: 400)
                }
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.m3p, .m3i, .plainText]) { result in
            if case .success(let url) = result { studio.importFile(url) }
        }
        .fileExporter(isPresented: $exporting, document: exportDoc, contentType: .m3p,
                      defaultFilename: studio.title) { result in
            if case .failure(let e) = result { studio.error = e.localizedDescription }
        }
        .onOpenURL { studio.importFile($0) }
        .onAppear(perform: autoOpen)
    }

    /// `-open NAME` (the build machine's screenshot run) opens that example.
    private func autoOpen() {
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-open"), i + 1 < args.count,
           let url = studio.examples.first(where: { $0.deletingPathExtension().lastPathComponent == args[i + 1] }) {
            studio.open(url)
            if args.contains("-demo") {
                Task { await studio.demo() }
            }
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
            }
            if studio.selected != nil {
                GestureLayer(studio: studio)
            } else {
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
