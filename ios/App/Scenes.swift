import CoreGraphics
import ImageIO
import SwiftUI

/// Small pictures of the scenes for the gallery: rendered one after the
/// other while the gallery is open, kept as PNG files in the app's caches
/// (by the file's name and date, so a scene saved again gets a new one).
@MainActor
@Observable
final class Thumbnails {
    static let shared = Thumbnails()
    /// The longest side of the small pictures.
    nonisolated static let side = 240

    private var images: [String: CGImage] = [:]
    @ObservationIgnored private var queue: [URL] = []
    @ObservationIgnored private var failed = Set<String>()
    @ObservationIgnored private var job: RenderJob?

    func image(_ url: URL) -> CGImage? {
        images[url.path]
    }

    /// The gallery shows this scene: its picture is wanted.
    func want(_ url: URL) {
        guard images[url.path] == nil, !failed.contains(url.path), !queue.contains(url) else { return }
        queue.append(url)
        next()
    }

    /// The gallery closed: the pictures not yet made wait for the next time.
    func rest() {
        queue = []
        job?.stop()
    }

    private func next() {
        guard job == nil, !queue.isEmpty else { return }
        let url = queue.removeFirst()
        let job = RenderJob { _, _ in }
        self.job = job
        Task.detached(priority: .utility) {
            let file = Thumbnails.file(for: url)
            let image = Thumbnails.load(file) ?? Thumbnails.make(url, job, file)
            await MainActor.run {
                let me = Thumbnails.shared
                if let image {
                    me.images[url.path] = image
                } else if !job.isStopped {
                    me.failed.insert(url.path)
                }
                me.job = nil
                me.next()
            }
        }
    }

    nonisolated private static func file(for url: URL) -> URL {
        let dir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Thumbnails", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let date = (try? url.resourceValues(forKeys: [.contentModificationDateKey]))?.contentModificationDate
        let stamp = Int(date?.timeIntervalSince1970 ?? 0)
        let folder = url.deletingLastPathComponent().lastPathComponent
        return dir.appendingPathComponent("\(folder)-\(url.lastPathComponent)-\(stamp).png")
    }

    nonisolated private static func load(_ file: URL) -> CGImage? {
        guard let src = CGImageSourceCreateWithURL(file as CFURL, nil) else { return nil }
        return CGImageSourceCreateImageAtIndex(src, 0, nil)
    }

    nonisolated private static func make(_ url: URL, _ job: RenderJob, _ file: URL) -> CGImage? {
        guard let s = try? FractalScene.open(url) else { return nil }
        let (w, h) = s.size
        let k = Double(side) / Double(max(w, h, 1))
        s.resize(width: max(16, Int(Double(w) * k)), height: max(16, Int(Double(h) * k)))
        guard let image = try? job.render(s).image else { return nil }
        _ = PNG.write(image, to: file)
        return image
    }
}

/// The scenes to open, as pictures: the ones opened last, the user's own
/// and the examples; a search by name; a file from the Files app, or
/// parameters from the clipboard. One's own scenes are copied, renamed and
/// deleted with a long press.
struct ScenesView: View {
    let studio: Studio
    @State private var importing = false
    @State private var search = ""
    @State private var renaming: URL?
    @State private var newName = ""
    @Environment(\.dismiss) private var dismiss

    private let thumbnails = Thumbnails.shared
    private let columns = [GridItem(.adaptive(minimum: 136, maximum: 200), spacing: 14)]

    var body: some View {
        NavigationStack {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 20) {
                    if search.isEmpty, !studio.recents.isEmpty {
                        group("Recent", studio.recents)
                    }
                    if !found(studio.myScenes).isEmpty {
                        group("My scenes", found(studio.myScenes))
                    }
                    if !found(studio.examples).isEmpty {
                        group("Examples", found(studio.examples))
                    }
                    HStack(spacing: 6) {
                        Text("Engine")
                        Text(verbatim: Engine.version)
                    }
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                .padding()
            }
            .searchable(text: $search)
            .navigationTitle("Scenes")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close") { dismiss() }
                }
                ToolbarItemGroup(placement: .primaryAction) {
                    Button("Paste parameters", systemImage: "doc.on.clipboard") {
                        studio.pasteParameters()
                        dismiss()
                    }
                    Button("Open from Files…", systemImage: "folder") { importing = true }
                }
            }
            .fileImporter(isPresented: $importing, allowedContentTypes: [.m3p, .m3i, .plainText]) { result in
                if case .success(let url) = result {
                    studio.importFile(url)
                    dismiss()
                }
            }
            .alert("Rename", isPresented: Binding(
                get: { renaming != nil },
                set: { if !$0 { renaming = nil } }
            )) {
                TextField("Name", text: $newName)
                Button("Rename") {
                    if let url = renaming { studio.rename(url, to: newName) }
                }
                Button("Close", role: .cancel) {}
            }
            .onDisappear { thumbnails.rest() }
        }
    }

    private func found(_ urls: [URL]) -> [URL] {
        search.isEmpty ? urls : urls.filter {
            $0.deletingPathExtension().lastPathComponent.localizedCaseInsensitiveContains(search)
        }
    }

    private func group(_ title: LocalizedStringKey, _ urls: [URL]) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title).font(.headline)
            LazyVGrid(columns: columns, alignment: .leading, spacing: 14) {
                ForEach(urls, id: \.self) { cell($0) }
            }
        }
    }

    private func cell(_ url: URL) -> some View {
        let name = url.deletingPathExtension().lastPathComponent
        let current = url == studio.selected
        return Button {
            if !current { studio.open(url) }
            dismiss()
        } label: {
            VStack(spacing: 6) {
                Color.black
                    .aspectRatio(4.0 / 3, contentMode: .fit)
                    .overlay {
                        if let image = thumbnails.image(url) {
                            Image(decorative: image, scale: 1).resizable().aspectRatio(contentMode: .fill)
                        } else {
                            ProgressView().tint(.white)
                        }
                    }
                    .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
                    .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .stroke(current ? Color.accentColor : Color.clear, lineWidth: 3))
                Text(name).font(.caption).lineLimit(1)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .hoverEffect()
        .onAppear { thumbnails.want(url) }
        .contextMenu {
            if Library.contains(url) {
                Button("Duplicate", systemImage: "plus.square.on.square") { studio.duplicate(url) }
                Button("Rename", systemImage: "pencil") {
                    newName = name
                    renaming = url
                }
                Button("Delete", systemImage: "trash", role: .destructive) { studio.delete(url) }
            }
        }
        .accessibilityLabel(name)
        .accessibilityAddTraits(current ? .isSelected : [])
    }
}
