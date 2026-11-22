import SwiftUI

/// A setting of the scene's text as a control: the less used ones of the
/// desktop's windows, listed per page under "More".
enum Field: Identifiable {
    /// key, title, the value when the scene has none
    case number(String, LocalizedStringKey, Double, whole: Bool = false)
    case slider(String, LocalizedStringKey, ClosedRange<Double>, Double, String, log: Bool = false, whole: Bool = false)
    case toggle(String, LocalizedStringKey, Bool = false)
    /// a switch named after what it turns off (on while the key is false)
    case off(String, LocalizedStringKey, Bool = true)
    /// the words the engine takes with their titles
    case choice(String, LocalizedStringKey, [(String, LocalizedStringKey)], String)
    /// a value like `a, b, c`: a title per number
    case vector(String, [LocalizedStringKey], [Double])
    /// one bit of a number of options (and the engine's own options while
    /// the scene has none)
    case bit(String, LocalizedStringKey, Int, Int = 0)

    var id: String {
        switch self {
        case .number(let k, _, _, _), .slider(let k, _, _, _, _, _, _), .toggle(let k, _, _), .off(let k, _, _),
             .choice(let k, _, _, _), .vector(let k, _, _):
            return k
        case .bit(let k, _, let m, _):
            return "\(k).\(m)"
        }
    }
}

struct Fields: View {
    let studio: Studio
    let fields: [Field]

    var body: some View {
        ForEach(fields) { f in
            switch f {
            case .number(let k, let t, let d, let whole):
                NumberField(title: t, value: studio.number(k, d, whole: whole), whole: whole, done: studio.commit)
            case .slider(let k, let t, let r, let d, let format, let log, let whole):
                LabeledSlider(title: t, value: studio.number(k, d, whole: whole), range: r, logarithmic: log,
                              format: format, done: studio.commit)
            case .toggle(let k, let t, let d):
                Toggle(t, isOn: studio.onOff(k, d))
            case .off(let k, let t, let d):
                Toggle(t, isOn: Binding(get: { !studio.onOff(k, d).wrappedValue },
                                        set: { studio.onOff(k, d).wrappedValue = !$0 }))
            case .choice(let k, let t, let options, let d):
                Picker(t, selection: studio.choice(k, d)) {
                    ForEach(options, id: \.0) { o in
                        Text(o.1).tag(o.0)
                    }
                }
            case .vector(let k, let titles, let d):
                ForEach(titles.indices, id: \.self) { i in
                    NumberField(title: titles[i], value: studio.component(k, i, d), done: studio.commit)
                }
            case .bit(let k, let t, let mask, let own):
                Toggle(t, isOn: studio.bit(k, mask, own))
            }
        }
    }
}

/// The settings the scene's text has under some names (the ones the engine
/// writes only while their effect is on), each as its value's shape asks.
struct PresentFields: View {
    let studio: Studio
    let prefixes: [String]
    /// the desktop's names for some of them (the others show their key)
    var titles: [String: LocalizedStringKey] = [:]

    var body: some View {
        ForEach(studio.doc.keys(startingWith: prefixes), id: \.self) { k in
            let value = studio.doc[k] ?? ""
            let parts = value.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
            let title = titles[k] ?? LocalizedStringKey(stringLiteral: k.replacingOccurrences(of: "_", with: " "))
            if ["true", "false", "on", "off"].contains(value.lowercased()) {
                Toggle(title, isOn: studio.onOff(k, false))
            } else if parts.count == 1, Double(parts[0]) != nil {
                NumberField(title: title, value: studio.number(k, 0), done: studio.commit)
            } else if parts.count > 1, parts.allSatisfy({ Double($0) != nil }) {
                ForEach(parts.indices, id: \.self) { i in
                    NumberField(title: LocalizedStringKey(stringLiteral: "\(k.replacingOccurrences(of: "_", with: " ")) \(i + 1)"),
                                value: studio.component(k, i, parts.map { Double($0) ?? 0 }), done: studio.commit)
                }
            } else {
                LabeledContent(title, value: value)
            }
        }
    }
}

/// A page's less used settings, folded away.
struct More<Content: View>: View {
    var title: LocalizedStringKey = "More"
    @ViewBuilder let content: () -> Content

    var body: some View {
        Section {
            DisclosureGroup(title) {
                content()
            }
        }
    }
}
