import Foundation

enum EngineResourceLocator {
    static let engineDirectoryName = "kata-engine"
    static let binaryFileName = "katago-metal"
    static let configFileName = "gtp.cfg"
    static let mediumModelFileName = "kata1-b18c384nbt.bin.gz"
    static let bundledModelFileNames = [mediumModelFileName]
    static var defaultBundledModelFileName: String { mediumModelFileName }

    static func bundledEnginePaths(in root: String) -> (binary: String, config: String, model: String)? {
        let binary = "\(root)/\(binaryFileName)"
        let config = "\(root)/\(configFileName)"
        let fileManager = FileManager.default
        let model = bundledModelFileNames
            .map { "\(root)/\($0)" }
            .first { isRegularFile(at: $0, fileManager: fileManager) }

        guard isRegularFile(at: config, fileManager: fileManager),
              let model,
              isRegularFile(at: binary, fileManager: fileManager),
              fileManager.isExecutableFile(atPath: binary) else {
            return nil
        }

        return (binary: binary, config: config, model: model)
    }

    static func engineRootCandidates(in bundle: Bundle = .main) -> [String] {
        var candidates: [URL] = []

        if let resourceURL = bundle.resourceURL {
            candidates.append(resourceURL.appendingPathComponent(engineDirectoryName, isDirectory: true))
        }

        if let binaryURL = bundle.url(
            forResource: binaryFileName,
            withExtension: nil,
            subdirectory: engineDirectoryName
        ) {
            candidates.append(binaryURL.deletingLastPathComponent())
        }

        candidates.append(
            bundle.bundleURL
                .appendingPathComponent("Contents/Resources", isDirectory: true)
                .appendingPathComponent(engineDirectoryName, isDirectory: true)
        )

        var seen = Set<String>()
        return candidates.map(\.path).filter { seen.insert($0).inserted }
    }

    static func expectedBundledEngineRoot(in bundle: Bundle = .main) -> String {
        if let resourceURL = bundle.resourceURL {
            return resourceURL.appendingPathComponent(engineDirectoryName, isDirectory: true).path
        }

        return bundle.bundleURL
            .appendingPathComponent("Contents/Resources", isDirectory: true)
            .appendingPathComponent(engineDirectoryName, isDirectory: true)
            .path
    }

    private static func isRegularFile(at path: String, fileManager: FileManager) -> Bool {
        var isDirectory: ObjCBool = false
        return fileManager.fileExists(atPath: path, isDirectory: &isDirectory) && !isDirectory.boolValue
    }
}
