import CoreGraphics
import SwiftUI

/// The inspector: the scene's settings grouped as on the desktop (camera,
/// fractal and its formulas, lighting and the lights, background and fog,
/// effects). A slider renders again when the finger lifts, a colour, a
/// switch or a choice a moment after the last change, a number when it
/// is entered.
struct SettingsView: View {
    @Bindable var studio: Studio

    var body: some View {
        NavigationStack {
            Form {
                camera
                fractal
                ForEach(studio.doc.formulas) { f in
                    FormulaSection(studio: studio, section: f)
                }
                lighting
                ForEach(Array(studio.doc.lights.enumerated()), id: \.element.id) { i, l in
                    LightSection(studio: studio, section: l, number: i + 1)
                }
                fog
                effects
                Section {
                    LabeledContent("Engine", value: Engine.version)
                }
            }
            .navigationTitle("Settings")
            .navigationBarTitleDisplayMode(.inline)
        }
        .disabled(studio.selected == nil)
    }

    private var camera: some View {
        Section("Camera") {
            LabeledSlider(title: "Zoom", value: studio.number("zoom", 1), range: 0.01...20, logarithmic: true,
                          format: "%.3f", done: studio.commit)
            LabeledSlider(title: "Field of view", value: studio.number("fov", 30), range: 5...150, format: "%.0f°",
                          done: studio.commit)
        }
    }

    private var fractal: some View {
        Section("Fractal") {
            LabeledSlider(title: "Iterations", value: studio.number("iterations", 30, whole: true), range: 1...250,
                          format: "%.0f", done: studio.commit)
            LabeledSlider(title: "Detail (DE stop)", value: studio.number("de_stop", 1), range: 0.05...8,
                          logarithmic: true, format: "%.2f", done: studio.commit)
            LabeledSlider(title: "Ray step", value: studio.number("raystep", 0.5), range: 0.05...1, format: "%.2f",
                          done: studio.commit)
        }
    }

    private var lighting: some View {
        Section("Lighting") {
            LabeledSlider(title: "Diffuse", value: studio.number("diffuse", 50, whole: true), range: 0...255,
                          format: "%.0f", done: studio.commit)
            LabeledSlider(title: "Specular", value: studio.number("specular", 50, whole: true), range: 0...255,
                          format: "%.0f", done: studio.commit)
            LabeledSlider(title: "Ambient", value: studio.number("ambient", 30, whole: true), range: 0...255,
                          format: "%.0f", done: studio.commit)
            LabeledSlider(title: "Gamma", value: studio.number("gamma", 32, whole: true), range: 0...100,
                          format: "%.0f", done: studio.commit)
            ColorPicker("Ambient from above", selection: studio.color("ambient_top"), supportsOpacity: false)
            ColorPicker("Ambient from below", selection: studio.color("ambient_bottom"), supportsOpacity: false)
        }
    }

    private var fog: some View {
        Section("Background and fog") {
            ColorPicker("Background near", selection: studio.color("depth_color"), supportsOpacity: false)
            ColorPicker("Background far", selection: studio.color("depth_color2"), supportsOpacity: false)
            LabeledSlider(title: "Depth fog", value: studio.number("depth_fog", 0, whole: true), range: 0...255,
                          format: "%.0f", done: studio.commit)
            Toggle("Far fog", isOn: studio.flag("far_fog"))
            LabeledSlider(title: "Glowing fog (53 = off)", value: studio.number("dyn_fog", 53, whole: true),
                          range: 0...255, format: "%.0f", done: studio.commit)
            ColorPicker("Glowing fog colour", selection: studio.color("dynfog_color"), supportsOpacity: false)
        }
    }

    private var effects: some View {
        Section("Effects") {
            Picker("Shadows", selection: studio.choice("shadows", "off")) {
                Text("Off").tag("off")
                Text("All lights").tag("all")
            }
            Toggle("Soft shadows", isOn: studio.flag("shadow_soft"))
            Picker("Ambient occlusion", selection: studio.choice("ao", "off")) {
                Text("Off").tag("off")
                Text("Screen space (15)").tag("ssao15")
                Text("Screen space (24)").tag("ssao24")
                Text("Distance estimate").tag("deao")
            }
            Toggle("Reflections", isOn: studio.flag("mc_reflections"))
        }
    }
}

/// A `[formula]` section: which formula (changing it takes the new one's
/// default values), its iterations and its own values.
struct FormulaSection: View {
    let studio: Studio
    let section: SceneText.Section

    var body: some View {
        Section("Formula \(studio.doc.formulas.firstIndex { $0.id == section.id }.map { $0 + 1 } ?? 1)") {
            Picker("Formula", selection: Binding(
                get: { section["name"] ?? "" },
                set: { studio.changeFormula(section.id, to: $0) }
            )) {
                ForEach(studio.formulaNames(including: section["name"]), id: \.self) { Text($0).tag($0) }
            }
            .pickerStyle(.navigationLink)
            ForEach(section.entries.filter { $0.key != "name" }, id: \.key) { e in
                NumberField(title: e.key == "iterations" ? "Iterations" : e.key,
                            value: studio.sectionNumber(section.id, e.key),
                            done: studio.commit)
            }
        }
    }
}

/// A `[light]` section: colour, strength, direction.
struct LightSection: View {
    let studio: Studio
    let section: SceneText.Section
    let number: Int

    var body: some View {
        Section("Light \(number)") {
            ColorPicker("Colour", selection: studio.sectionColor(section.id, "color"), supportsOpacity: false)
            LabeledSlider(title: "Strength", value: studio.sectionNumber(section.id, "amplitude"), range: 0...4,
                          format: "%.2f", done: studio.commit)
            LabeledSlider(title: "From the side", value: studio.sectionNumber(section.id, "x_angle"),
                          range: -180...180, format: "%.0f°", done: studio.commit)
            LabeledSlider(title: "From above", value: studio.sectionNumber(section.id, "y_angle"),
                          range: -180...180, format: "%.0f°", done: studio.commit)
        }
    }
}

/// A number typed in (with the number keyboard), applied when entered.
struct NumberField: View {
    let title: String
    @Binding var value: Double
    let done: () -> Void

    var body: some View {
        LabeledContent(title) {
            TextField(title, value: $value, format: .number.precision(.significantDigits(1...10)))
                .keyboardType(.numbersAndPunctuation)
                .multilineTextAlignment(.trailing)
                .monospacedDigit()
                .onSubmit(done)
        }
    }
}

/// A slider that renders again when the finger lifts.
struct LabeledSlider: View {
    let title: String
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
