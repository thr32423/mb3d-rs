import SwiftUI
import UniformTypeIdentifiers

/// The app's one screen: the picture fills it; the settings' groups are
/// icons on a rail (at the right, at the bottom on an upright iPad) and
/// the group chosen opens in a panel beside the picture, as do the
/// animation and the 3D model. While a value is held the rail and the
/// panel fade, so the picture shows the change. The interface is dark
/// around the picture, whatever the system's appearance.
struct ContentView: View {
    @Bindable var studio: Studio
    /// the panel of an upright iPad drawn up higher
    @State private var raised = false
    @State private var exporting = false
    @State private var exportDoc: M3PDocument?
    /// the gestures were shown once
    @AppStorage("seenGestures") private var seenGestures = false

    var body: some View {
        NavigationStack {
            GeometryReader { geo in
                workspace(geo.size)
            }
            .background(Color.black.ignoresSafeArea())
            .navigationTitle(studio.title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(.visible, for: .navigationBar)
            .toolbar(studio.pictureOnly ? .hidden : .visible, for: .navigationBar)
            .toolbar {
                ToolbarItemGroup(placement: .topBarLeading) {
                    Button("Scenes", systemImage: "square.grid.2x2") { studio.showScenes = true }
                    Button("Undo", systemImage: "arrow.uturn.backward") { studio.undo() }
                        .disabled(!studio.canUndo)
                        .keyboardShortcut("z", modifiers: .command)
                    Button("Redo", systemImage: "arrow.uturn.forward") { studio.redo() }
                        .disabled(!studio.canRedo)
                        .keyboardShortcut("z", modifiers: [.command, .shift])
                }
                ToolbarItemGroup(placement: .topBarTrailing) {
                    Menu("Create", systemImage: "wand.and.stars") {
                        Button("Variations", systemImage: "leaf") { studio.breeding = true }
                        Button("Animation", systemImage: "film") { show(.animation) }
                        Button("3D model and AR", systemImage: "cube.transparent") { show(.model) }
                    }
                    .disabled(studio.selected == nil)
                    Menu("Share", systemImage: "square.and.arrow.up") {
                        if let url = studio.pictureURL {
                            ShareLink(item: url, preview: SharePreview(studio.title)) {
                                Label("Share the picture", systemImage: "square.and.arrow.up")
                            }
                        }
                        Button("Save to Photos", systemImage: "photo.badge.plus") { studio.saveToPhotos() }
                            .disabled(studio.image == nil)
                        Divider()
                        Button("Save", systemImage: "square.and.arrow.down") { studio.save() }
                        Button("Export…", systemImage: "square.and.arrow.up.on.square") {
                            exportDoc = studio.exportDocument()
                            exporting = exportDoc != nil
                        }
                        Divider()
                        Button("Copy parameters", systemImage: "doc.on.doc") { studio.copyParameters() }
                    }
                    .disabled(studio.selected == nil)
                    renderButton
                }
            }
        }
        .preferredColorScheme(.dark)
        .fileExporter(isPresented: $exporting, document: exportDoc, contentType: .m3p,
                      defaultFilename: studio.title) { result in
            if case .failure(let e) = result { studio.error = e.localizedDescription }
        }
        .sheet(isPresented: $studio.showScenes) {
            ScenesView(studio: studio).preferredColorScheme(.dark)
        }
        .fullScreenCover(isPresented: $studio.breeding) {
            VariationsView(studio: studio).preferredColorScheme(.dark)
        }
        .sensoryFeedback(.success, trigger: studio.finishedCount)
        .sensoryFeedback(.impact, trigger: studio.focusCount)
        .sensoryFeedback(.selection, trigger: studio.panel)
        .onOpenURL {
            studio.showScenes = false
            studio.importFile($0)
        }
        .onChange(of: studio.selected) {
            // the gestures once, with the first scene (not on the build
            // machine's screenshots)
            if !seenGestures, studio.selected != nil, Tour.argument("-open") == nil {
                seenGestures = true
                studio.showHelp = true
            }
        }
        .onAppear(perform: autoOpen)
    }

    private func show(_ panel: Studio.Panel?) {
        withAnimation(.snappy) { studio.panel = panel }
    }

    /// Render, the one filled button; while a full picture is rendered it
    /// is Stop inside a ring that fills with the progress.
    @ViewBuilder
    private var renderButton: some View {
        if studio.busy {
            Button { studio.stop() } label: {
                ZStack {
                    Circle().stroke(Color.secondary.opacity(0.3), lineWidth: 3)
                    Circle().trim(from: 0, to: studio.progress ?? 0)
                        .stroke(Color.accentColor, style: StrokeStyle(lineWidth: 3, lineCap: .round))
                        .rotationEffect(.degrees(-90))
                    Image(systemName: "stop.fill").font(.caption)
                }
                .frame(width: 28, height: 28)
            }
            .accessibilityLabel("Stop")
        } else {
            Menu {
                Button("Render", systemImage: "play.fill") { studio.render() }
                Button("Monte Carlo (soft light)", systemImage: "sun.haze") { studio.renderMonteCarlo() }
            } label: {
                Label("Render", systemImage: "play.fill").labelStyle(.titleAndIcon)
            } primaryAction: {
                studio.render()
            }
            .buttonStyle(.borderedProminent)
            .disabled(studio.selected == nil)
        }
    }

    // MARK: the picture, the rail and the panel

    @ViewBuilder
    private func workspace(_ size: CGSize) -> some View {
        let wide = size.width > size.height
        let panelWidth = max(240, min(360, size.width - 92))
        let panelHeight = size.height * (raised ? 0.62 : 0.4)
        let bare = studio.pictureOnly
        let open = studio.panel != nil && !bare
        ZStack {
            // the picture keeps clear of the open panel
            PictureView(studio: studio)
                .overlay(alignment: .bottom) {
                    if studio.panel == .animation, !studio.director.turntable, !bare {
                        KeyframeStrip(studio: studio, director: studio.director)
                            .frame(maxWidth: 620)
                            .padding(.horizontal, 10)
                            .padding(.bottom, wide || open ? 10 : 76)
                            .opacity(studio.adjusting == nil ? 1 : 0.02)
                    }
                }
                .padding(.trailing, wide && open ? panelWidth + 80 : 0)
                .padding(.bottom, !wide && open ? panelHeight + 76 : 0)
            if !bare {
                Group {
                    if wide {
                        HStack(spacing: 10) {
                            Spacer(minLength: 0)
                            if let panel = studio.panel {
                                self.panel(panel)
                                    .frame(width: panelWidth)
                                    .transition(.move(edge: .trailing).combined(with: .opacity))
                            }
                            rail(upright: true)
                        }
                    } else {
                        VStack(spacing: 10) {
                            Spacer(minLength: 0)
                            if let panel = studio.panel {
                                VStack(spacing: 0) {
                                    handle
                                    self.panel(panel)
                                }
                                .frame(height: panelHeight)
                                .transition(.move(edge: .bottom).combined(with: .opacity))
                            }
                            rail(upright: false)
                        }
                    }
                }
                .padding(10)
                .opacity(studio.adjusting == nil ? 1 : 0.02)
                .animation(.easeOut(duration: 0.15), value: studio.adjusting == nil)
                if studio.selected != nil {
                    viewControls
                        .padding(10)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                        .opacity(studio.adjusting == nil ? 1 : 0.02)
                }
            } else {
                Button { studio.pictureOnly = false } label: {
                    Image(systemName: "arrow.down.right.and.arrow.up.left")
                        .frame(width: 40, height: 40)
                        .background(.regularMaterial, in: Circle())
                }
                .buttonStyle(.plain)
                .opacity(0.5)
                .accessibilityLabel("Picture only")
                .padding(10)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
            }
            if let a = studio.adjusting {
                AdjustingView(adjusting: a)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                    .padding(.top, 12)
                    .allowsHitTesting(false)
            }
            if studio.showHelp {
                GesturesCard { studio.showHelp = false }
            }
        }
    }

    @ViewBuilder
    private func panel(_ p: Studio.Panel) -> some View {
        Group {
            switch p {
            case .settings(let page):
                SettingsPanel(studio: studio, page: page) { show(nil) }
            case .animation:
                AnimationView(studio: studio, director: studio.director) { show(nil) }
            case .model:
                SculptureView(studio: studio, sculptor: studio.sculptor) { show(nil) }
            }
        }
        .clipShape(RoundedRectangle(cornerRadius: 20, style: .continuous))
        .shadow(color: .black.opacity(0.35), radius: 14, y: 4)
    }

    /// The upright panel's handle: up for more room, down for less, then
    /// away.
    private var handle: some View {
        Capsule()
            .fill(Color.white.opacity(0.6))
            .frame(width: 40, height: 5)
            .frame(maxWidth: .infinity)
            .frame(height: 22)
            .contentShape(Rectangle())
            .onTapGesture { withAnimation(.snappy) { raised.toggle() } }
            .gesture(DragGesture(minimumDistance: 8).onEnded { g in
                withAnimation(.snappy) {
                    if g.translation.height < -24 {
                        raised = true
                    } else if g.translation.height > 24 {
                        if raised { raised = false } else { studio.panel = nil }
                    }
                }
            })
            .accessibilityHidden(true)
    }

    /// The settings' groups as icons: a tap opens the group's panel, a tap
    /// on the open one closes it. A dot marks a group with something
    /// switched on.
    private func rail(upright: Bool) -> some View {
        let icons = ForEach(SettingsPage.allCases) { p in
            let on = studio.panel == .settings(p)
            Button {
                show(on ? nil : .settings(p))
            } label: {
                Image(systemName: p.icon)
                    .font(.title3)
                    .frame(width: 46, height: 46)
                    .foregroundStyle(on ? Color.white : Color.primary)
                    .background(on ? Color.accentColor : Color.clear,
                                in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                    .overlay(alignment: .topTrailing) {
                        if !on, studio.isActive(p) {
                            Circle().fill(Color.accentColor).frame(width: 7, height: 7).padding(5)
                        }
                    }
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .hoverEffect()
            .help(p.title)
            .accessibilityLabel(p.title)
            .accessibilityAddTraits(on ? .isSelected : [])
        }
        return Group {
            if upright {
                ViewThatFits(in: .vertical) {
                    VStack(spacing: 4) { icons }.padding(6)
                    ScrollView(.vertical, showsIndicators: false) { VStack(spacing: 4) { icons }.padding(6) }
                }
            } else {
                ViewThatFits(in: .horizontal) {
                    HStack(spacing: 4) { icons }.padding(6)
                    ScrollView(.horizontal, showsIndicators: false) { HStack(spacing: 4) { icons }.padding(6) }
                }
            }
        }
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
        .disabled(studio.selected == nil)
    }

    /// On the picture: back to the opened view, the views kept, a closer
    /// look at the finished picture, the gestures.
    private var viewControls: some View {
        VStack(spacing: 4) {
            // the compass: where the scene's x, y and z point in the
            // picture; a tap goes back to the opened view
            Button { studio.resetView() } label: {
                Compass(axes: studio.axes).frame(width: 40, height: 40).contentShape(Rectangle())
            }
            .accessibilityLabel("Opened view")
            .help("Opened view")
            Menu {
                Button("Keep this view", systemImage: "plus") { studio.keepView() }
                if !studio.views.isEmpty {
                    Divider()
                    ForEach(studio.views) { v in
                        Button("View \(v.number)") { studio.go(to: v.text) }
                    }
                    Divider()
                    Button("Forget the views", systemImage: "trash", role: .destructive) { studio.views = [] }
                }
            } label: {
                controlIcon("bookmark", on: false)
            }
            .accessibilityLabel("Views")
            Button { studio.inspecting.toggle() } label: { controlIcon("magnifyingglass", on: studio.inspecting) }
                .accessibilityLabel("Look closer")
                .help("Look closer")
            Button { studio.showHelp = true } label: { controlIcon("questionmark", on: false) }
                .accessibilityLabel("Gestures")
                .help("Gestures")
        }
        .buttonStyle(.plain)
        .padding(5)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    private func controlIcon(_ name: String, on: Bool) -> some View {
        Image(systemName: name)
            .frame(width: 40, height: 40)
            .foregroundStyle(on ? Color.white : Color.primary)
            .background(on ? Color.accentColor : Color.clear, in: RoundedRectangle(cornerRadius: 11, style: .continuous))
            .contentShape(Rectangle())
            .hoverEffect()
    }

    /// `-open NAME` (the build machine's screenshot run) opens that example.
    private func autoOpen() {
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-open"), i + 1 < args.count,
           let url = studio.examples.first(where: { $0.deletingPathExtension().lastPathComponent == args[i + 1] }) {
            studio.open(url)
            switch Tour.sheet {
            case "variations": studio.breeding = true
            case "animation": studio.panel = .animation
            case "curves":
                // two keyframes (the view and a closer one), then the curves
                Task {
                    while studio.currentScene == nil { try? await Task.sleep(for: .milliseconds(200)) }
                    try? await Task.sleep(for: .seconds(2))
                    if let s = studio.currentScene {
                        studio.director.add(s, thumbnail: studio.image)
                        var t = SceneText(s.text)
                        t["zoom"] = SceneText.format((t.number("zoom") ?? 1) * 3)
                        if let closer = try? FractalScene(text: t.text) { studio.director.add(closer, thumbnail: studio.image) }
                    }
                    studio.panel = .animation
                }
            case "model": studio.panel = .model
            default: break
            }
            if args.contains("-demo") {
                Task { await studio.demo() }
            }
        }
        if studio.selected == nil { studio.showScenes = true }
    }
}

/// The value a finger is changing, alone over the picture.
struct AdjustingView: View {
    let adjusting: Studio.Adjusting

    var body: some View {
        VStack(spacing: 8) {
            HStack {
                Text(adjusting.title)
                Spacer()
                Text(adjusting.value).monospacedDigit()
            }
            .font(.headline)
            if let f = adjusting.fraction {
                ProgressView(value: min(max(f, 0), 1))
            }
        }
        .padding(14)
        .frame(width: 280)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }
}

/// The picture's gestures, shown with the first scene and from the
/// question mark on the picture.
struct GesturesCard: View {
    let close: () -> Void

    private let rows: [(String, LocalizedStringKey)] = [
        ("hand.point.up.left", "One finger: look around"),
        ("hand.draw", "Two fingers: slide"),
        ("arrow.up.left.and.arrow.down.right", "Pinch: fly forward and back"),
        ("arrow.triangle.2.circlepath", "Twist: roll"),
        ("hand.tap", "Double tap: fly to that place"),
        ("scope", "Tap: focus there (with depth of field on)"),
        ("arrow.uturn.backward", "Tap with two fingers: undo; with three: redo"),
        ("clock.arrow.circlepath", "Hold still: the picture as it was opened"),
    ]

    var body: some View {
        ZStack {
            Color.black.opacity(0.45).ignoresSafeArea().onTapGesture(perform: close)
            VStack(alignment: .leading, spacing: 14) {
                Text("Gestures").font(.title3.weight(.semibold))
                ForEach(rows, id: \.0) { row in
                    Label { Text(row.1) } icon: { Image(systemName: row.0).frame(width: 28) }
                }
                Button("OK", action: close)
                    .buttonStyle(.borderedProminent)
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
            .padding(22)
            .frame(maxWidth: 420)
            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
            .padding()
        }
    }
}

/// The picture with its gestures; what the app is doing in a line at the
/// top (the render's progress is the ring in the toolbar). With "Look
/// closer" on, the fingers zoom and move the finished picture and leave
/// the camera alone.
struct PictureView: View {
    let studio: Studio
    @State private var zoom: CGFloat = 1
    @State private var offset: CGSize = .zero
    @GestureState private var pinch: CGFloat = 1
    @GestureState private var drag: CGSize = .zero
    @State private var showDetails = false

    /// The scene in a few lines: its name, the file's and the picture's
    /// size, the formulas.
    private var details: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(verbatim: studio.title).font(.headline)
            LabeledContent("File size", value: "\(studio.pictureSize.width) × \(studio.pictureSize.height)")
            if let image = studio.image {
                LabeledContent("Picture", value: "\(image.width) × \(image.height)")
            }
            Divider()
            Text("Formulas").font(.subheadline.weight(.semibold))
            ForEach(studio.doc.formulas) { f in
                Text(verbatim: "\(f["name"] ?? "")  × \(f["iterations"] ?? "1")").font(.callout)
            }
        }
        .padding()
        .frame(minWidth: 260, alignment: .leading)
        .presentationCompactAdaptation(.popover)
    }

    var body: some View {
        let before = studio.comparing ? studio.before : nil
        ZStack {
            if let image = before ?? studio.image {
                Image(decorative: image, scale: 1)
                    .resizable()
                    .interpolation(.high)
                    .aspectRatio(contentMode: .fit)
                    .scaleEffect(studio.inspecting ? zoom * pinch : 1)
                    .offset(studio.inspecting ? CGSize(width: offset.width + drag.width, height: offset.height + drag.height)
                                              : .zero)
            }
            if studio.selected == nil {
                ContentUnavailableView {
                    Label("Choose an example", systemImage: "cube.transparent")
                } actions: {
                    Button("Scenes") { studio.showScenes = true }.buttonStyle(.borderedProminent)
                }
            } else if studio.inspecting {
                Color.clear
                    .contentShape(Rectangle())
                    .gesture(
                        MagnifyGesture()
                            .updating($pinch) { v, state, _ in state = v.magnification }
                            .onEnded { v in zoom = min(max(zoom * v.magnification, 1), 24) }
                            .simultaneously(with: DragGesture()
                                .updating($drag) { v, state, _ in state = v.translation }
                                .onEnded { v in
                                    offset.width += v.translation.width
                                    offset.height += v.translation.height
                                })
                    )
                    .onTapGesture(count: 2) {
                        withAnimation(.snappy) {
                            zoom = 1
                            offset = .zero
                        }
                    }
            } else {
                GestureLayer(studio: studio)
                if let h = studio.hover, studio.doc["dof"].map({ $0 != "off" }) ?? false {
                    Circle()
                        .stroke(Color.white.opacity(0.9), lineWidth: 1.5)
                        .frame(width: 30, height: 30)
                        .position(h)
                        .allowsHitTesting(false)
                }
            }
            if let line = line, studio.adjusting == nil, !studio.pictureOnly {
                // (a tap: what the scene is)
                Button { showDetails = true } label: {
                    Text(line)
                        .font(.caption.monospacedDigit())
                        .padding(.horizontal, 10)
                        .padding(.vertical, 6)
                        .background(.regularMaterial, in: Capsule())
                }
                .buttonStyle(.plain)
                .popover(isPresented: $showDetails) { details }
                .padding(.top, 12)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .clipped()
        .onChange(of: studio.inspecting) {
            zoom = 1
            offset = .zero
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

    /// What the picture is: the one from before, a quick preview, the
    /// render's stage and part, or the finished picture's size and time.
    private var line: String? {
        if studio.comparing, studio.before != nil { return String(localized: "Before") }
        if studio.busy, let p = studio.progress {
            var t = "\(studio.status) · \(Int((p * 100).rounded())) %"
            if let left = studio.timeLeft {
                t += left < 90 ? " · ~\(Int(left.rounded())) s" : " · ~\(Int((left / 60).rounded())) min"
            }
            return t
        }
        if !studio.status.isEmpty { return studio.status }
        if studio.showsPreview, studio.image != nil { return String(localized: "Preview") }
        return nil
    }
}

/// The compass: the scene's x (red), y (green) and z (blue) directions as
/// the camera sees them; the ones pointing away from the viewer are faint.
struct Compass: View {
    /// the picture's right, down and depth in the scene (9 numbers)
    let axes: [Double]

    var body: some View {
        Canvas { ctx, size in
            guard axes.count == 9 else { return }
            let c = CGPoint(x: size.width / 2, y: size.height / 2)
            let r = min(size.width, size.height) / 2 - 5
            let colours: [Color] = [.red, .green, .blue]
            // the far ones first, so the near ones lie over them
            for i in [0, 1, 2].sorted(by: { axes[6 + $0] > axes[6 + $1] }) {
                let away = axes[6 + i] > 0
                let end = CGPoint(x: c.x + CGFloat(axes[i]) * r, y: c.y + CGFloat(axes[3 + i]) * r)
                var line = Path()
                line.move(to: c)
                line.addLine(to: end)
                let colour = colours[i].opacity(away ? 0.4 : 1)
                ctx.stroke(line, with: .color(colour), style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
                ctx.fill(Path(ellipseIn: CGRect(x: end.x - 3.5, y: end.y - 3.5, width: 7, height: 7)), with: .color(colour))
            }
        }
        .accessibilityHidden(true)
    }
}
