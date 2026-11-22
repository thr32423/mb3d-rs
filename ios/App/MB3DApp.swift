import SwiftUI

@main
struct MB3DApp: App {
    @State private var studio: Studio

    init() {
        // (the engine first: the studio lists its examples)
        Engine.start()
        _studio = State(initialValue: Studio())
    }

    var body: some Scene {
        WindowGroup {
            ContentView(studio: studio)
        }
        // the menu bar (iPadOS 26) and the keyboard's shortcuts
        .commands {
            CommandGroup(after: .newItem) {
                Button("Scenes") { studio.showScenes = true }
                    .keyboardShortcut("o", modifiers: .command)
                Button("Save") { studio.save() }
                    .keyboardShortcut("s", modifiers: .command)
                    .disabled(studio.selected == nil)
            }
            CommandMenu("Picture") {
                Button("Render") { studio.render() }
                    .keyboardShortcut("r", modifiers: .command)
                Button("Monte Carlo (soft light)") { studio.renderMonteCarlo() }
                    .keyboardShortcut("r", modifiers: [.command, .shift])
                Button("Stop") { studio.stop() }
                    .keyboardShortcut(".", modifiers: .command)
                Divider()
                Button("Picture only") { studio.pictureOnly.toggle() }
                    .keyboardShortcut("f", modifiers: [.command, .shift])
                Button("Look closer") { studio.inspecting.toggle() }
                    .keyboardShortcut("l", modifiers: .command)
                Divider()
                Button("Variations") { studio.breeding = true }
                    .keyboardShortcut("b", modifiers: .command)
                Button("Animation") { studio.panel = .animation }
                Button("3D model and AR") { studio.panel = .model }
            }
            CommandMenu("Settings") {
                ForEach(Array(SettingsPage.allCases.enumerated()), id: \.element) { i, p in
                    Button {
                        studio.panel = studio.panel == .settings(p) ? nil : .settings(p)
                    } label: {
                        Text(p.title)
                    }
                    .keyboardShortcut(KeyEquivalent(Character(String((i + 1) % 10))), modifiers: .command)
                }
            }
        }
    }
}
