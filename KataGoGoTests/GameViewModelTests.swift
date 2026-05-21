import XCTest
@testable import KataGoGo

final class GameViewModelTests: XCTestCase {

    // MARK: - continueFromReview truncation

    func testContinueFromReviewTruncatesMoveHistory() {
        let vm = GameViewModel()

        // Simulate: played 5 moves, entered review, jumped back to move 3
        vm.recordedMoves = [
            SavedMove(color: "b", vertex: "Q16"),
            SavedMove(color: "w", vertex: "D4"),
            SavedMove(color: "b", vertex: "C17"),
            SavedMove(color: "w", vertex: "R4"),
            SavedMove(color: "b", vertex: "K10"),
        ]
        vm.moveElapsedAtMove = [1.0, 5.2, 12.0, 18.5, 30.0]
        vm.moveAnalysisHistory = [
            MoveAnalysisSnapshot(moveNumber: 1, winrateBlack: 0.55, leadBlack: 1.5, evaluationAccuracy: 0.3, currentPlayer: "w"),
            MoveAnalysisSnapshot(moveNumber: 3, winrateBlack: 0.48, leadBlack: -0.5, evaluationAccuracy: 0.6, currentPlayer: "w"),
            MoveAnalysisSnapshot(moveNumber: 5, winrateBlack: 0.60, leadBlack: 3.0, evaluationAccuracy: 0.8, currentPlayer: "w"),
        ]
        vm.isReviewMode = true
        vm.replayTargetMove = 3

        vm.continueFromReview()

        XCTAssertEqual(vm.recordedMoves.count, 3, "Should keep only first 3 moves")
        XCTAssertEqual(vm.recordedMoves.last?.vertex, "C17", "Last move should be the 3rd move")
        XCTAssertEqual(vm.moveElapsedAtMove.count, 3, "moveElapsedAtMove should be truncated to 3")
        XCTAssertEqual(vm.moveElapsedAtMove.last, 12.0, "Last elapsed should match 3rd move")
        XCTAssertFalse(vm.moveAnalysisHistory.contains(where: { $0.moveNumber > 3 }),
                       "Analysis snapshots beyond move 3 should be removed")
        XCTAssertTrue(vm.moveAnalysisHistory.contains(where: { $0.moveNumber == 3 }),
                      "Analysis snapshot at move 3 should remain")
        XCTAssertFalse(vm.isReviewMode, "Should exit review mode")
        XCTAssertEqual(vm.replayTargetMove, 0, "replayTargetMove should reset")
    }

    func testContinueFromReviewAtFirstMoveClearsEverything() {
        let vm = GameViewModel()

        vm.recordedMoves = [
            SavedMove(color: "b", vertex: "Q16"),
            SavedMove(color: "w", vertex: "D4"),
        ]
        vm.moveElapsedAtMove = [1.0, 5.0]
        vm.moveAnalysisHistory = [
            MoveAnalysisSnapshot(moveNumber: 1, winrateBlack: 0.5, leadBlack: 0.0, evaluationAccuracy: 0.2, currentPlayer: "w"),
        ]
        vm.isReviewMode = true
        vm.replayTargetMove = 0

        vm.continueFromReview()

        XCTAssertTrue(vm.recordedMoves.isEmpty)
        XCTAssertTrue(vm.moveElapsedAtMove.isEmpty)
        XCTAssertTrue(vm.moveAnalysisHistory.isEmpty)
        XCTAssertFalse(vm.isReviewMode)
    }

    func testContinueFromReviewAtFullLengthKeepsAllMoves() {
        let vm = GameViewModel()

        vm.recordedMoves = [
            SavedMove(color: "b", vertex: "Q16"),
            SavedMove(color: "w", vertex: "D4"),
        ]
        vm.moveElapsedAtMove = [1.0, 5.0]
        vm.isReviewMode = true
        vm.replayTargetMove = 2

        vm.continueFromReview()

        XCTAssertEqual(vm.recordedMoves.count, 2, "All moves should be kept when replayTargetMove equals total")
        XCTAssertEqual(vm.moveElapsedAtMove.count, 2)
        XCTAssertFalse(vm.isReviewMode)
    }

    func testContinueFromReviewDoesNothingWhenNotInReviewMode() {
        let vm = GameViewModel()

        vm.recordedMoves = [
            SavedMove(color: "b", vertex: "Q16"),
            SavedMove(color: "w", vertex: "D4"),
        ]
        vm.moveElapsedAtMove = [1.0, 5.0]
        vm.isReviewMode = false

        vm.continueFromReview()

        XCTAssertEqual(vm.recordedMoves.count, 2, "Moves should be untouched when not in review")
        XCTAssertFalse(vm.isReviewMode)
    }
}
