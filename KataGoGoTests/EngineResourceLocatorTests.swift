import XCTest
@testable import KataGoGo

final class EngineResourceLocatorTests: XCTestCase {
    private var temporaryDirectories: [URL] = []

    override func tearDownWithError() throws {
        for directory in temporaryDirectories {
            try? FileManager.default.removeItem(at: directory)
        }
        temporaryDirectories.removeAll()
    }

    func testBundledEngineUsesMediumModelWhenItIsTheOnlyBundledModel() throws {
        let root = try makeEngineDirectory()
        try writeRequiredEngineFiles(to: root, excluding: ["kata1-b18c384nbt.bin.gz"])

        XCTAssertNil(EngineResourceLocator.bundledEnginePaths(in: root.path))

        try writeFile("model", to: root.appendingPathComponent("kata1-b18c384nbt.bin.gz"))

        let paths = EngineResourceLocator.bundledEnginePaths(in: root.path)
        XCTAssertEqual(paths?.binary, root.appendingPathComponent(EngineResourceLocator.binaryFileName).path)
        XCTAssertEqual(paths?.config, root.appendingPathComponent("gtp.cfg").path)
        XCTAssertEqual(paths?.model, root.appendingPathComponent("kata1-b18c384nbt.bin.gz").path)
    }

    func testBundledEnginePrefersTinyModelWhenAvailable() throws {
        let root = try makeEngineDirectory()
        try writeRequiredEngineFiles(to: root, excluding: ["kata1-b18c384nbt.bin.gz"])
        try writeFile("tiny", to: root.appendingPathComponent("lionffen_b24c64.bin.gz"))

        let paths = EngineResourceLocator.bundledEnginePaths(in: root.path)
        XCTAssertEqual(paths?.model, root.appendingPathComponent("lionffen_b24c64.bin.gz").path)
    }

    private func makeEngineDirectory() throws -> URL {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        temporaryDirectories.append(directory)
        return directory
    }

    private func writeFile(_ contents: String, to url: URL) throws {
        try contents.write(to: url, atomically: true, encoding: .utf8)
    }

    private func writeRequiredEngineFiles(to root: URL, excluding excludedFiles: Set<String> = []) throws {
        let requiredFiles = [
            EngineResourceLocator.binaryFileName,
            "gtp.cfg",
            "kata1-b18c384nbt.bin.gz",
        ]

        for fileName in requiredFiles where !excludedFiles.contains(fileName) {
            try writeFile(fileName, to: root.appendingPathComponent(fileName))
        }

        try FileManager.default.setAttributes(
            [.posixPermissions: 0o755],
            ofItemAtPath: root.appendingPathComponent(EngineResourceLocator.binaryFileName).path
        )
    }
}
