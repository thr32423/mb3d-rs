import Foundation

/// A scene's text (`key = value` lines, then `[formula]` and `[light]`
/// sections) taken apart for the settings, and put back together: changes
/// to the sections go through the whole text (`FractalScene(text:)`), as
/// a `[light]` line among changes would switch off the lights not listed.
struct SceneText {
    struct Entry: Hashable {
        var key: String
        var value: String
    }

    struct Section: Identifiable {
        let id: Int
        var kind: String
        var entries: [Entry]

        subscript(key: String) -> String? {
            get { entries.first { $0.key == key }?.value }
            set {
                if let i = entries.firstIndex(where: { $0.key == key }) {
                    if let newValue { entries[i].value = newValue } else { entries.remove(at: i) }
                } else if let newValue {
                    entries.append(Entry(key: key, value: newValue))
                }
            }
        }
    }

    /// the lines before the first section (comments kept as they are)
    var head: [String] = []
    var sections: [Section] = []

    init(_ text: String) {
        var current: Section?
        for raw in text.split(separator: "\n", omittingEmptySubsequences: false) {
            let line = raw.trimmingCharacters(in: .whitespaces)
            if line.hasPrefix("[") && line.hasSuffix("]") {
                if let current { sections.append(current) }
                current = Section(id: sections.count, kind: String(line.dropFirst().dropLast()), entries: [])
            } else if current != nil {
                let parts = line.split(separator: "=", maxSplits: 1)
                if parts.count == 2 {
                    current!.entries.append(Entry(key: parts[0].trimmingCharacters(in: .whitespaces),
                                                  value: parts[1].trimmingCharacters(in: .whitespaces)))
                }
            } else {
                head.append(String(raw))
            }
        }
        if let current { sections.append(current) }
    }

    var text: String {
        var out = head.joined(separator: "\n")
        for s in sections {
            out += "\n[\(s.kind)]\n"
            out += s.entries.map { "\($0.key) = \($0.value)" }.joined(separator: "\n")
            out += "\n"
        }
        return out
    }

    /// A global value (a line before the sections).
    subscript(key: String) -> String? {
        get {
            for line in head {
                let parts = line.split(separator: "=", maxSplits: 1)
                if parts.count == 2, parts[0].trimmingCharacters(in: .whitespaces) == key {
                    return parts[1].trimmingCharacters(in: .whitespaces)
                }
            }
            return nil
        }
        set {
            let i = head.firstIndex {
                let parts = $0.split(separator: "=", maxSplits: 1)
                return parts.count == 2 && parts[0].trimmingCharacters(in: .whitespaces) == key
            }
            switch (i, newValue) {
            case (let i?, let v?): head[i] = "\(key) = \(v)"
            case (let i?, nil): head.remove(at: i)
            case (nil, let v?): head.append("\(key) = \(v)")
            default: break
            }
        }
    }

    func number(_ key: String) -> Double? {
        self[key].flatMap { Double($0) }
    }

    var formulas: [Section] { sections.filter { $0.kind == "formula" } }
    var lights: [Section] { sections.filter { $0.kind == "light" } }

    mutating func update(_ s: Section) {
        if let i = sections.firstIndex(where: { $0.id == s.id }) { sections[i] = s }
    }
}

/// `#RRGGBB` and back, for the colour pickers.
enum HexColor {
    static func rgb(_ hex: String) -> (Double, Double, Double)? {
        let h = hex.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: "#", with: "")
        guard h.count == 6, let v = UInt32(h, radix: 16) else { return nil }
        return (Double((v >> 16) & 255) / 255, Double((v >> 8) & 255) / 255, Double(v & 255) / 255)
    }

    static func hex(_ r: Double, _ g: Double, _ b: Double) -> String {
        func c(_ x: Double) -> Int { Int((min(max(x, 0), 1) * 255).rounded()) }
        return String(format: "#%02X%02X%02X", c(r), c(g), c(b))
    }
}
