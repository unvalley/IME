import Foundation

@main
enum AdapterTests {
    static func main() throws {
        let engine = try RustEngine()
        var latestPreedit: String?

        for scalar in "nihon".unicodeScalars {
            let actions = try engine.process(.character(scalar))
            latestPreedit = actions.last(where: { $0.type == "update_preedit" })?.text
        }
        try expect(latestPreedit == "にほん", "romaji preedit should become にほん")

        let conversion = try engine.process(.space)
        let candidateAction = conversion.first(where: { $0.type == "show_candidates" })
        try expect(candidateAction?.candidates?.contains("日本") == true, "日本 should be a candidate")

        let commit = try engine.process(.enter)
        try expect(
            commit.contains(where: { $0.type == "commit" && $0.text == "日本" }),
            "selected candidate should be committed"
        )

        print("macOS Swift adapter tests passed")
    }

    private static func expect(_ condition: @autoclosure () -> Bool, _ message: String) throws {
        guard condition() else {
            throw TestFailure(message: message)
        }
    }

    private struct TestFailure: Error, CustomStringConvertible {
        let message: String
        var description: String { message }
    }
}
