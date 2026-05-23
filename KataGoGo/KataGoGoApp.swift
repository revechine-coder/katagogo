import SwiftUI

extension Notification.Name {
    static let newGame = Notification.Name("newGame")
}

@main
struct KataGoGoApp: App {
    init() {
        AppState.shared.enforceBundledEnginePaths()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .onAppear {
                    AppState.shared.enforceBundledEnginePaths()
                }
        }
    }
}
