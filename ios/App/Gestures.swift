import SwiftUI
import UIKit

/// The picture's gestures, with UIKit's own recognizers (they work
/// together as on any iPad app): one finger looks around, two fingers
/// slide, a pinch flies forward and back, a twist rolls, a double tap
/// flies to what is there, a tap focuses (with depth of field on). A tap
/// with two fingers steps back, with three forward again; a finger held
/// still shows the picture as it was when the scene was opened. A mouse
/// wheel or two fingers on a trackpad fly; the Pencil's double tap or
/// squeeze leaves the picture alone on the screen, and where it hovers a
/// ring shows what a tap would focus on.
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
        let back = UITapGestureRecognizer(target: c, action: #selector(Coordinator.back(_:)))
        back.numberOfTouchesRequired = 2
        let forward = UITapGestureRecognizer(target: c, action: #selector(Coordinator.forward(_:)))
        forward.numberOfTouchesRequired = 3
        let hold = UILongPressGestureRecognizer(target: c, action: #selector(Coordinator.hold(_:)))
        hold.minimumPressDuration = 0.45
        hold.allowableMovement = 8
        // a mouse wheel, two fingers on a trackpad (no touches on the screen)
        let wheel = UIPanGestureRecognizer(target: c, action: #selector(Coordinator.wheel(_:)))
        wheel.allowedScrollTypesMask = .all
        wheel.maximumNumberOfTouches = 0
        let hover = UIHoverGestureRecognizer(target: c, action: #selector(Coordinator.hover(_:)))

        for g in [look, slide, pinch, twist, reset, focus, back, forward, hold, wheel, hover] as [UIGestureRecognizer] {
            g.delegate = c
            view.addGestureRecognizer(g)
        }
        let pencil = UIPencilInteraction()
        pencil.delegate = c
        view.addInteraction(pencil)
        return view
    }

    func updateUIView(_ view: UIView, context: Context) {}

    @MainActor
    final class Coordinator: NSObject, UIGestureRecognizerDelegate, UIPencilInteractionDelegate {
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
            if g.state == .ended, let v = g.view {
                studio.flyTo(g.location(in: v), in: v.bounds.size)
            }
        }

        @objc func back(_ g: UITapGestureRecognizer) {
            if g.state == .ended, studio.canUndo { studio.undo() }
        }

        @objc func forward(_ g: UITapGestureRecognizer) {
            if g.state == .ended, studio.canRedo { studio.redo() }
        }

        @objc func hold(_ g: UILongPressGestureRecognizer) {
            switch g.state {
            case .began: studio.comparing = true
            case .ended, .cancelled, .failed: studio.comparing = false
            default: break
            }
        }

        @objc func wheel(_ g: UIPanGestureRecognizer) {
            guard track(g), let v = g.view else { return }
            let t = g.translation(in: v)
            g.setTranslation(.zero, in: v)
            studio.pinch(scale: exp(-t.y / 300))
        }

        @objc func hover(_ g: UIHoverGestureRecognizer) {
            switch g.state {
            case .began, .changed: studio.hover = g.view.map { g.location(in: $0) }
            default: studio.hover = nil
            }
        }

        func pencilInteractionDidTap(_ interaction: UIPencilInteraction) {
            studio.pictureOnly.toggle()
        }

        @available(iOS 17.5, *)
        func pencilInteraction(_ interaction: UIPencilInteraction, didReceiveSqueeze squeeze: UIPencilInteraction.Squeeze) {
            if squeeze.phase == .ended { studio.pictureOnly.toggle() }
        }
    }
}
