import Foundation

enum EngineResourceLocator {
    static let engineDirectoryName = "kata-engine"
    static let binaryFileName = "katago-metal"
    static let configFileName = "gtp.cfg"
    static let tinyModelFileName = "lionffen_b24c64.bin.gz"
    static let mediumModelFileName = "kata1-b18c384nbt.bin.gz"
    static var defaultBundledModelFileName: String { mediumModelFileName }

    /// Model file names checked in priority order (first found wins).
    static let bundledModelFileNames = [tinyModelFileName, mediumModelFileName]

    /// Environment variable key for overriding the engine root at runtime.
    /// Set `KATAGOGO_ENGINE_ROOT` to a directory containing katago-metal, gtp.cfg, and a model.
    static let engineRootEnvKey = "KATAGOGO_ENGINE_ROOT"

    // MARK: - Tier 1: App Bundle Resources (highest priority)

    /// Searches for the engine root inside the app bundle's Resources.
    ///
    /// Lookup order within the bundle:
    /// 1. `Bundle.url(forResource:subdirectory:)` — the canonical Bundle API
    /// 2. `resourceURL` fallback — for bundle structures where #1 misses
    /// 3. Explicit `Contents/Resources` — traditional macOS .app layout
    static func bundledEngineRoot(in bundle: Bundle = .main) -> String? {
        // 1. Canonical Bundle resource lookup
        if let url = bundle.url(
            forResource: binaryFileName,
            withExtension: nil,
            subdirectory: engineDirectoryName
        ) {
            let root = url.deletingLastPathComponent().path
            if bundledEnginePaths(in: root) != nil {
                return root
            }
        }

        // 2. resourceURL-based lookup
        if let resourceURL = bundle.resourceURL {
            let engineDir = resourceURL.appendingPathComponent(engineDirectoryName, isDirectory: true)
            if bundledEnginePaths(in: engineDir.path) != nil {
                return engineDir.path
            }
        }

        // 3. Explicit Contents/Resources (traditional .app layout)
        let contentsEngine = bundle.bundleURL
            .appendingPathComponent("Contents/Resources", isDirectory: true)
            .appendingPathComponent(engineDirectoryName, isDirectory: true)
        if bundledEnginePaths(in: contentsEngine.path) != nil {
            return contentsEngine.path
        }

        return nil
    }

    // MARK: - Tier 2: Environment Variable

    /// Returns the engine root from the `KATAGOGO_ENGINE_ROOT` environment variable,
    /// after validating that the required files exist at that path.
    static func environmentEngineRoot() -> String? {
        guard let raw = ProcessInfo.processInfo.environment[engineRootEnvKey],
              !raw.isEmpty else {
            return nil
        }
        let expanded = (raw as NSString).expandingTildeInPath
        guard bundledEnginePaths(in: expanded) != nil else {
            return nil
        }
        return expanded
    }

    // MARK: - Tier 3: Development Fallback

    /// Returns the default development engine root under the user's Documents directory.
    /// Does NOT validate existence — callers must check via `bundledEnginePaths(in:)`.
    static func developmentEngineRoot() -> String {
        "\(NSHomeDirectory())/Documents/katagogo/kata-engine"
    }

    // MARK: - Resolution

    /// Returns the best available engine root by trying each tier in priority order:
    /// 1. App Bundle Resources
    /// 2. `KATAGOGO_ENGINE_ROOT` environment variable
    /// 3. Development fallback (`~/Documents/katagogo/kata-engine`)
    ///
    /// The returned path is NOT guaranteed to contain valid engine files —
    /// callers must validate via `bundledEnginePaths(in:)` or `isConfigured`.
    static func resolveEngineRoot(in bundle: Bundle = .main) -> String {
        if let bundled = bundledEngineRoot(in: bundle) {
            return bundled
        }
        if let env = environmentEngineRoot() {
            return env
        }
        return developmentEngineRoot()
    }

    // MARK: - Validation

    /// Validates that the given root directory contains the required engine files.
    /// Returns the resolved binary, config, and model paths, or `nil` if any are missing.
    static func bundledEnginePaths(in root: String) -> (binary: String, config: String, model: String)? {
        let binary = "\(root)/\(binaryFileName)"
        let config = "\(root)/\(configFileName)"
        let fm = FileManager.default

        let model = bundledModelFileNames
            .map { "\(root)/\($0)" }
            .first { isRegularFile(at: $0, fileManager: fm) }

        guard isRegularFile(at: config, fileManager: fm),
              let model,
              isRegularFile(at: binary, fileManager: fm),
              fm.isExecutableFile(atPath: binary) else {
            return nil
        }

        return (binary: binary, config: config, model: model)
    }

    /// Returns candidate engine root paths, used for discovery of bundled engines.
    /// Maintained for backward compatibility with existing callers.
    static func engineRootCandidates(in bundle: Bundle = .main) -> [String] {
        var candidates: [String] = []

        if let bundled = bundledEngineRoot(in: bundle) {
            candidates.append(bundled)
        }

        if let resourceURL = bundle.resourceURL {
            let engineDir = resourceURL.appendingPathComponent(engineDirectoryName, isDirectory: true).path
            if !candidates.contains(engineDir) {
                candidates.append(engineDir)
            }
        }

        let contentsEngine = bundle.bundleURL
            .appendingPathComponent("Contents/Resources", isDirectory: true)
            .appendingPathComponent(engineDirectoryName, isDirectory: true)
            .path
        if !candidates.contains(contentsEngine) {
            candidates.append(contentsEngine)
        }

        return candidates
    }

    /// Returns the expected bundled engine root path, used as a fallback
    /// when the engine hasn't been validated yet.
    static func expectedBundledEngineRoot(in bundle: Bundle = .main) -> String {
        if let resourceURL = bundle.resourceURL {
            return resourceURL.appendingPathComponent(engineDirectoryName, isDirectory: true).path
        }
        return bundle.bundleURL
            .appendingPathComponent("Contents/Resources", isDirectory: true)
            .appendingPathComponent(engineDirectoryName, isDirectory: true)
            .path
    }

    // MARK: - Helpers

    private static func isRegularFile(at path: String, fileManager: FileManager = .default) -> Bool {
        var isDirectory: ObjCBool = false
        return fileManager.fileExists(atPath: path, isDirectory: &isDirectory) && !isDirectory.boolValue
    }
}
