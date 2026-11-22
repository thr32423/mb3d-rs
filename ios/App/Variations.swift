import CoreGraphics
import SwiftUI

/// MutaGen: generations of 15 variations of the scene (formulas, their
/// values, Julia, iterations changed); the first is the parent. Pick one to
/// breed the next generation from it, or to use it.
@MainActor
@Observable
final class Breeder {
    struct Member: Identifiable {
        let id: Int
        let label: String
        let scene: FractalScene
        var image: CGImage?
    }

    static let labels = ["1", "1.1", "1.2", "1.1.1", "1.2.1", "1.1.2", "1.2.2", "1.1.1.1", "1.1.1.2",
                         "1.2.1.1", "1.2.1.2", "1.1.2.1", "1.1.2.2", "1.2.2.1", "1.2.2.2"]
    /// The longest side of the small pictures.
    static let side = 240

    var members: [Member] = []
    var chosen: Int?
    var progress: Double?
    var status = ""
    var generation = 0
    /// MutaGen's settings, as its window has them: how likely a child gets
    /// other formulas, then how likely and how much the formulas' values,
    /// the Julia constant and the iteration counts change.
    var formulaWeight = 0.75
    var valuesWeight = 1.0
    var valuesStrength = 1.0
    var juliaWeight = 0.5
    var juliaStrength = 1.0
    var iterationsWeight = 0.5
    var iterationsStrength = 1.0

    var mutation: [Double] {
        [formulaWeight, valuesWeight, valuesStrength, juliaWeight, juliaStrength, iterationsWeight, iterationsStrength]
    }

    func resetMutation() {
        (formulaWeight, valuesWeight, valuesStrength) = (0.75, 1, 1)
        (juliaWeight, juliaStrength, iterationsWeight, iterationsStrength) = (0.5, 1, 0.5, 1)
    }
    @ObservationIgnored private var job: RenderJob?

    var busy: Bool { job != nil }

    /// A new generation from `parent` (a copy is bred).
    func breed(from parent: FractalScene) {
        job?.stop()
        guard let p = try? parent.copy() else { return }
        let (w, h) = p.size
        let k = Double(Self.side) / Double(max(w, h, 1))
        p.resize(width: max(16, Int(Double(w) * k)), height: max(16, Int(Double(h) * k)))
        let job = RenderJob { stage, part in
            Task { @MainActor [weak self] in
                guard let self, self.job != nil else { return }
                self.progress = part
                self.status = stage
            }
        }
        self.job = job
        members = []
        chosen = nil
        progress = 0
        status = String(localized: "Breeding")
        generation += 1
        let seed = UInt64.random(in: 1...UInt64.max)
        let mutation = self.mutation
        Task.detached(priority: .userInitiated) {
            let result = Result { try job.variations(p, seed: seed, mutation: mutation) }
            await MainActor.run { [weak self] in self?.bred(job, result) }
        }
    }

    private func bred(_ done: RenderJob, _ result: Result<[FractalScene], Error>) {
        guard job === done else { return }
        switch result {
        case .success(let scenes):
            members = scenes.enumerated().map { i, s in
                Member(id: i, label: i < Self.labels.count ? Self.labels[i] : "\(i + 1)", scene: s)
            }
            renderNext(done)
        case .failure(let e):
            job = nil
            progress = nil
            status = done.isStopped ? "" : "\(e)"
        }
    }

    /// The small pictures, one after the other.
    private func renderNext(_ job: RenderJob) {
        guard self.job === job else { return }
        guard let i = members.firstIndex(where: { $0.image == nil }), let s = try? members[i].scene.copy() else {
            self.job = nil
            progress = nil
            status = ""
            return
        }
        progress = Double(i) / Double(members.count)
        status = String(localized: "Pictures")
        Task.detached(priority: .userInitiated) {
            let r = try? job.render(s)
            await MainActor.run { [weak self] in
                guard let self, self.job === job, i < self.members.count else { return }
                // a failed picture stays black, the next goes on
                self.members[i].image = r?.image ?? Self.black
                self.renderNext(job)
            }
        }
    }

    func stop() {
        job?.stop()
        job = nil
        progress = nil
        status = ""
    }

    nonisolated static let black: CGImage? = {
        let ctx = CGContext(data: nil, width: 1, height: 1, bitsPerComponent: 8, bytesPerRow: 4,
                            space: CGColorSpace(name: CGColorSpace.sRGB)!,
                            bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)
        return ctx?.makeImage()
    }()
}

/// Variations over the whole screen: the scene (or the variation chosen)
/// large, the generation's fourteen others as a grid. A tap chooses one,
/// a double tap or Breed makes the next generation from it, Use takes it;
/// a finger held on the large picture shows the scene to compare.
struct VariationsView: View {
    let studio: Studio
    @State private var breeder = Breeder()
    @State private var comparing = false
    @State private var showMutation = false
    @Environment(\.dismiss) private var dismiss

    private let columns = [GridItem(.adaptive(minimum: 112, maximum: 200), spacing: 10)]

    var body: some View {
        NavigationStack {
            GeometryReader { geo in
                let wide = geo.size.width > geo.size.height
                let layout = wide ? AnyLayout(HStackLayout(alignment: .top, spacing: 16))
                                  : AnyLayout(VStackLayout(spacing: 16))
                layout {
                    large
                        .frame(maxWidth: wide ? geo.size.width * 0.44 : .infinity,
                               maxHeight: wide ? .infinity : geo.size.height * 0.44)
                    ScrollView {
                        LazyVGrid(columns: columns, spacing: 10) {
                            ForEach(breeder.members.dropFirst()) { m in
                                cell(m)
                            }
                        }
                        .padding(2)
                    }
                }
                .padding()
            }
            .background(Color.black.ignoresSafeArea())
            .overlay(alignment: .bottom) {
                if let p = breeder.progress {
                    VStack(spacing: 6) {
                        ProgressView(value: p)
                        Text(breeder.status).font(.caption).foregroundStyle(.secondary)
                    }
                    .padding()
                    .frame(maxWidth: 360)
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
                    .padding()
                }
            }
            .navigationTitle(breeder.generation > 0 ? "Variations, generation \(breeder.generation)" : "Variations")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close") {
                        breeder.stop()
                        dismiss()
                    }
                }
                ToolbarItemGroup(placement: .primaryAction) {
                    if breeder.busy {
                        Button("Stop", systemImage: "stop.fill") { breeder.stop() }
                    }
                    Button("Mutation", systemImage: "slider.horizontal.3") { showMutation = true }
                        .popover(isPresented: $showMutation) { mutation }
                    Button {
                        if let s = chosenScene ?? studio.currentScene { breeder.breed(from: s) }
                    } label: {
                        Label("Breed", systemImage: "leaf.arrow.triangle.circlepath").labelStyle(.titleAndIcon)
                    }
                    .disabled(breeder.busy)
                    Button("Use") {
                        if let s = chosenScene {
                            breeder.stop()
                            studio.takeVariation(s)
                            dismiss()
                        }
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(chosenScene == nil)
                }
            }
            .onAppear {
                if breeder.members.isEmpty, let s = studio.currentScene { breeder.breed(from: s) }
            }
        }
    }

    /// MutaGen's settings for the next generation, as in its window: a
    /// weight (how likely it changes) and a strength (how much).
    private var mutation: some View {
        @Bindable var b = breeder
        return Form {
            Section("Formulas") {
                LabeledSlider(title: "Weight", value: $b.formulaWeight, range: 0...1, format: "%.2f", done: {})
            }
            Section("Values") {
                LabeledSlider(title: "Weight", value: $b.valuesWeight, range: 0...1, format: "%.2f", done: {})
                LabeledSlider(title: "Strength", value: $b.valuesStrength, range: 0...2, format: "%.2f", done: {})
            }
            Section("Julia") {
                LabeledSlider(title: "Weight", value: $b.juliaWeight, range: 0...1, format: "%.2f", done: {})
                LabeledSlider(title: "Strength", value: $b.juliaStrength, range: 0...2, format: "%.2f", done: {})
            }
            Section("Iterations") {
                LabeledSlider(title: "Weight", value: $b.iterationsWeight, range: 0...1, format: "%.2f", done: {})
                LabeledSlider(title: "Strength", value: $b.iterationsStrength, range: 0...2, format: "%.2f", done: {})
            }
            Section {
                Button("MutaGen's own values", systemImage: "arrow.counterclockwise") { breeder.resetMutation() }
            }
        }
        .frame(minWidth: 380, minHeight: 620)
        .presentationCompactAdaptation(.popover)
    }

    private var chosenScene: FractalScene? {
        breeder.chosen.flatMap { i in breeder.members.first { $0.id == i }?.scene }
    }

    /// The variation chosen, large (the scene itself while none is, or
    /// while a finger is held on it).
    private var large: some View {
        let parent = breeder.members.first
        let chosen = breeder.chosen.flatMap { i in breeder.members.first { $0.id == i } }
        let shown = comparing ? parent : chosen ?? parent
        return VStack(spacing: 8) {
            ZStack {
                Color.black
                if let img = shown?.image {
                    Image(decorative: img, scale: 1).resizable().interpolation(.high).aspectRatio(contentMode: .fit)
                } else {
                    ProgressView().tint(.white)
                }
            }
            .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
            .onLongPressGesture(minimumDuration: 0.15, perform: {}, onPressingChanged: { comparing = $0 })
            Text(chosen == nil || comparing ? "The scene" : "Hold the picture to compare with the scene")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private func cell(_ m: Breeder.Member) -> some View {
        ZStack {
            Color.black
            if let img = m.image {
                Image(decorative: img, scale: 1).resizable().aspectRatio(contentMode: .fit)
            } else {
                ProgressView().tint(.white)
            }
        }
        .aspectRatio(1, contentMode: .fit)
        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous)
            .stroke(breeder.chosen == m.id ? Color.accentColor : Color.white.opacity(0.12),
                    lineWidth: breeder.chosen == m.id ? 3 : 1))
        .contentShape(Rectangle())
        .hoverEffect()
        .onTapGesture(count: 2) {
            breeder.chosen = m.id
            breeder.breed(from: m.scene)
        }
        .onTapGesture { breeder.chosen = m.id }
        .accessibilityLabel("Variation \(m.label)")
        .accessibilityAddTraits(breeder.chosen == m.id ? .isSelected : [])
    }
}
