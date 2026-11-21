import Charts
import SwiftUI

/// The animation's curves (the desktop's Curves window): one value of the
/// scene over all frames as the keyframes interpolate it, the keyframes as
/// points on it. A point dragged up or down, or a value typed for the
/// chosen keyframe, changes that keyframe.
struct CurvesView: View {
    @Bindable var director: Director
    @State private var param = 0
    @State private var chosen: Int?
    @State private var curve: (values: [Double], keys: [(frame: Int, value: Double)]) = ([], [])
    /// the value range drawn (kept while a point is dragged)
    @State private var range: ClosedRange<Double> = 0...1
    @State private var dragging = false
    @State private var typed = ""

    static func title(_ key: String) -> LocalizedStringKey {
        switch key {
        case "zoom": "Zoom"
        case "x": "Position X"
        case "y": "Position Y"
        case "z": "Position Z"
        case "rotation_x": "Rotation X"
        case "rotation_y": "Rotation Y"
        case "rotation_z": "Rotation Z"
        case "fov": "Field of view"
        case "z_start": "Ray start"
        case "ground": "Ground position"
        case "cloud_x": "Cloud drift X"
        case "cloud_y": "Cloud drift Y"
        case "cloud_z": "Cloud drift Z"
        default: LocalizedStringKey(key)
        }
    }

    var body: some View {
        Form {
            Section {
                Picker("Value", selection: $param) {
                    ForEach(Array(Film.curveNames.enumerated()), id: \.offset) { i, name in
                        Text(Self.title(name)).tag(i)
                    }
                }
            }
            Section {
                if curve.values.isEmpty || curve.values.allSatisfy({ $0.isNaN }) {
                    Text("The scenes of the keyframes do not have this value.")
                        .foregroundStyle(.secondary)
                } else {
                    chart.frame(height: 280)
                }
            } footer: {
                Text("Drag a keyframe's point up or down. The line is the value of every frame.")
            }
            if let k = chosen, k < curve.keys.count {
                Section("Keyframe \(k + 1)") {
                    HStack {
                        TextField("Value", text: $typed)
                            .keyboardType(.numbersAndPunctuation)
                            .onSubmit { apply(typed) }
                        Button("Set") { apply(typed) }
                    }
                }
            }
        }
        .navigationTitle("Curves")
        .onAppear(perform: reload)
        .onChange(of: param) {
            chosen = nil
            reload()
        }
    }

    private var chart: some View {
        Chart {
            ForEach(Array(curve.values.enumerated()), id: \.offset) { f, v in
                if v.isFinite {
                    LineMark(x: .value("Frame", f), y: .value("Value", v))
                        .interpolationMethod(.linear)
                }
            }
            ForEach(Array(curve.keys.enumerated()), id: \.offset) { i, k in
                if k.value.isFinite {
                    PointMark(x: .value("Frame", k.frame), y: .value("Value", k.value))
                        .symbol(.square)
                        .symbolSize(i == chosen ? 220 : 120)
                        .foregroundStyle(i == chosen ? Color.orange : Color.accentColor)
                }
            }
        }
        .chartYScale(domain: range)
        .chartXAxisLabel("Frame")
        .chartOverlay { proxy in
            GeometryReader { geo in
                Rectangle().fill(.clear).contentShape(Rectangle())
                    .gesture(DragGesture(minimumDistance: 0)
                        .onChanged { g in drag(g, proxy: proxy, geo: geo) }
                        .onEnded { _ in
                            if dragging, let k = chosen, k < curve.keys.count {
                                commit(k, curve.keys[k].value)
                            }
                            dragging = false
                        })
            }
        }
    }

    private func drag(_ g: DragGesture.Value, proxy: ChartProxy, geo: GeometryProxy) {
        guard let plot = proxy.plotFrame else { return }
        let origin = geo[plot].origin
        let p = CGPoint(x: g.location.x - origin.x, y: g.location.y - origin.y)
        if !dragging {
            // the keyframe nearest the finger (within 40 points)
            let start = CGPoint(x: g.startLocation.x - origin.x, y: g.startLocation.y - origin.y)
            var best: (Int, CGFloat)?
            for (i, k) in curve.keys.enumerated() where k.value.isFinite {
                guard let x = proxy.position(forX: k.frame), let y = proxy.position(forY: k.value) else { continue }
                let d = hypot(x - start.x, y - start.y)
                if d < 40, d < (best?.1 ?? .infinity) { best = (i, d) }
            }
            guard let (i, _) = best else { return }
            chosen = i
            typed = Self.short(curve.keys[i].value)
            dragging = true
        }
        guard let k = chosen, let v = proxy.value(atY: p.y, as: Double.self) else { return }
        curve.keys[k].value = v
        typed = Self.short(v)
    }

    private func apply(_ text: String) {
        guard let k = chosen, let v = Double(text.replacingOccurrences(of: ",", with: ".")) else { return }
        commit(k, v)
    }

    /// Keyframe `k`'s value set in the director's keyframe.
    private func commit(_ k: Int, _ v: Double) {
        guard let film = try? director.film(), k < director.keys.count else { return }
        film.setCurveKey(param, key: k, value: v)
        if let s = try? film.keyframe(k) {
            director.keys[k].text = s.text
        }
        reload()
    }

    private func reload() {
        guard let film = try? director.film(), let c = film.curve(param) else {
            curve = ([], [])
            return
        }
        curve = c
        let all = (c.values + c.keys.map(\.value)).filter(\.isFinite)
        var lo = all.min() ?? 0
        var hi = all.max() ?? 1
        if lo == hi {
            lo -= max(abs(lo) * 0.1, 1e-6)
            hi += max(abs(hi) * 0.1, 1e-6)
        }
        let pad = (hi - lo) * 0.08
        range = (lo - pad)...(hi + pad)
        if let k = chosen, k < c.keys.count { typed = Self.short(c.keys[k].value) }
    }

    static func short(_ v: Double) -> String {
        var s = String(format: "%.6g", v)
        if s == "-0" { s = "0" }
        return s
    }
}
