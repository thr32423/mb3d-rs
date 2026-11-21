import SwiftUI
import UniformTypeIdentifiers

extension UTType {
    /// Mandelbulb3D's parameter file (declared in the app's Info.plist).
    static let m3p = UTType(importedAs: "local.mb3d.m3p", conformingTo: .data)
    /// Mandelbulb3D's picture with its parameters.
    static let m3i = UTType(importedAs: "local.mb3d.m3i", conformingTo: .data)
    /// A Mandelbulb3D formula.
    static let m3f = UTType(importedAs: "local.mb3d.m3f", conformingTo: .data)
    /// A Radiance HDR picture (light probes).
    static let hdr = UTType(filenameExtension: "hdr") ?? .image
}

/// The app's own scenes: the Documents folder (shown in the Files app
/// under On My iPad ▸ Mandelbulb3D).
enum Library {
    static let kinds: Set<String> = ["m3p", "m3i", "txt"]

    static var folder: URL {
        FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
    }

    /// The user's own formulas (.m3f) and background pictures.
    static var formulas: URL { folder.appendingPathComponent("Formulas", isDirectory: true) }
    static var backgrounds: URL { folder.appendingPathComponent("Backgrounds", isDirectory: true) }

    /// The .m3f files of the user's formula folder.
    static func ownFormulas() -> [String] {
        let files = (try? FileManager.default.contentsOfDirectory(at: formulas, includingPropertiesForKeys: nil)) ?? []
        return files.filter { $0.pathExtension.lowercased() == "m3f" }.map { $0.deletingPathExtension().lastPathComponent }
            .sorted { $0.localizedStandardCompare($1) == .orderedAscending }
    }

    /// Files brought into `dir` (a file of the same name is replaced).
    static func copy(_ urls: [URL], into dir: URL) throws -> [URL] {
        try urls.map { url in
            let scoped = url.startAccessingSecurityScopedResource()
            defer { if scoped { url.stopAccessingSecurityScopedResource() } }
            let dest = dir.appendingPathComponent(url.lastPathComponent)
            if FileManager.default.fileExists(atPath: dest.path) { try FileManager.default.removeItem(at: dest) }
            try FileManager.default.copyItem(at: url, to: dest)
            return dest
        }
    }

    static func scenes() -> [URL] {
        let files = (try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil)) ?? []
        return files.filter { kinds.contains($0.pathExtension.lowercased()) }
            .sorted { $0.lastPathComponent.localizedStandardCompare($1.lastPathComponent) == .orderedAscending }
    }

    /// `name.ext` in the folder, or `name 2.ext`, … when taken.
    static func freeName(_ name: String, _ ext: String) -> URL {
        var url = folder.appendingPathComponent(name).appendingPathExtension(ext)
        var n = 2
        while FileManager.default.fileExists(atPath: url.path) {
            url = folder.appendingPathComponent("\(name) \(n)").appendingPathExtension(ext)
            n += 1
        }
        return url
    }

    static func contains(_ url: URL) -> Bool {
        url.deletingLastPathComponent().standardizedFileURL.path == folder.standardizedFileURL.path
    }

    /// A file from elsewhere (the Files app, another app) copied in.
    static func bringIn(_ url: URL) throws -> URL {
        if contains(url) { return url }
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        let dest = freeName(url.deletingPathExtension().lastPathComponent, url.pathExtension.lowercased())
        try FileManager.default.copyItem(at: url, to: dest)
        return dest
    }
}

/// A scene as an .m3p file for "Export…" (the Files app's save dialog).
struct M3PDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.m3p] }
    var data: Data

    init(data: Data) {
        self.data = data
    }

    init(configuration: ReadConfiguration) throws {
        data = configuration.file.regularFileContents ?? Data()
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: data)
    }
}
