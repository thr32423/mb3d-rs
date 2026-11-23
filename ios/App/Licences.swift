import SwiftUI

/// Who made the program and under which licence it is given: the LGPL 2.1
/// asks for its text and the source code's address to come with the app.
/// The credits (from the README) and the licence are English, as written.
struct LicencesView: View {
    let close: () -> Void

    private static func resource(_ name: String) -> String {
        guard let url = Bundle.main.url(forResource: name, withExtension: "txt"),
              let text = try? String(contentsOf: url, encoding: .utf8) else { return "" }
        return text
    }
    private let credits: [(String, String)] = resource("Credits")
        .components(separatedBy: "\n\n")
        .compactMap { block in
            let lines = block.trimmingCharacters(in: .whitespacesAndNewlines).components(separatedBy: "\n")
            guard let name = lines.first, !name.isEmpty else { return nil }
            return (name, lines.dropFirst().joined(separator: " "))
        }
    private let licence = resource("LGPL-2.1")
    private var version: String {
        let info = Bundle.main.infoDictionary
        return "\(info?["CFBundleShortVersionString"] as? String ?? "") (\(info?["CFBundleVersion"] as? String ?? ""))"
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    VStack(alignment: .leading, spacing: 6) {
                        Text(verbatim: "Orbit Trap \(version)").font(.headline)
                        Text(verbatim: "Orbit Trap is a port of Mandelbulb3D (MB3D) to the iPad, built on the mb3d-rs engine. It is free software under the GNU Lesser General Public License 2.1, as MB3D is; the whole source code is on GitHub.")
                            .font(.callout)
                    }
                    Link(destination: URL(string: "https://github.com/thr32423/mb3d-rs")!) {
                        Label("Source code", systemImage: "chevron.left.forwardslash.chevron.right")
                    }
                }
                Section {
                    ForEach(credits, id: \.0) { person in
                        VStack(alignment: .leading, spacing: 2) {
                            Text(verbatim: person.0).font(.subheadline.weight(.semibold))
                            Text(verbatim: person.1).font(.caption).foregroundStyle(.secondary)
                        }
                    }
                } header: {
                    Text(verbatim: "Mandelbulb3D was made by")
                }
                Section {
                    Text(verbatim: licence).font(.system(.caption2, design: .monospaced)).textSelection(.enabled)
                } header: {
                    Text(verbatim: "GNU Lesser General Public License 2.1")
                }
            }
            .navigationTitle("Licences and credits")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) { Button("Close", action: close) }
            }
        }
    }
}
