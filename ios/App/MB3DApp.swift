import SwiftUI

@main
struct MB3DApp: App {
    init() {
        Engine.start()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
