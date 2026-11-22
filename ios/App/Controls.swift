import SwiftUI

/// A light's direction as a dot dragged over a ball: right and left for
/// the side it comes from, up and down for the height. Inside the dashed
/// ring the light comes from the front (up to 90°), outside it from behind.
struct LightBall: View {
    @Binding var side: Double
    @Binding var above: Double
    let colour: CGColor
    let done: () -> Void

    @Environment(Studio.self) private var studio: Studio?
    @GestureState private var dragging = false
    @State private var held = false
    private let size: CGFloat = 172

    var body: some View {
        let r = size / 2
        let d = dot(r)
        VStack(spacing: 6) {
            ZStack {
                Circle().fill(Color.secondary.opacity(0.14))
                // the object, lit from the light's side
                Circle()
                    .fill(RadialGradient(colors: [Color(white: 0.85), Color(white: 0.2)],
                                         center: UnitPoint(x: 0.5 + d.x / r * 0.35, y: 0.5 + d.y / r * 0.35),
                                         startRadius: 2, endRadius: r * 0.7))
                    .frame(width: r, height: r)
                Circle()
                    .stroke(Color.secondary.opacity(0.6), style: StrokeStyle(lineWidth: 1, dash: [3, 3]))
                    .frame(width: r, height: r)
                Circle()
                    .fill(Color(cgColor: colour))
                    .overlay(Circle().stroke(Color.white, lineWidth: 2))
                    .frame(width: 24, height: 24)
                    .shadow(radius: 2)
                    .offset(x: d.x, y: d.y)
            }
            .frame(width: size, height: size)
            .contentShape(Circle())
            .gesture(
                DragGesture(minimumDistance: 0)
                    .updating($dragging) { _, state, _ in state = true }
                    .onChanged { g in
                        held = true
                        var x = Double(g.location.x - r) / Double(r)
                        var y = Double(g.location.y - r) / Double(r)
                        let n = (x * x + y * y).squareRoot()
                        if n > 1 { x /= n; y /= n }
                        side = (x * 180).rounded()
                        above = (-y * 180).rounded()
                        studio?.adjusting = Studio.Adjusting(title: "Light", value: text, fraction: nil)
                        studio?.previewSettings()
                    }
            )
            .onChange(of: dragging) {
                if !dragging, held {
                    held = false
                    studio?.adjusting = nil
                    done()
                }
            }
            Text(text).font(.caption.monospacedDigit()).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .accessibilityElement()
        .accessibilityLabel("Light")
        .accessibilityValue(text)
    }

    private var text: String {
        String(format: "%.0f°, %.0f°", side, above)
    }

    /// The dot's place from the middle (180° at the rim).
    private func dot(_ r: CGFloat) -> CGPoint {
        var x = side / 180, y = -above / 180
        let n = (x * x + y * y).squareRoot()
        if n > 1 { x /= n; y /= n }
        return CGPoint(x: x * Double(r), y: y * Double(r))
    }
}

/// The palette as a strip (the diffuse colours, the specular ones in a
/// thin line under them) with its ten stops as handles: a handle dragged
/// moves the stop between its neighbours, the one chosen shows its two
/// colours underneath.
struct PaletteStrip: View {
    let studio: Studio
    @State private var chosen = 0
    @GestureState private var dragging = false
    @State private var held = false
    /// the position at the strip's right end (kept while a stop is dragged)
    @State private var span = 1.0

    var body: some View {
        let stops = studio.stops
        let full = held ? span : Double(max(stops.map(\.position).max() ?? 1, 1))
        VStack(spacing: 12) {
            GeometryReader { geo in
                let w = max(geo.size.width - 24, 1)
                ZStack(alignment: .topLeading) {
                    VStack(spacing: 2) {
                        gradient(stops, full, specular: false).frame(height: 30)
                        gradient(stops, full, specular: true).frame(height: 10)
                    }
                    .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                    .padding(.horizontal, 12)
                    ForEach(stops) { s in
                        Circle()
                            .fill(Color(cgColor: Studio.cgColor(s.diffuse)))
                            .overlay(Circle().stroke(s.id == chosen ? Color.accentColor : Color.white,
                                                     lineWidth: s.id == chosen ? 3 : 1.5))
                            .frame(width: 22, height: 22)
                            .shadow(radius: 1)
                            .frame(width: 30, height: 34)
                            .contentShape(Rectangle())
                            .position(x: 12 + CGFloat(Double(s.position) / full) * w, y: 62)
                            .gesture(
                                DragGesture(minimumDistance: 0, coordinateSpace: .named("palette"))
                                    .updating($dragging) { _, state, _ in state = true }
                                    .onChanged { g in
                                        if !held {
                                            held = true
                                            span = full
                                        }
                                        chosen = s.id
                                        guard abs(g.translation.width) > 2 else { return }
                                        let p = Double(g.location.x - 12) / Double(w) * span
                                        studio.moveStop(s.id, to: Int(p.rounded()), limit: Int(span))
                                    }
                            )
                            .accessibilityLabel("Colour \(s.id + 1)")
                    }
                }
                .coordinateSpace(name: "palette")
            }
            .frame(height: 80)
            .onChange(of: dragging) {
                if !dragging, held {
                    held = false
                    studio.commit()
                }
            }
            if stops.contains(where: { $0.id == chosen }) {
                HStack {
                    Text("Colour \(chosen + 1)")
                    Spacer()
                    ColorPicker("Diffuse", selection: studio.stopColor(chosen, specular: false), supportsOpacity: false)
                        .labelsHidden()
                    ColorPicker("Specular", selection: studio.stopColor(chosen, specular: true), supportsOpacity: false)
                        .labelsHidden()
                }
            }
        }
        .padding(.vertical, 4)
    }

    private func gradient(_ stops: [Studio.Stop], _ full: Double, specular: Bool) -> LinearGradient {
        let sorted = stops.sorted { $0.position < $1.position }
        return LinearGradient(
            stops: sorted.map {
                Gradient.Stop(color: Color(cgColor: Studio.cgColor(specular ? $0.specular : $0.diffuse)),
                              location: min(max(CGFloat(Double($0.position) / full), 0), 1))
            },
            startPoint: .leading, endPoint: .trailing)
    }
}

/// Small pictures for the formula list: the scene with each formula in
/// the slot being chosen (its default values), made while its row shows,
/// the row seen last first. Kept only while the list is open.
@MainActor
@Observable
final class FormulaPreviews {
    private var images: [String: CGImage] = [:]
    @ObservationIgnored private var wanted: [String] = []
    @ObservationIgnored private var failed = Set<String>()
    @ObservationIgnored private var job: RenderJob?
    /// the scene's text with a formula in the slot
    @ObservationIgnored var text: (String) -> String? = { _ in nil }

    func image(_ name: String) -> CGImage? {
        images[name]
    }

    func want(_ name: String) {
        guard images[name] == nil, !failed.contains(name) else { return }
        wanted.removeAll { $0 == name }
        wanted.append(name)
        next()
    }

    func drop(_ name: String) {
        wanted.removeAll { $0 == name }
    }

    func rest() {
        wanted = []
        job?.stop()
    }

    private func next() {
        guard job == nil, let name = wanted.popLast() else { return }
        guard let t = text(name), let s = try? FractalScene(text: t) else {
            failed.insert(name)
            next()
            return
        }
        let (w, h) = s.size
        let k = 96 / Double(max(w, h, 1))
        s.resize(width: max(16, Int(Double(w) * k)), height: max(16, Int(Double(h) * k)))
        let job = RenderJob { _, _ in }
        self.job = job
        Task.detached(priority: .utility) {
            let image = try? job.render(s).image
            await MainActor.run { [weak self] in
                guard let self else { return }
                if let image {
                    self.images[name] = image
                } else if !job.isStopped {
                    self.failed.insert(name)
                }
                self.job = nil
                self.next()
            }
        }
    }
}

/// The formula list: search, the favourites first (a swipe to the right
/// makes one), then the user's own, the fractals, the transforms (MB3D
/// names them with a leading underscore) and the dIFS shapes; the formula
/// in use ticked. Each row shows the scene with that formula.
struct FormulaPicker: View {
    let names: [String]
    let current: String
    /// the user's own formula files
    let own: [String]
    /// the scene's text with a formula in the slot (for the small pictures)
    let preview: (String) -> String?
    let choose: (String) -> Void

    @State private var previews = FormulaPreviews()

    @State private var search = ""
    @AppStorage("favouriteFormulas") private var stored = ""
    @Environment(\.dismiss) private var dismiss

    private var favourites: [String] {
        stored.split(separator: "\n").map(String.init)
    }

    var body: some View {
        let favs = favourites
        let found = search.isEmpty ? names : names.filter { $0.localizedCaseInsensitiveContains(search) }
        let rest = found.filter { !own.contains($0) }
        let shapes = rest.filter { $0.lowercased().hasPrefix("difs") }
        let transforms = rest.filter { $0.hasPrefix("_") }
        let fractals = rest.filter { !$0.hasPrefix("_") && !$0.lowercased().hasPrefix("difs") }
        List {
            if search.isEmpty, !favs.isEmpty {
                Section("Favourites") {
                    ForEach(names.filter { favs.contains($0) }, id: \.self) { row($0, favs) }
                }
            }
            group("Your formulas", found.filter { own.contains($0) }, favs)
            group("Fractals", fractals, favs)
            group("Transforms", transforms, favs)
            group("dIFS shapes", shapes, favs)
        }
        .searchable(text: $search)
        .onAppear { previews.text = preview }
        .onDisappear { previews.rest() }
        .navigationTitle("Formula")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private func group(_ title: LocalizedStringKey, _ items: [String], _ favs: [String]) -> some View {
        if !items.isEmpty {
            Section(title) {
                ForEach(items, id: \.self) { row($0, favs) }
            }
        }
    }

    private func row(_ name: String, _ favs: [String]) -> some View {
        Button {
            choose(name)
            dismiss()
        } label: {
            HStack {
                Color.black
                    .frame(width: 44, height: 44)
                    .overlay {
                        if let image = previews.image(name) {
                            Image(decorative: image, scale: 1).resizable().aspectRatio(contentMode: .fill)
                        }
                    }
                    .clipShape(RoundedRectangle(cornerRadius: 7, style: .continuous))
                Text(name).foregroundStyle(Color.primary)
                Spacer()
                if favs.contains(name) {
                    Image(systemName: "star.fill").font(.caption).foregroundStyle(Color.yellow)
                }
                if name == current {
                    Image(systemName: "checkmark").foregroundStyle(Color.accentColor)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onAppear { previews.want(name) }
        .onDisappear { previews.drop(name) }
        .swipeActions(edge: .leading) {
            Button("Favourite", systemImage: favs.contains(name) ? "star.slash" : "star") {
                var f = favs
                if let i = f.firstIndex(of: name) { f.remove(at: i) } else { f.append(name) }
                stored = f.joined(separator: "\n")
            }
            .tint(.yellow)
        }
    }
}

/// The hybrid's formulas as cards in a row: a tap shows that formula's
/// values underneath; a card dragged onto another takes its place; held,
/// a card moves earlier or later or goes.
struct FormulaCards: View {
    let studio: Studio
    @Binding var chosen: Int?

    var body: some View {
        let formulas = studio.doc.formulas
        let current = formulas.first { $0.id == chosen }?.id ?? formulas.first?.id
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(Array(formulas.enumerated()), id: \.element.id) { i, f in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(verbatim: "\(i + 1)").font(.caption2.monospacedDigit()).foregroundStyle(.secondary)
                        Text(f["name"] ?? "").font(.subheadline.weight(.semibold)).lineLimit(2)
                        Spacer(minLength: 0)
                        Text(verbatim: "× " + (f["iterations"] ?? "1")).font(.caption.monospacedDigit()).foregroundStyle(.secondary)
                    }
                    .padding(10)
                    .frame(width: 124, height: 88, alignment: .topLeading)
                    .background(f.id == current ? Color.accentColor.opacity(0.22) : Color.secondary.opacity(0.12),
                                in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                    .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .stroke(f.id == current ? Color.accentColor : Color.clear, lineWidth: 2))
                    .contentShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                    .onTapGesture { chosen = f.id }
                    .draggable(String(f.id))
                    .dropDestination(for: String.self) { items, _ in
                        guard let from = items.first.flatMap({ Int($0) }), from != f.id else { return false }
                        chosen = studio.placeFormula(from, at: f.id)
                        return true
                    }
                    .contextMenu {
                        Button("Move earlier", systemImage: "arrow.left") { chosen = studio.moveFormula(f.id, by: -1) }
                            .disabled(i == 0)
                        Button("Move later", systemImage: "arrow.right") { chosen = studio.moveFormula(f.id, by: 1) }
                            .disabled(i == formulas.count - 1)
                        if formulas.count > 1 {
                            Button("Remove this formula", systemImage: "minus.circle", role: .destructive) {
                                studio.removeSection(f.id)
                            }
                        }
                    }
                    .accessibilityElement(children: .combine)
                    .accessibilityAddTraits(f.id == current ? .isSelected : [])
                }
                if formulas.count < 6 {
                    Button { studio.addFormula() } label: {
                        Image(systemName: "plus")
                            .font(.title2)
                            .frame(width: 60, height: 88)
                            .background(Color.secondary.opacity(0.12),
                                        in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Add a formula")
                }
            }
            .padding(.vertical, 2)
        }
    }
}
