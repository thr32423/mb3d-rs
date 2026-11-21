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
        status = "Breeding"
        generation += 1
        let seed = UInt64.random(in: 1...UInt64.max)
        Task.detached(priority: .userInitiated) {
            let result = Result { try job.variations(p, seed: seed) }
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
        status = "Pictures"
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

struct VariationsView: View {
    let studio: Studio
    @State private var breeder = Breeder()
    @Environment(\.dismiss) private var dismiss

    private let columns = Array(repeating: GridItem(.flexible(), spacing: 10), count: 5)

    var body: some View {
        NavigationStack {
            ScrollView {
                LazyVGrid(columns: columns, spacing: 10) {
                    ForEach(breeder.members) { m in
                        cell(m)
                    }
                }
                .padding()
                if breeder.members.isEmpty && !breeder.busy {
                    ContentUnavailableView("No variations", systemImage: "leaf",
                                           description: Text("Breed a generation from the scene."))
                }
            }
            .overlay(alignment: .bottom) {
                if let p = breeder.progress {
                    VStack(spacing: 6) {
                        ProgressView(value: p)
                        Text(breeder.status).font(.caption).foregroundStyle(.secondary)
                    }
                    .padding()
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
                    Button("Breed", systemImage: "leaf.arrow.triangle.circlepath") {
                        if let s = chosenScene ?? studio.currentScene { breeder.breed(from: s) }
                    }
                    .disabled(breeder.busy)
                    Button("Use") {
                        if let s = chosenScene {
                            breeder.stop()
                            studio.takeVariation(s)
                            dismiss()
                        }
                    }
                    .disabled(chosenScene == nil)
                }
            }
            .onAppear {
                if breeder.members.isEmpty, let s = studio.currentScene { breeder.breed(from: s) }
            }
        }
    }

    private var chosenScene: FractalScene? {
        breeder.chosen.flatMap { i in breeder.members.first { $0.id == i }?.scene }
    }

    private func cell(_ m: Breeder.Member) -> some View {
        VStack(spacing: 4) {
            ZStack {
                Color.black
                if let img = m.image {
                    Image(decorative: img, scale: 1).resizable().aspectRatio(contentMode: .fit)
                } else {
                    ProgressView()
                }
            }
            .aspectRatio(1, contentMode: .fit)
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .overlay(RoundedRectangle(cornerRadius: 8)
                .stroke(breeder.chosen == m.id ? Color.accentColor : .clear, lineWidth: 3))
            Text(m.id == 0 ? "\(m.label) (parent)" : m.label)
                .font(.caption.monospacedDigit())
                .foregroundStyle(.secondary)
        }
        .contentShape(Rectangle())
        .onTapGesture(count: 2) {
            breeder.chosen = m.id
            breeder.breed(from: m.scene)
        }
        .onTapGesture { breeder.chosen = m.id }
        .accessibilityLabel("Variation \(m.label)")
        .accessibilityAddTraits(breeder.chosen == m.id ? .isSelected : [])
    }
}
