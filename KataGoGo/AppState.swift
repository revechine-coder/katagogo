import Foundation

@Observable
final class AppState {
    static let shared = AppState()
    
    var kataGoBinaryPath = "\(NSHomeDirectory())/Desktop/KataGoGo/kata-engine/katago-eigen"
    var kataGoConfigPath = "\(NSHomeDirectory())/Desktop/KataGoGo/kata-engine/gtp.cfg"
    var kataGoModelPath = "\(NSHomeDirectory())/Desktop/KataGoGo/kata-engine/lionffen_b24c64.bin.gz"
    let boardSize = 19
    var aiLevel = 4
    
    var isConfigured: Bool { true }
}
