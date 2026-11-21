import SwiftUI
import UIKit

/// The picture's gestures, with UIKit's own recognizers (they work
/// together as on any iPad app): one finger looks around, two fingers
/// slide, a pinch flies forward and back, a twist rolls, a double tap goes
/// back to the opened view, a tap focuses (with depth of field on).
struct GestureLayer: UIViewRepresentable {
    let studio: Studio

    func makeCoordinator() -> Coordinator {
        Coordinator(studio: studio)
    }

    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .clear
        let c = context.coordinator

        let look = UIPanGestureRecognizer(target: c, action: #selector(Coordinator.look(_:)))
        look.minimumNumberOfTouches = 1
        look.maximumNumberOfTouches = 1
        let slide = UIPanGestureRecognizer(target: c, action: #selector(Coordinator.slide(_:)))
        slide.minimumNumberOfTouches = 2
        slide.maximumNumberOfTouches = 2
        let pinch = UIPinchGestureRecognizer(target: c, action: #selector(Coordinator.pinch(_:)))
        let twist = UIRotationGestureRecognizer(target: c, action: #selector(Coordinator.twist(_:)))
        let reset = UITapGestureRecognizer(target: c, action: #selector(Coordinator.reset(_:)))
        reset.numberOfTapsRequired = 2
        let focus = UITapGestureRecognizer(target: c, action: #selector(Coordinator.focus(_:)))
        focus.require(toFail: reset)

        for g in [look, slide, pinch, twist, reset, focus] as [UIGestureRecognizer] {
            g.delegate = c
            view.addGestureRecognizer(g)
        }
        return view
    }

    func updateUIView(_ view: UIView, context: Context) {}

    @MainActor
    final class Coordinator: NSObject, UIGestureRecognizerDelegate {
        let studio: Studio
        /// recognizers still running (the full picture waits for all)
        private var active = Set<ObjectIdentifier>()

        init(studio: Studio) {
            self.studio = studio
        }

        // two fingers may slide, pinch and twist at the same time
        func gestureRecognizer(_ g: UIGestureRecognizer,
                               shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool {
            !(g is UITapGestureRecognizer || other is UITapGestureRecognizer)
        }

        private func track(_ g: UIGestureRecognizer) -> Bool {
            switch g.state {
            case .began:
                active.insert(ObjectIdentifier(g))
                return true
            case .changed:
                return true
            case .ended, .cancelled, .failed:
                if active.remove(ObjectIdentifier(g)) != nil, active.isEmpty {
                    studio.gestureEnded()
                }
                return false
            default:
                return false
            }
        }

        @objc func look(_ g: UIPanGestureRecognizer) {
            guard track(g), let v = g.view else { return }
            let t = g.translation(in: v)
            g.setTranslation(.zero, in: v)
            studio.look(dx: t.x, dy: t.y, height: v.bounds.height)
        }

        @objc func slide(_ g: UIPanGestureRecognizer) {
            guard track(g), let v = g.view else { return }
            let t = g.translation(in: v)
            g.setTranslation(.zero, in: v)
            studio.slide(dx: t.x, dy: t.y, height: v.bounds.height)
        }

        @objc func pinch(_ g: UIPinchGestureRecognizer) {
            guard track(g) else { return }
            let s = g.scale
            g.scale = 1
            studio.pinch(scale: s)
        }

        @objc func twist(_ g: UIRotationGestureRecognizer) {
            guard track(g) else { return }
            let r = g.rotation
            g.rotation = 0
            studio.twist(radians: r)
        }

        @objc func focus(_ g: UITapGestureRecognizer) {
            if g.state == .ended, let v = g.view {
                studio.tap(at: g.location(in: v), in: v.bounds.size)
            }
        }

        @objc func reset(_ g: UITapGestureRecognizer) {
            if g.state == .ended {
                studio.resetView()
            }
        }
    }
}
