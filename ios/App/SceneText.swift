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

    /// Switches an effect off: `key = off`, and its settings go too (the
    /// engine makes the effect again from any of them, e.g. a
    /// `ground_reflect` after `ground = off` brings the ground back).
    mutating func switchOff(_ key: String) {
        let prefix = ["clouds": "cloud_", "detail": "detail_"][key] ?? key + "_"
        head.removeAll { line in
            let parts = line.split(separator: "=", maxSplits: 1)
            return parts.count == 2 && parts[0].trimmingCharacters(in: .whitespaces).hasPrefix(prefix)
        }
        self[key] = "off"
    }

    /// The global keys that start with one of `prefixes`, in the text's order.
    func keys(startingWith prefixes: [String]) -> [String] {
        head.compactMap { line in
            let parts = line.split(separator: "=", maxSplits: 1)
            guard parts.count == 2 else { return nil }
            let k = parts[0].trimmingCharacters(in: .whitespaces)
            return prefixes.contains(where: { k.hasPrefix($0) }) ? k : nil
        }
    }

    func number(_ key: String) -> Double? {
        self[key].flatMap { Double($0) }
    }

    var formulas: [Section] { sections.filter { $0.kind == "formula" } }
    var lights: [Section] { sections.filter { $0.kind == "light" } }

    private var nextID: Int { (sections.map(\.id).max() ?? -1) + 1 }

    /// A new section of `kind` after the last one of that kind (or at the
    /// end).
    @discardableResult
    mutating func insert(_ kind: String, _ entries: [Entry]) -> Int {
        let id = nextID
        let s = Section(id: id, kind: kind, entries: entries)
        if let i = sections.lastIndex(where: { $0.kind == kind }) {
            sections.insert(s, at: i + 1)
        } else if kind == "formula", let i = sections.firstIndex(where: { $0.kind == "light" }) {
            sections.insert(s, at: i)
        } else {
            sections.append(s)
        }
        return id
    }

    mutating func remove(_ id: Int) {
        sections.removeAll { $0.id == id }
    }

    /// Component `i` of a value like `a, b, c`.
    func component(_ key: String, _ i: Int) -> Double? {
        let parts = (self[key] ?? "").split(separator: ",")
        return i < parts.count ? Double(parts[i].trimmingCharacters(in: .whitespaces)) : nil
    }

    /// Sets component `i` of `key` (the others from the value or `fallback`).
    mutating func setComponent(_ key: String, _ i: Int, _ v: Double, fallback: [Double]) {
        var parts = fallback
        for k in parts.indices {
            if let x = component(key, k) { parts[k] = x }
        }
        if i < parts.count { parts[i] = v }
        self[key] = parts.map { SceneText.format($0) }.joined(separator: ", ")
    }

    /// A number as the text takes it (no needless decimals).
    static func format(_ v: Double) -> String {
        v == v.rounded() && abs(v) < 1e15 ? String(Int(v)) : String(v)
    }

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
