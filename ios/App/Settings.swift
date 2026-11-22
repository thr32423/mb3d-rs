import CoreGraphics
import PhotosUI
import SwiftUI

/// The groups of settings, as the desktop groups them (main window tabs,
/// Formulas, Lighting, Post-process): the icons of the rail beside the
/// picture.
enum SettingsPage: String, Hashable, CaseIterable, Identifiable {
    case picture, fractal, formulas, julia, lighting, colours, fog, effects, finishing, ground

    var id: String { rawValue }

    var title: LocalizedStringKey {
        switch self {
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

    var icon: String {
        switch self {
        case .picture: "photo"
        case .fractal: "cube"
        case .formulas: "function"
        case .julia: "scissors"
        case .lighting: "lightbulb"
        case .colours: "paintpalette"
        case .fog: "cloud.fog"
        case .effects: "sparkles"
        case .finishing: "camera.filters"
        case .ground: "water.waves"
        }
    }
}

/// One group's settings in the panel that floats beside the picture. While
/// a slider or a number is held, quick small pictures follow it and the
/// full one comes when the finger lifts; a colour, a switch or a choice
/// shows quickly at once and in full a moment after the last change.
struct SettingsPanel: View {
    let studio: Studio
    let page: SettingsPage
    let close: () -> Void

    var body: some View {
        NavigationStack {
            content
                .navigationTitle(page.title)
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .topBarTrailing) {
                        Button("Close", systemImage: "xmark", action: close)
                    }
                }
        }
        .id(page)
        .environment(studio)
        .disabled(studio.selected == nil)
    }

    @ViewBuilder
    private var content: some View {
        switch page {
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

    static let page = argument("-page").flatMap(SettingsPage.init(rawValue:))
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
                Button("Render a stereo pair", systemImage: "eyeglasses") { studio.renderStereoPair() }
                Toggle("Cross-eyed (else parallel)", isOn: $studio.crossEyed)
                PresentFields(studio: studio, prefixes: ["stereo_screen"])
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
            More {
                Fields(studio: studio, fields: [
                    .number("min_iterations", "Minimum iterations", 0, whole: true),
                    .number("rstop", "Bailout", 16),
                    .number("raystep_limiter", "Ray step limiter", 1),
                    .number("z_start", "Ray start", 0),
                    .number("z_end", "Ray end", 0),
                    .choice("inside", "Rendering", [("outside", "Outside"), ("inside", "Inside"), ("both", "Inside and outside")], "outside"),
                    .choice("slice_2d", "2D slice", [("off", "Off"), ("start", "At the ray start"), ("mid", "In the middle"), ("end", "At the ray end")], "off"),
                    .toggle("normals_on_zbuf", "Normals on the z-buffer"),
                    .vector("rotation_4d", ["4D rotation XW", "4D rotation YW", "4D rotation ZW"], [0, 0, 0]),
                ])
            }
            More(title: "Experimental speed-ups") {
                Fields(studio: studio, fields: [
                    .toggle("experimental", "Experimental speed-ups"),
                    .slider("step_relax", "Longer march steps", 1...2, 1, "%.2f"),
                    .toggle("secant_search", "Secant search"),
                    .toggle("free_steps", "No step limit"),
                    .toggle("occlusion_half", "Occlusion at half size"),
                    .toggle("cone_march", "Cone march"),
                ])
            }
        }
    }
}

struct FormulasPage: View {
    let studio: Studio
    @State private var importing = false
    /// the card whose values show (a section of the scene's text)
    @State private var chosen: Int?

    var body: some View {
        Form {
            Section {
                FormulaCards(studio: studio, chosen: $chosen)
                    .listRowInsets(EdgeInsets(top: 8, leading: 12, bottom: 8, trailing: 12))
            }
            if let f = studio.doc.formulas.first(where: { $0.id == chosen }) ?? studio.doc.formulas.first {
                FormulaSection(studio: studio, section: f)
            }
            Section {
                LabeledSlider(title: "Repeat from formula", value: studio.repeatFrom(),
                              range: 1...Double(max(studio.doc.formulas.count, 2)), format: "%.0f",
                              done: studio.commit)
                Toggle("Disable analytic DE", isOn: studio.flag("disable_analytic_de"))
            } footer: {
                Text("Up to six formulas run one after the other (a hybrid); after the last, the iteration goes on from the one chosen.")
            }
            More(title: "Hybrid: interpolation, two objects") {
                Toggle("Interpolate the first two formulas", isOn: studio.effect("interpolation", on: "0.5, 0.5"))
                if studio.effect("interpolation", on: "0.5, 0.5").wrappedValue {
                    Fields(studio: studio, fields: [.vector("interpolation", ["Weight of formula 1", "Weight of formula 2"], [0.5, 0.5])])
                }
                Fields(studio: studio, fields: [
                    .choice("de_combination", "Combine two parts", [
                        ("off", "Off"), ("min", "Union"), ("max", "Intersection"), ("max_inverted", "Cut out"),
                        ("smooth_linear", "Smooth, linear"), ("smooth", "Smooth"), ("mix", "Mix"),
                    ], "off"),
                ])
                PresentFields(studio: studio, prefixes: ["decomb_", "part2_"])
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
                ForEach(0..<4) { i in
                    NumberField(title: LocalizedStringKey(stringLiteral: ["x", "y", "z", "w"][i]), value: studio.component("julia_c", i, [0, 0, 0, 0]),
                                done: studio.commit)
                }
            } header: {
                Toggle("Julia", isOn: studio.flag("julia"))
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
                NumberField(title: "Roughness", value: studio.number("roughness", 255, whole: true), whole: true,
                            done: studio.commit)
                ColorPicker("Ambient from above", selection: studio.color("ambient_top"), supportsOpacity: false)
                ColorPicker("Ambient from below", selection: studio.color("ambient_bottom"), supportsOpacity: false)
            }
            More {
                Fields(studio: studio, fields: [
                    .number("indirect_light", "Indirect light", 0),
                    .number("diffuse_shadowing", "Diffuse shadowing", 0),
                    .toggle("ambient_relative_to_object", "Ambient relative to the object"),
                    .toggle("internal_gamma2", "Gamma 2 inside"),
                    .number("light_mode", "Second light mode", 0, whole: true),
                    .choice("vol_light", "Volumetric light of", [("off", "Off"), ("1", "Light 1"), ("2", "Light 2"), ("3", "Light 3"),
                                                                 ("4", "Light 4"), ("5", "Light 5"), ("6", "Light 6")], "off"),
                    .slider("vol_light_map_size", "Volumetric light: map size", -7...7, 0, "%.0f", whole: true),
                ])
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
                PaletteStrip(studio: studio)
            }
            Section {
                NumberField(title: "Colour start", value: studio.number("color_start", 0), done: studio.commit)
                NumberField(title: "Colour end", value: studio.number("color_end", 0), done: studio.commit)
                NumberField(title: "Colour multiplier", value: studio.number("color_mul", 1), done: studio.commit)
                Toggle("Colour cycling", isOn: studio.flag("color_cycling"))
            } footer: {
                Text("Each palette colour has a diffuse (left) and a specular (right) part; start and end place the palette over the surface.")
            }
            if studio.doc["interior_colors"] != nil {
                Section("Inside colours") {
                    ForEach(0..<4) { i in
                        ColorPicker("Colour \(i + 1)", selection: studio.interiorColor(i), supportsOpacity: false)
                    }
                    Fields(studio: studio, fields: [
                        .number("interior_start", "Colour start", 0),
                        .number("interior_end", "Colour end", 0),
                    ])
                }
            }
            More {
                Fields(studio: studio, fields: [
                    .toggle("color_on_otrap", "Colour on orbit trap"),
                    .number("color_on_iteration", "Colour on iteration (-1: off)", -1, whole: true),
                    .choice("color_option", "Second colour choice", [("0", "0"), ("1", "1"), ("2", "2"), ("3", "3"), ("4", "4"), ("5", "5")], "0"),
                    .number("color_var_z", "Colour variation on z", 0),
                    .toggle("color_interpolation", "Blend the palette colours", true),
                    .number("diffuse_map", "Diffuse map number", 0, whole: true),
                    .choice("diffuse_map_mode", "Diffuse map on", [("iterations_otrap", "Iterations, orbit trap"), ("normals", "Normals"),
                                                                    ("wrap_sine", "Wrap, sine"), ("wrap", "Wrap")], "iterations_otrap"),
                ])
                Toggle("Fine colour adjustment", isOn: studio.effect("fine_color_adjust", on: "0, 0"))
                PresentFields(studio: studio, prefixes: ["fine_color_adjust", "diffuse_map_offset", "diffuse_map_rotation",
                                                         "diffuse_map_scale", "diffuse_map_brightness_only"])
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
            More {
                Fields(studio: studio, fields: [
                    .number("fog_offset", "Fog offset", 0),
                    .number("fog_far_offset", "Far fog offset", 0),
                    .choice("depth_func", "Background colour function", [("0", "0"), ("1", "1"), ("2", "2"), ("3", "3")], "0"),
                    .bit("dynfog_options", "Glowing fog: blend", 1),
                    .bit("dynfog_options", "Glowing fog: positive only", 2),
                    .number("dfog_on_it", "Glowing fog on iteration", 0, whole: true),
                ])
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
                if studio.effect("dof", on: "sorted").wrappedValue {
                    NumberField(title: "Focus", value: studio.number("dof_focus", 1), done: studio.commit)
                    LabeledSlider(title: "Aperture", value: studio.number("dof_aperture", 0.1), range: 0.001...2,
                                  logarithmic: true, format: "%.3f", done: studio.commit)
                }
            } header: {
                Toggle("Depth of field", isOn: studio.effect("dof", on: "sorted"))
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
            More(title: "Shadows, occlusion, depth of field") {
                Fields(studio: studio, fields: [
                    .slider("shadow_soft_radius", "Soft shadow radius", 0.01...20, 1, "%.2f", log: true),
                    .number("shadow_max_len", "Shadow length", 1),
                    .toggle("shadow_set_cos", "Shadows change the shading"),
                    .number("ao_threshold", "Occlusion threshold", 1),
                    .slider("ao_border", "Occlusion at the borders", 0...0.9, 0, "%.2f"),
                    .number("ao_random", "Occlusion: random", 0, whole: true),
                    .choice("deao_quality", "DE occlusion rays", [("0", "3"), ("1", "7"), ("2", "17"), ("3", "33")], "1"),
                    .choice("deao_dither", "DE occlusion dithering", [("0", "Off"), ("1", "2 × 2"), ("2", "3 × 3")], "0"),
                    .number("deao_max_len", "DE occlusion reach", 1),
                    .number("dof_focus2", "Second focus", 1),
                    .number("dof_max_radius", "Largest blur", 1),
                    .number("dof_passes", "Depth of field passes", 1, whole: true),
                ])
                PresentFields(studio: studio, prefixes: ["deao_random"])
            }
            More(title: "Monte Carlo") {
                Fields(studio: studio, fields: [
                    .number("mc_depth", "Ray depth", 3, whole: true),
                    .number("mc_reflection_depth", "Reflection depth", 2, whole: true),
                    .number("mc_diffuse_reflects", "Diffuse reflections", 0),
                    .number("mc_soft_shadow_radius", "Soft shadow radius", 1),
                    .slider("mc_exposure", "Exposure", 0...255, 100, "%.0f", whole: true),
                    .slider("mc_saturation", "Saturation", 0...127, 64, "%.0f", whole: true),
                    .number("mc_refraction_index", "Refraction index", 1.5),
                    .number("mc_absorption", "Absorption", 1),
                    .number("mc_scattering", "Scattering", 0),
                    .toggle("mc_only_difs", "Only dIFS"),
                    .bit("mc_options", "Soft clip", 1),
                    .bit("mc_options", "Secant search", 2),
                    .bit("mc_options", "Auto clip", 4),
                    .bit("mc_options", "Gaussian antialiasing", 8),
                ])
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
                Toggle("Droste", isOn: studio.effect("droste", on: "0.5, 0.5, 0.12, 0.45"))
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
                Toggle("Ground and water", isOn: studio.effect("ground", on: "y, 1.2"))
            } footer: {
                Text("Down the screen is +y in most views: try both signs. Water shows with reflections on.")
            }
            Section {
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
                Toggle("Clouds", isOn: studio.effect("clouds", on: "y, -1.2, -2.2"))
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

    /// the formula's own values (a double tap on a number goes back to it)
    private var defaults: [String: Double] {
        var d: [String: Double] = [:]
        for s in Engine.formulaSettings(section["name"] ?? "") { d[s.key] = Double(s.value) }
        return d
    }

    var body: some View {
        let defaults = self.defaults
        Section {
            NavigationLink {
                FormulaPicker(names: studio.formulaNames(including: section["name"]), current: section["name"] ?? "",
                              own: studio.ownFormulas, preview: { studio.text(with: $0, in: section.id) }) {
                    studio.changeFormula(section.id, to: $0)
                }
            } label: {
                LabeledContent("Formula", value: section["name"] ?? "")
            }
            ForEach(section.entries.filter { $0.key != "name" }, id: \.key) { e in
                NumberField(title: e.key == "iterations" ? "Iterations" : LocalizedStringKey(stringLiteral: e.key),
                            value: studio.sectionNumber(section.id, e.key),
                            whole: e.key == "iterations",
                            reset: e.key == "iterations" ? nil : defaults[e.key],
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
                LightBall(side: studio.sectionNumber(section.id, "x_angle"),
                          above: studio.sectionNumber(section.id, "y_angle"),
                          colour: studio.sectionColor(section.id, "color").wrappedValue, done: studio.commit)
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
            DisclosureGroup("More") {
                Toggle("At a place in the scene", isOn: studio.positional(section.id))
                if section["position"] != nil {
                    ForEach(0..<3) { i in
                        NumberField(title: (["x", "y", "z"] as [LocalizedStringKey])[i],
                                    value: studio.sectionComponent(section.id, "position", i), done: studio.commit)
                    }
                }
                Picker("Visible light source", selection: studio.sectionChoice(section.id, "visible", "0")) {
                    Text("Off").tag("0")
                    ForEach(1..<8) { n in Text(verbatim: "\(n)").tag("\(n)") }
                }
                Toggle("Casts hard shadows", isOn: studio.sectionFlag(section.id, "shadow", true))
                NumberField(title: "Light map number", value: studio.sectionNumber(section.id, "map"), whole: true,
                            done: studio.commit)
            }
            if removable {
                Button("Remove this light", systemImage: "minus.circle", role: .destructive) {
                    studio.removeSection(section.id)
                }
            }
        }
    }
}

/// A number changed by dragging it: left and right over the value, finer
/// the further the finger moves away up or down (a tenth past 50 points, a
/// hundredth past 120). A tap types it in, a double tap goes back to
/// `reset` (a formula value's default).
struct NumberField: View {
    let title: LocalizedStringKey
    @Binding var value: Double
    /// counts in whole steps (iterations)
    var whole = false
    var reset: Double?
    let done: () -> Void

    @Environment(Studio.self) private var studio: Studio?
    @GestureState private var dragging = false
    /// the value under the finger before rounding, and the finger's last x
    @State private var held: Double?
    @State private var lastX: CGFloat = 0
    @State private var unit = 0.0
    @State private var typing = false
    @State private var typed = 0.0
    @FocusState private var focused: Bool

    var body: some View {
        LabeledContent(title) {
            if typing {
                TextField(title, value: $typed, format: .number.precision(.significantDigits(1...10)))
                    .keyboardType(.numbersAndPunctuation)
                    .multilineTextAlignment(.trailing)
                    .monospacedDigit()
                    .frame(maxWidth: 110)
                    .focused($focused)
                    .onAppear { focused = true }
                    .onSubmit {
                        value = typed
                        typing = false
                        done()
                    }
                    .onChange(of: focused) { if !focused { typing = false } }
            } else {
                Text(shown)
                    .monospacedDigit()
                    .padding(.horizontal, 10)
                    .padding(.vertical, 5)
                    .frame(minWidth: 72, alignment: .trailing)
                    .background(Color.secondary.opacity(0.16), in: Capsule())
                    .contentShape(Capsule())
                    .hoverEffect()
                    .gesture(drag)
                    .onTapGesture(count: 2) {
                        if let reset, reset != value {
                            value = reset
                            done()
                        }
                    }
                    .onTapGesture {
                        typed = value
                        typing = true
                    }
                    .onChange(of: dragging) {
                        // the finger lifted (or the drag was taken away)
                        if !dragging, held != nil {
                            held = nil
                            studio?.adjusting = nil
                            done()
                        }
                    }
                    .accessibilityValue(shown)
                    .accessibilityAdjustableAction { direction in
                        let step = whole ? 1 : max(abs(value), 0.25) * 0.05
                        switch direction {
                        case .increment: value = tidy(value + step)
                        case .decrement: value = tidy(value - step)
                        @unknown default: break
                        }
                        done()
                    }
            }
        }
    }

    private var shown: String {
        whole ? String(format: "%.0f", value) : value.formatted(.number.precision(.significantDigits(1...6)))
    }

    private var drag: some Gesture {
        DragGesture(minimumDistance: 4)
            .updating($dragging) { _, state, _ in state = true }
            .onChanged { g in
                if held == nil {
                    held = value
                    lastX = 0
                    // a full drag across a panel about doubles a value
                    unit = whole ? 0.125 : max(abs(value), 0.25) * 0.004
                }
                let away = abs(g.translation.height)
                let fine = whole ? 1 : away < 50 ? 1 : away < 120 ? 0.1 : 0.01
                let now = (held ?? value) + Double(g.translation.width - lastX) * unit * fine
                lastX = g.translation.width
                held = now
                // (in steps of the size being moved by, so the digits stay few)
                let q = whole ? 1 : pow(10, (log10(unit * fine)).rounded(.down))
                let v = tidy((now / q).rounded() * q)
                if v != value {
                    value = v
                    studio?.previewSettings()
                }
                studio?.adjusting = Studio.Adjusting(title: title, value: shown, fraction: nil)
            }
    }

    /// Without the binary tail (2.0010000000000003).
    private func tidy(_ v: Double) -> Double {
        whole ? v.rounded() : Double(String(format: "%.10g", v)) ?? v
    }
}

/// A slider: quick pictures follow it while it is held (its name and value
/// alone over the picture), the full one comes when the finger lifts.
struct LabeledSlider: View {
    let title: LocalizedStringKey
    @Binding var value: Double
    let range: ClosedRange<Double>
    var logarithmic = false
    let format: String
    let done: () -> Void

    @Environment(Studio.self) private var studio: Studio?

    var body: some View {
        VStack(alignment: .leading) {
            LabeledContent(title, value: String(format: format, value))
            Slider(value: position, in: 0...1) { editing in
                if !editing {
                    studio?.adjusting = nil
                    done()
                }
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
                if let studio {
                    studio.adjusting = Studio.Adjusting(title: title, value: String(format: format, value), fraction: p)
                    studio.previewSettings()
                }
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
