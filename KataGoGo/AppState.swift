import Foundation
import Combine

final class AppState: ObservableObject {
    static let shared = AppState()

    private enum DefaultsKey {
        static let kataGoBinaryPath = "kataGoBinaryPath"
        static let kataGoConfigPath = "kataGoConfigPath"
        static let kataGoModelPath = "kataGoModelPath"
        static let handicapCount = "handicapCount"
    }

    private let defaults = UserDefaults.standard
    @Published private var customKataGoBinaryPath = ""
    @Published private var customKataGoConfigPath = ""
    @Published private var customKataGoModelPath = ""

    private var packagedAppRequiresBundledEngine: Bool {
        Bundle.main.bundleURL.pathExtension == "app"
    }

    private var developmentEngineRoot: String {
        "\(NSHomeDirectory())/Documents/katagogo/kata-engine"
    }

    private var defaultEngineRoot: String {
        if let bundled = bundledEngineRootIfAvailable {
            return bundled
        }
        if packagedAppRequiresBundledEngine {
            return EngineResourceLocator.expectedBundledEngineRoot()
        }
        return developmentEngineRoot
    }

    private var bundledEngineRootIfAvailable: String? {
        EngineResourceLocator.engineRootCandidates().first { root in
            EngineResourceLocator.bundledEnginePaths(in: root) != nil
        }
    }

    private var defaultModelFileName: String {
        EngineResourceLocator.defaultBundledModelFileName
    }

    var kataGoBinaryPath: String {
        get { activeEnginePaths.binary }
        set {
            if bundledEnginePaths == nil && !packagedAppRequiresBundledEngine {
                customKataGoBinaryPath = newValue
            }
            defaults.set(kataGoBinaryPath, forKey: DefaultsKey.kataGoBinaryPath)
        }
    }

    var kataGoConfigPath: String {
        get { activeEnginePaths.config }
        set {
            if bundledEnginePaths == nil && !packagedAppRequiresBundledEngine {
                customKataGoConfigPath = newValue
            }
            defaults.set(kataGoConfigPath, forKey: DefaultsKey.kataGoConfigPath)
        }
    }

    var kataGoModelPath: String {
        get { activeEnginePaths.model }
        set {
            if bundledEnginePaths == nil && !packagedAppRequiresBundledEngine {
                customKataGoModelPath = newValue
            }
            defaults.set(kataGoModelPath, forKey: DefaultsKey.kataGoModelPath)
        }
    }

    let boardSize = 19
    @Published var handicapCount: Int = 0 {
        didSet { defaults.set(handicapCount, forKey: DefaultsKey.handicapCount) }
    }

    var modelDisplayName: String {
        URL(fileURLWithPath: kataGoModelPath).lastPathComponent
    }

    var engineDisplayName: String {
        URL(fileURLWithPath: kataGoBinaryPath).lastPathComponent
    }

    var isUsingBundledEngine: Bool {
        packagedAppRequiresBundledEngine || bundledEnginePaths != nil
    }

    var isConfigured: Bool {
        configurationError == nil
    }

    var configurationError: String? {
        guard fileExists(at: kataGoBinaryPath) else {
            return "KataGo 可执行文件不存在：\(kataGoBinaryPath)"
        }
        guard fileExists(at: kataGoConfigPath) else {
            return "KataGo 配置文件不存在：\(kataGoConfigPath)"
        }
        guard fileExists(at: kataGoModelPath) else {
            return "KataGo 模型文件不存在：\(kataGoModelPath)"
        }
        guard FileManager.default.isExecutableFile(atPath: kataGoBinaryPath) else {
            return "KataGo 文件不可执行：\(kataGoBinaryPath)"
        }
        return nil
    }

    init() {
        let root = defaultEngineRoot
        let defaultBinaryPath = "\(root)/\(EngineResourceLocator.binaryFileName)"
        let defaultConfigPath = "\(root)/\(EngineResourceLocator.configFileName)"
        let defaultModelPath = "\(root)/\(defaultModelFileName)"

        customKataGoBinaryPath = defaults.string(forKey: DefaultsKey.kataGoBinaryPath) ?? defaultBinaryPath
        customKataGoConfigPath = defaults.string(forKey: DefaultsKey.kataGoConfigPath) ?? defaultConfigPath
        customKataGoModelPath = defaults.string(forKey: DefaultsKey.kataGoModelPath) ?? defaultModelPath

        if defaults.object(forKey: DefaultsKey.handicapCount) == nil {
            handicapCount = 0
        } else {
            let savedHandicapCount = defaults.integer(forKey: DefaultsKey.handicapCount)
            handicapCount = savedHandicapCount == 0 ? 0 : min(max(savedHandicapCount, 2), 9)
        }

        persistEnginePaths()
    }

    func resetEnginePathsToDefaults() {
        let root = defaultEngineRoot
        kataGoBinaryPath = "\(root)/\(EngineResourceLocator.binaryFileName)"
        kataGoConfigPath = "\(root)/\(EngineResourceLocator.configFileName)"
        kataGoModelPath = "\(root)/\(defaultModelFileName)"
    }

    func enforceBundledEnginePaths() {
        guard let paths = bundledEnginePaths else {
            if packagedAppRequiresBundledEngine {
                persistEnginePaths()
            }
            return
        }
        customKataGoBinaryPath = paths.binary
        customKataGoConfigPath = paths.config
        customKataGoModelPath = paths.model
        persistEnginePaths()
    }

    private func fileExists(at path: String) -> Bool {
        var isDirectory: ObjCBool = false
        return FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory) && !isDirectory.boolValue
    }

    private var bundledEnginePaths: (binary: String, config: String, model: String)? {
        guard let root = bundledEngineRootIfAvailable else { return nil }
        return EngineResourceLocator.bundledEnginePaths(in: root)
    }

    private var activeEnginePaths: (binary: String, config: String, model: String) {
        if let paths = bundledEnginePaths {
            return paths
        }

        if packagedAppRequiresBundledEngine {
            let root = EngineResourceLocator.expectedBundledEngineRoot()
            return (
                binary: "\(root)/\(EngineResourceLocator.binaryFileName)",
                config: "\(root)/\(EngineResourceLocator.configFileName)",
                model: "\(root)/\(EngineResourceLocator.defaultBundledModelFileName)"
            )
        }

        return (
            binary: customKataGoBinaryPath,
            config: customKataGoConfigPath,
            model: customKataGoModelPath
        )
    }

    private func persistEnginePaths() {
        defaults.set(kataGoBinaryPath, forKey: DefaultsKey.kataGoBinaryPath)
        defaults.set(kataGoConfigPath, forKey: DefaultsKey.kataGoConfigPath)
        defaults.set(kataGoModelPath, forKey: DefaultsKey.kataGoModelPath)
    }
}
