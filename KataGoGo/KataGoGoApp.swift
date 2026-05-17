import SwiftUI

extension Notification.Name {
    static let newGame = Notification.Name("newGame")
}

@main
struct KataGoGoApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        .windowResizability(.contentMinSize)
    }
}
