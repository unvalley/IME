import Carbon
import Foundation

struct InputRuntimeOptions: Equatable {
    let liveConversion: Bool
    let historyCompletion: Bool
    let historyLearning: Bool
    let dictionaryPacks: UInt32

    init(
        liveConversion: Bool,
        historyCompletion: Bool,
        historyLearning: Bool,
        dictionaryPacks: UInt32,
        secureEventInput: Bool
    ) {
        self.liveConversion = liveConversion
        self.historyCompletion = historyCompletion
        self.historyLearning = historyLearning && !secureEventInput
        self.dictionaryPacks = dictionaryPacks
    }
}

/// HIToolbox documents this process-wide query as not thread-safe. InputMethodKit
/// normally calls the controller on the main thread; if it does not, fail closed
/// and pause learning for that event instead of inspecting unsafe global state.
func secureEventInputIsEnabled() -> Bool {
    guard Thread.isMainThread else {
        return true
    }
    return IsSecureEventInputEnabled()
}
