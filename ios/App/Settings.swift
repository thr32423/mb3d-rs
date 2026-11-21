import CoreGraphics
import PhotosUI
import SwiftUI

/// The inspector: the scene's settings on pages, as the desktop groups
/// them (main window tabs, Formulas, Lighting, Post-process). A slider
/// renders again when the finger lifts, a colour, a switch or a choice a
/// moment after the last change, a number when it is entered.
struct SettingsView: View {
    @Bindable var studio: Studio
    /// the page shown (`-page NAME` opens one at the start: the build
    /// machine's screenshots of the interface)
    @State private var path: [Page] = Tour.page.map { [$0] } ?? []

    enum Page: String, Hashable, CaseIterable {
        case picture, fractal, formulas, julia, lighting, colours, fog, effects, finishing, ground
    }

    var body: some View {
        NavigationStack(path: $path) {
            List {
                Section {
                    link(.picture, "Picture", "photo")
                    link(.fractal, "Fractal", "cube")
                    link(.formulas, "Formulas", "function")
                    link(.julia, "Julia and cutting", "scissors")
                }
                Section {
                    link(.lighting, "Lighting", "lightbulb")
                    link(.colours, "Colours", "paintpalette")
                    link(.fog, "Background and fog", "cloud.fog")
                }
                Section {
                    link(.effects, "Effects", "sparkles")
                    link(.finishing, "Finishing", "camera.filters")
                    link(.ground, "Ground, water and clouds", "water.waves")
                }
                Section {
                    Button("Copy parameters", systemImage: "doc.on.doc") { studio.copyParameters() }
                    Button("Paste parameters", systemImage: "doc.on.clipboard") { studio.pasteParameters() }
                } footer: {
                    Text("Mandelbulb3D text, as on the forums and in the desktop program.")
                }
                Section {
                    LabeledContent("Engine", value: Engine.version)
                }
            }
            .navigationTitle("Settings")
            .navigationBarTitleDisplayMode(.inline)
            .navigationDestination(for: Page.self) { p in
                destination(p).navigationTitle(title(p)).navigationBarTitleDisplayMode(.inline)
            }
        }
        .disabled(studio.selected == nil)
    }

    private func link(_ p: Page, _ title: LocalizedStringKey, _ icon: String) -> some View {
        NavigationLink(value: p) {
            Label(title, systemImage: icon)
        }
    }

    private func title(_ p: Page) -> LocalizedStringKey {
        switch p {
        case .picture: "Picture"
        case .fractal: "Fractal"
        case .formulas: "Formulas"
        case .julia: "Julia and cutting"
        case .lighting: "Lighting"
        case .colours: "Colours"
        case .fog: "Background and fog"
        case .effects: "Effects"
        case .finishing: "Finishing"
        case .ground: "Ground, water and clouds"
        }
    }

    @ViewBuilder
    private func destination(_ p: Page) -> some View {
        switch p {
        case .picture: PicturePage(studio: studio)
        case .fractal: FractalPage(studio: studio)
        case .formulas: FormulasPage(studio: studio)
        case .julia: JuliaPage(studio: studio)
        case .lighting: LightingPage(studio: studio)
        case .colours: ColoursPage(studio: studio)
        case .fog: FogPage(studio: studio)
        case .effects: EffectsPage(studio: studio)
        case .finishing: FinishingPage(studio: studio)
        case .ground: GroundPage(studio: studio)
        }
    }
}

/// `-page NAME` / `-sheet NAME`: the app opens a settings page or a window
/// at the start (the build machine's screenshots of the interface).
enum Tour {
    static func argument(_ flag: String) -> String? {
        let a = ProcessInfo.processInfo.arguments
        guard let i = a.firstIndex(of: flag), i + 1 < a.count else { return nil }
        return a[i + 1]
    }

    static let page = argument("-page").flatMap(SettingsView.Page.init(rawValue:))
    static let sheet = argument("-sheet")
}

// MARK: pages

struct PicturePage: View {
    @Bindable var studio: Studio
    @State private var panoURL: URL?

    var body: some View {
        Form {
            Section("Shape") {
                Picker("Aspect", selection: $studio.aspect) {
                    Text("Square").tag(1.0)
                    Text("4 : 3").tag(4.0 / 3)
                    Text("3 : 2").tag(1.5)
                    Text("16 : 9").tag(16.0 / 9)
                    Text("3 : 4").tag(0.75)
                    Text("9 : 16").tag(9.0 / 16)
                    if ![1.0, 4.0 / 3, 1.5, 16.0 / 9, 0.75, 9.0 / 16].contains(where: { abs($0 - studio.aspect) < 0.001 }) {
                        Text(String(format: "%.2f : 1 (the file's)", studio.aspect)).tag(studio.aspect)
                    }
                }
                LabeledContent("File size", value: "\(studio.pictureSize.width) × \(studio.pictureSize.height)")
            }
            Section {
                Picker("Quality", selection: $studio.renderSide) {
                    Text("Preview (512)").tag(512)
                    Text("Medium (1024)").tag(1024)
                    Text("High (2048)").tag(2048)
                    Text("The file's size").tag(0)
                }
            } footer: {
                Text("The longest side of the rendered picture. Save keeps the file's size.")
            }
            Section("Camera") {
                LabeledSlider(title: "Zoom", value: studio.number("zoom", 1), range: 0.01...20, logarithmic: true,
                              format: "%.3f", done: studio.commit)
                LabeledSlider(title: "Field of view", value: studio.number("fov", 30), range: 5...150, format: "%.0f°",
                              done: studio.commit)
                Picker("Lens", selection: studio.choice("optic", "0")) {
                    Text("Common").tag("0")
                    Text("Planar").tag("1")
                    Text("Panorama").tag("2")
                    Text("Fisheye").tag("3")
                }
            }
            Section {
                Button("Save to Photos", systemImage: "photo.badge.plus") { studio.saveToPhotos() }
                    .disabled(studio.image == nil)
            }
            Section {
                Button("Render a stereo pair", systemImage: "eyeglasses") { studio.renderStereoPair() }
                Toggle("Cross-eyed (else parallel)", isOn: $studio.crossEyed)
            } header: {
                Text("Stereo")
            } footer: {
                Text("The left and the right eye's pictures side by side.")
            }
            Section {
                Button("Make a 360° panorama", systemImage: "pano") { studio.makePanorama() }
                Button("Share as a 360° photo", systemImage: "globe") {
                    if let url = studio.panoramaPhoto() { panoURL = url }
                }
                .disabled(studio.image.map { $0.width != 2 * $0.height } ?? true)
            } header: {
                Text("360°")
            } footer: {
                Text("A panorama all around the camera, tagged so Facebook and photo apps show it as a 360° photo.")
            }
            .sheet(item: $panoURL) { url in
                ShareSheet(items: [url])
            }
        }
    }
}

struct FractalPage: View {
    let studio: Studio

    var body: some View {
        Form {
            Section("Detail") {
                LabeledSlider(title: "Iterations", value: studio.number("iterations", 30, whole: true), range: 1...250,
                              format: "%.0f", done: studio.commit)
                LabeledSlider(title: "Detail (DE stop)", value: studio.detail(), range: 0.05...8,
                              logarithmic: true, format: "%.2f", done: studio.commit)
                LabeledSlider(title: "Ray step", value: studio.number("raystep", 0.5), range: 0.05...1, format: "%.2f",
                              done: studio.commit)
                LabeledSlider(title: "Smooth normals", value: studio.number("smooth_normals", 0, whole: true),
                              range: 0...8, format: "%.0f", done: studio.commit)
                LabeledSlider(title: "Surface search steps", value: studio.number("bin_search", 4, whole: true),
                              range: 0...16, format: "%.0f", done: studio.commit)
            }
            Section("Calculation") {
                Toggle("Vary DE stop with the field of view", isOn: studio.flag("vary_de_stop"))
                Toggle("Normals on DE", isOn: studio.flag("normals_on_de"))
                Toggle("First step random", isOn: studio.flag("first_step_random"))
                Toggle("Ray step minus DE stop", isOn: studio.flag("step_sub_de_stop"))
            }
        }
    }
}

struct FormulasPage: View {
    let studio: Studio
    @State private var importing = false

    var body: some View {
        Form {
            ForEach(studio.doc.formulas) { f in
                FormulaSection(studio: studio, section: f)
            }
            Section {
                Button("Add a formula", systemImage: "plus") { studio.addFormula() }
                    .disabled(studio.doc.formulas.count >= 6)
                LabeledSlider(title: "Repeat from formula", value: studio.repeatFrom(),
                              range: 1...Double(max(studio.doc.formulas.count, 2)), format: "%.0f",
                              done: studio.commit)
                Toggle("Disable analytic DE", isOn: studio.flag("disable_analytic_de"))
            } footer: {
                Text("Up to six formulas run one after the other (a hybrid); after the last, the iteration goes on from the one chosen.")
            }
            Section {
                ForEach(studio.ownFormulas, id: \.self) { name in
                    Text(name)
                        .swipeActions {
                            Button("Delete", systemImage: "trash", role: .destructive) { studio.removeFormula(name) }
                        }
                }
                Button("Add formula files (.m3f)", systemImage: "plus.rectangle.on.folder") { importing = true }
            } header: {
                Text("Your formulas")
            } footer: {
                Text("They join the formula list; they are also in the app's Formulas folder in the Files app. Without the desktop's compiler they run slower.")
            }
            .fileImporter(isPresented: $importing, allowedContentTypes: [.m3f], allowsMultipleSelection: true) { r in
                if case .success(let urls) = r { studio.importFormulas(urls) }
            }
        }
    }
}

struct JuliaPage: View {
    let studio: Studio

    var body: some View {
        Form {
            Section {
                Toggle("Julia mode", isOn: studio.flag("julia"))
                ForEach(0..<4) { i in
                    NumberField(title: LocalizedStringKey(stringLiteral: ["x", "y", "z", "w"][i]), value: studio.component("julia_c", i, [0, 0, 0, 0]),
                                done: studio.commit)
                }
            } header: {
                Text("Julia")
            } footer: {
                Text("A Julia set takes this constant in place of each point's own.")
            }
            Section {
                ForEach(["x", "y", "z"], id: \.self) { axis in
                    Toggle("Cut at \(axis)", isOn: studio.effect("cut_\(axis)", on: "0"))
                    if studio.effect("cut_\(axis)", on: "0").wrappedValue {
                        NumberField(title: "Position on \(axis)", value: studio.number("cut_\(axis)", 0), done: studio.commit)
                    }
                }
            } header: {
                Text("Cutting")
            } footer: {
                Text("Cuts the object open at a plane.")
            }
        }
    }
}

struct LightingPage: View {
    let studio: Studio

    var body: some View {
        Form {
            Section("Light") {
                LabeledSlider(title: "Diffuse", value: studio.number("diffuse", 50, whole: true), range: 0...255,
                              format: "%.0f", done: studio.commit)
                LabeledSlider(title: "Specular", value: studio.number("specular", 50, whole: true), range: 0...255,
                              format: "%.0f", done: studio.commit)
                LabeledSlider(title: "Ambient", value: studio.number("ambient", 30, whole: true), range: 0...255,
                              format: "%.0f", done: studio.commit)
                LabeledSlider(title: "Ambient shadow", value: studio.number("ambient_shadow", 53, whole: true),
                              range: 0...255, format: "%.0f", done: studio.commit)
                LabeledSlider(title: "Gamma", value: studio.number("gamma", 32, whole: true), range: 0...100,
                              format: "%.0f", done: studio.commit)
                NumberField(title: "Roughness", value: studio.number("roughness", 255, whole: true), done: studio.commit)
                ColorPicker("Ambient from above", selection: studio.color("ambient_top"), supportsOpacity: false)
                ColorPicker("Ambient from below", selection: studio.color("ambient_bottom"), supportsOpacity: false)
            }
            ForEach(Array(studio.doc.lights.enumerated()), id: \.element.id) { i, l in
                LightSection(studio: studio, section: l, number: i + 1, removable: true)
            }
            Section {
                Button("Add a light", systemImage: "plus") { studio.addLight() }
                    .disabled(studio.doc.lights.count >= 6)
            }
        }
    }
}

struct ColoursPage: View {
    let studio: Studio

    var body: some View {
        Form {
            Section("Palette") {
                ForEach(studio.stops) { s in
                    HStack {
                        Text("Colour \(s.id + 1)")
                        Spacer()
                        ColorPicker("Diffuse", selection: studio.stopColor(s.id, specular: false), supportsOpacity: false)
                            .labelsHidden()
                        ColorPicker("Specular", selection: studio.stopColor(s.id, specular: true), supportsOpacity: false)
                            .labelsHidden()
                    }
                }
            }
            Section {
                NumberField(title: "Colour start", value: studio.number("color_start", 0), done: studio.commit)
                NumberField(title: "Colour end", value: studio.number("color_end", 0), done: studio.commit)
                NumberField(title: "Colour multiplier", value: studio.number("color_mul", 1), done: studio.commit)
                Toggle("Colour cycling", isOn: studio.flag("color_cycling"))
            } footer: {
                Text("Each palette colour has a diffuse (left) and a specular (right) part; start and end place the palette over the surface.")
            }
        }
    }
}

struct FogPage: View {
    let studio: Studio
    @State private var photo: PhotosPickerItem?
    @State private var choosingFile = false

    var body: some View {
        Form {
            Section {
                if studio.backgroundName.isEmpty {
                    LabeledContent("Picture", value: "None")
                } else {
                    LabeledContent("Picture", value: studio.backgroundName)
                }
                PhotosPicker(selection: $photo, matching: .images) {
                    Label("Choose from Photos", systemImage: "photo.on.rectangle")
                }
                Button("Choose a file (JPEG, PNG, HDR)", systemImage: "folder") { choosingFile = true }
                if !studio.backgroundName.isEmpty {
                    Toggle("As full background sphere", isOn: studio.backgroundSphere())
                    LabeledSlider(title: "Brightness", value: studio.number("background_brightness", 40, whole: true),
                                  range: 0...255, format: "%.0f", done: studio.commit)
                    ForEach(0..<3) { i in
                        LabeledSlider(title: (["Turn about x", "Turn about y", "Turn about z"] as [LocalizedStringKey])[i],
                                      value: studio.component("background_rotation", i, [0, 0, 0]),
                                      range: 0...255, format: "%.0f", done: studio.commit)
                    }
                    Toggle("Scale rows to geographic projection", isOn: studio.flag("background_geographic"))
                    Toggle("Fit left and right borders", isOn: studio.flag("background_fit_borders"))
                    Toggle("Smooth the picture", isOn: studio.flag("background_smooth"))
                    Toggle("Add to the background depth", isOn: studio.flag("background_add_light"))
                    Toggle("Use it as the ambient colour", isOn: studio.flag("background_ambient"))
                    Button("Remove the picture", systemImage: "trash", role: .destructive) { studio.removeBackground() }
                }
            } header: {
                Text("Background picture")
            } footer: {
                Text("An HDR picture lights the scene in a Monte Carlo render. Pictures are kept in the app's Backgrounds folder.")
            }
            .onChange(of: photo) {
                guard let item = photo else { return }
                Task {
                    if let data = try? await item.loadTransferable(type: Data.self) {
                        studio.setBackground(imageData: data)
                    }
                    photo = nil
                }
            }
            .fileImporter(isPresented: $choosingFile, allowedContentTypes: [.jpeg, .png, .hdr, .bmp]) { r in
                if case .success(let url) = r { studio.setBackground(file: url) }
            }
            Section("Background") {
                ColorPicker("Near", selection: studio.color("depth_color"), supportsOpacity: false)
                ColorPicker("Far", selection: studio.color("depth_color2"), supportsOpacity: false)
            }
            Section("Depth fog") {
                LabeledSlider(title: "Depth fog", value: studio.number("depth_fog", 0, whole: true), range: 0...255,
                              format: "%.0f", done: studio.commit)
                Toggle("Far fog", isOn: studio.flag("far_fog"))
            }
            Section("Glowing fog") {
                LabeledSlider(title: "Amount (53 = off)", value: studio.number("dyn_fog", 53, whole: true),
                              range: 0...255, format: "%.0f", done: studio.commit)
                ColorPicker("Colour", selection: studio.color("dynfog_color"), supportsOpacity: false)
                ColorPicker("Colour 2", selection: studio.color("dynfog_color2"), supportsOpacity: false)
            }
        }
    }
}

struct EffectsPage: View {
    let studio: Studio

    var body: some View {
        Form {
            Section("Shadows") {
                Picker("Hard shadows", selection: studio.choice("shadows", "off")) {
                    Text("Off").tag("off")
                    Text("All lights").tag("all")
                    if let v = studio.doc["shadows"], !["off", "all"].contains(v) {
                        Text("Lights \(v)").tag(v)
                    }
                }
                Toggle("Soft shadows", isOn: studio.flag("shadow_soft"))
                Picker("Ambient occlusion", selection: studio.choice("ao", "off")) {
                    Text("Off").tag("off")
                    Text("Screen space (15)").tag("ssao15")
                    Text("Screen space (24)").tag("ssao24")
                    Text("Distance estimate").tag("deao")
                    if let v = studio.doc["ao"], !["off", "ssao15", "ssao24", "deao"].contains(v) {
                        Text(v).tag(v)
                    }
                }
            }
            Section("Reflections") {
                Toggle("Reflections", isOn: studio.flag("mc_reflections"))
                LabeledSlider(title: "Amount", value: studio.number("mc_reflection_amount", 0.5), range: 0...1,
                              format: "%.2f", done: studio.commit)
                Toggle("Transparency", isOn: studio.flag("mc_transparency"))
            }
            Section {
                Toggle("Depth of field", isOn: studio.effect("dof", on: "sorted"))
                if studio.effect("dof", on: "sorted").wrappedValue {
                    NumberField(title: "Focus", value: studio.number("dof_focus", 1), done: studio.commit)
                    LabeledSlider(title: "Aperture", value: studio.number("dof_aperture", 0.1), range: 0.001...2,
                                  logarithmic: true, format: "%.3f", done: studio.commit)
                }
            } header: {
                Text("Depth of field")
            } footer: {
                Text("Tap the picture to focus on what is there.")
            }
            Section("Material") {
                LabeledSlider(title: "Edge reflection (Fresnel)", value: studio.number("material_fresnel", 0),
                              range: 0...1, format: "%.2f", done: studio.commit)
                LabeledSlider(title: "Iridescence", value: studio.component("material_iridescence", 0, [0, 0]),
                              range: 0...1, format: "%.2f", done: studio.commit)
                LabeledSlider(title: "Iridescence shift", value: studio.component("material_iridescence", 1, [0, 0]),
                              range: 0...1, format: "%.2f", done: studio.commit)
                LabeledSlider(title: "Glow", value: studio.number("material_glow", 0), range: 0...4, format: "%.2f",
                              done: studio.commit)
            }
            Section {
                Picker("Surface detail", selection: studio.detailKind()) {
                    Text("Off").tag("off")
                    Text("Rough").tag("rough")
                    Text("Stone").tag("stone")
                    Text("Grain").tag("grain")
                }
                if studio.detailKind().wrappedValue != "off" {
                    LabeledSlider(title: "Strength", value: studio.component("detail", 1, [0, 1, 0.05, 0.5]),
                                  range: 0...4, format: "%.2f", done: studio.commit)
                    NumberField(title: "Size", value: studio.component("detail", 2, [0, 1, 0.05, 0.5]),
                                done: studio.commit)
                    LabeledSlider(title: "Fineness", value: studio.component("detail", 3, [0, 1, 0.05, 0.5]),
                                  range: 0...1, format: "%.2f", done: studio.commit)
                }
            } header: {
                Text("Surface detail")
            } footer: {
                Text("A relief over the surface; its size in fractal units, about a twentieth of what the view shows.")
            }
        }
    }
}

struct FinishingPage: View {
    let studio: Studio

    var body: some View {
        Form {
            Section {
                LabeledSlider(title: "Exposure (stops)", value: studio.number("finish_exposure", 0), range: -3...3,
                              format: "%+.1f", done: studio.commit)
                Toggle("Filmic tone map", isOn: studio.effect("finish_tonemap", on: "filmic"))
                LabeledSlider(title: "Contrast", value: studio.number("finish_contrast", 0), range: -1...1,
                              format: "%+.2f", done: studio.commit)
                LabeledSlider(title: "Saturation", value: studio.number("finish_saturation", 1), range: 0...2,
                              format: "%.2f", done: studio.commit)
                LabeledSlider(title: "Warmth", value: studio.number("finish_warmth", 0), range: -1...1,
                              format: "%+.2f", done: studio.commit)
            } footer: {
                Text("A last pass over the picture.")
            }
            Section("Lens") {
                LabeledSlider(title: "Glow", value: studio.component("glow", 0, [0, 0.02, 0.8]), range: 0...2,
                              format: "%.2f", done: studio.commit)
                LabeledSlider(title: "Glow radius", value: studio.component("glow", 1, [0, 0.02, 0.8]),
                              range: 0.002...0.2, logarithmic: true, format: "%.3f", done: studio.commit)
                LabeledSlider(title: "Glow threshold", value: studio.component("glow", 2, [0, 0.02, 0.8]),
                              range: 0...1, format: "%.2f", done: studio.commit)
                LabeledSlider(title: "Vignette", value: studio.number("vignette", 0), range: 0...1, format: "%.2f",
                              done: studio.commit)
                LabeledSlider(title: "Colour fringes", value: studio.number("chromatic", 0), range: 0...1,
                              format: "%.2f", done: studio.commit)
            }
            Section {
                Toggle("Droste effect", isOn: studio.effect("droste", on: "0.5, 0.5, 0.12, 0.45"))
                if studio.effect("droste", on: "0.5, 0.5, 0.12, 0.45").wrappedValue {
                    LabeledSlider(title: "Centre across", value: studio.component("droste", 0, [0.5, 0.5, 0.12, 0.45]), range: 0...1,
                                  format: "%.2f", done: studio.commit)
                    LabeledSlider(title: "Centre down", value: studio.component("droste", 1, [0.5, 0.5, 0.12, 0.45]), range: 0...1,
                                  format: "%.2f", done: studio.commit)
                    LabeledSlider(title: "Inner radius", value: studio.component("droste", 2, [0.5, 0.5, 0.12, 0.45]), range: 0.002...1,
                                  logarithmic: true, format: "%.3f", done: studio.commit)
                    LabeledSlider(title: "Outer radius", value: studio.component("droste", 3, [0.5, 0.5, 0.12, 0.45]), range: 0.01...4,
                                  logarithmic: true, format: "%.3f", done: studio.commit)
                    LabeledSlider(title: "Twist", value: studio.number("droste_twist", 0), range: -8...8,
                                  format: "%.2f", done: studio.commit)
                    LabeledSlider(title: "Phase", value: studio.number("droste_phase", 0), range: -4...4,
                                  format: "%.2f", done: studio.commit)
                    LabeledSlider(title: "Blend", value: studio.number("droste_blend", 0), range: 0...1,
                                  format: "%.2f", done: studio.commit)
                }
            } header: {
                Text("Droste")
            } footer: {
                Text("The picture inside itself, again and again: the ring between the radii repeats inwards.")
            }
        }
    }
}

struct GroundPage: View {
    let studio: Studio
    private let waves = [0.0, 0.4]

    var body: some View {
        Form {
            Section {
                Toggle("Ground plane", isOn: studio.effect("ground", on: "y, 1.2"))
                if studio.effect("ground", on: "y, 1.2").wrappedValue {
                    Picker("Axis", selection: studio.groundAxis()) {
                        Text("x").tag("x")
                        Text("y").tag("y")
                        Text("z").tag("z")
                    }
                    NumberField(title: "Position", value: studio.groundPosition(), done: studio.commit)
                    ColorPicker("Colour", selection: studio.color("ground_color"), supportsOpacity: false)
                    LabeledSlider(title: "Reflection", value: studio.number("ground_reflect", 0), range: 0...1,
                                  format: "%.2f", done: studio.commit)
                    NumberField(title: "Wave height", value: studio.component("ground_waves", 0, waves),
                                done: studio.commit)
                    NumberField(title: "Wave length", value: studio.component("ground_waves", 1, waves),
                                done: studio.commit)
                    LabeledSlider(title: "Transparency (water)", value: studio.number("ground_transparency", 0),
                                  range: 0...1, format: "%.2f", done: studio.commit)
                    LabeledSlider(title: "Murkiness", value: studio.number("ground_absorption", 1), range: 0...5,
                                  format: "%.2f", done: studio.commit)
                    ColorPicker("Deep water", selection: studio.color("ground_water_color"), supportsOpacity: false)
                    LabeledSlider(title: "Light in the water", value: studio.number("ground_scattering", 0),
                                  range: 0...1, format: "%.2f", done: studio.commit)
                }
            } header: {
                Text("Ground and water")
            } footer: {
                Text("Down the screen is +y in most views: try both signs. Water shows with reflections on.")
            }
            Section {
                Toggle("Clouds", isOn: studio.effect("clouds", on: "y, -1.2, -2.2"))
                if studio.effect("clouds", on: "y, -1.2, -2.2").wrappedValue {
                    NumberField(title: "Layer from", value: studio.cloudHeight(0), done: studio.commit)
                    NumberField(title: "Layer to", value: studio.cloudHeight(1), done: studio.commit)
                    LabeledSlider(title: "Density", value: studio.number("cloud_density", 1), range: 0...4,
                                  format: "%.2f", done: studio.commit)
                    LabeledSlider(title: "Coverage", value: studio.number("cloud_coverage", 0.5), range: 0...1,
                                  format: "%.2f", done: studio.commit)
                    NumberField(title: "Size", value: studio.number("cloud_size", 1), done: studio.commit)
                    ColorPicker("Colour", selection: studio.color("cloud_color"), supportsOpacity: false)
                }
            } header: {
                Text("Clouds")
            } footer: {
                Text("Painted over the picture; −y is up in most views.")
            }
        }
    }
}

// MARK: sections

/// A `[formula]` section: which formula (changing it takes the new one's
/// default values), its iterations and its own values.
struct FormulaSection: View {
    let studio: Studio
    let section: SceneText.Section

    var body: some View {
        Section {
            Picker("Formula", selection: Binding(
                get: { section["name"] ?? "" },
                set: { studio.changeFormula(section.id, to: $0) }
            )) {
                ForEach(studio.formulaNames(including: section["name"]), id: \.self) { Text($0).tag($0) }
            }
            .pickerStyle(.navigationLink)
            ForEach(section.entries.filter { $0.key != "name" }, id: \.key) { e in
                NumberField(title: e.key == "iterations" ? "Iterations" : LocalizedStringKey(stringLiteral: e.key),
                            value: studio.sectionNumber(section.id, e.key),
                            done: studio.commit)
            }
            if studio.doc.formulas.count > 1 {
                Button("Remove this formula", systemImage: "minus.circle", role: .destructive) {
                    studio.removeSection(section.id)
                }
            }
        } header: {
            Text("Formula \(studio.doc.formulas.firstIndex { $0.id == section.id }.map { $0 + 1 } ?? 1)")
        }
    }
}

/// A `[light]` section: colour, strength, direction, highlight (a light
/// switched off is no light: Remove takes it away).
struct LightSection: View {
    let studio: Studio
    let section: SceneText.Section
    let number: Int
    let removable: Bool

    var body: some View {
        Section("Light \(number)") {
            ColorPicker("Colour", selection: studio.sectionColor(section.id, "color"), supportsOpacity: false)
            LabeledSlider(title: "Strength", value: studio.sectionNumber(section.id, "amplitude"), range: 0...4,
                          format: "%.2f", done: studio.commit)
            if section["position"] == nil {
                LabeledSlider(title: "From the side", value: studio.sectionNumber(section.id, "x_angle"),
                              range: -180...180, format: "%.0f°", done: studio.commit)
                LabeledSlider(title: "From above", value: studio.sectionNumber(section.id, "y_angle"),
                              range: -180...180, format: "%.0f°", done: studio.commit)
            }
            Picker("Highlight", selection: studio.sectionChoice(section.id, "spec_power", "8")) {
                ForEach(["2", "4", "8", "16", "32", "64", "128", "256"], id: \.self) { Text("Size \($0)").tag($0) }
            }
            Picker("Shading", selection: studio.sectionChoice(section.id, "diffuse_func", "0")) {
                Text("Cos").tag("0")
                Text("Cos²").tag("1")
                Text("Cos/2+").tag("2")
                Text("(Cos/2+)²").tag("3")
            }
            Toggle("Relative to the object", isOn: studio.sectionFlag(section.id, "relative_to_object", false))
            if removable {
                Button("Remove this light", systemImage: "minus.circle", role: .destructive) {
                    studio.removeSection(section.id)
                }
            }
        }
    }
}

/// A number typed in (with the number keyboard), applied when entered.
struct NumberField: View {
    let title: LocalizedStringKey
    @Binding var value: Double
    let done: () -> Void

    var body: some View {
        LabeledContent(title) {
            TextField(title, value: $value, format: .number.precision(.significantDigits(1...10)))
                .keyboardType(.numbersAndPunctuation)
                .multilineTextAlignment(.trailing)
                .monospacedDigit()
                .frame(maxWidth: 110)
                .onSubmit(done)
        }
    }
}

/// A slider that renders again when the finger lifts.
struct LabeledSlider: View {
    let title: LocalizedStringKey
    @Binding var value: Double
    let range: ClosedRange<Double>
    var logarithmic = false
    let format: String
    let done: () -> Void

    var body: some View {
        VStack(alignment: .leading) {
            LabeledContent(title, value: String(format: format, value))
            Slider(value: position, in: 0...1) { editing in
                if !editing { done() }
            }
        }
    }

    /// The slider's place 0...1 (on a log scale for wide ranges).
    private var position: Binding<Double> {
        Binding(
            get: {
                let v = min(max(value, range.lowerBound), range.upperBound)
                return logarithmic
                    ? log(v / range.lowerBound) / log(range.upperBound / range.lowerBound)
                    : (v - range.lowerBound) / (range.upperBound - range.lowerBound)
            },
            set: { p in
                value = logarithmic
                    ? range.lowerBound * pow(range.upperBound / range.lowerBound, p)
                    : range.lowerBound + p * (range.upperBound - range.lowerBound)
            }
        )
    }
}

/// The system's share sheet for files made on request.
struct ShareSheet: UIViewControllerRepresentable {
    let items: [Any]

    func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }

    func updateUIViewController(_ vc: UIActivityViewController, context: Context) {}
}

extension URL: @retroactive Identifiable {
    public var id: String { absoluteString }
}
