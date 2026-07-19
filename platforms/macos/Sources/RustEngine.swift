import Foundation

final class RustEngine {
    enum Event {
        case character(Unicode.Scalar)
        case space
        case enter
        case escape
        case backspace
        case nextCandidate
        case previousCandidate

        fileprivate var rawValue: UInt32 {
            switch self {
            case .character: 0
            case .space: 1
            case .enter: 2
            case .escape: 3
            case .backspace: 4
            case .nextCandidate: 5
            case .previousCandidate: 6
            }
        }

        fileprivate var scalar: UInt32 {
            guard case let .character(value) = self else { return 0 }
            return value.value
        }
    }

    struct Action: Decodable, Equatable {
        let type: String
        let text: String?
        let candidates: [String]?
        let selected: Int?
    }

    enum EngineError: Error, Equatable {
        case creationFailed
        case invalidBuffer
        case rejected(String)
    }

    private struct Response: Decodable {
        let ok: Bool
        let actions: [Action]?
        let error: String?
    }

    private let handle: OpaquePointer

    init() throws {
        guard let handle = ime_create() else {
            throw EngineError.creationFailed
        }
        self.handle = handle
    }

    deinit {
        ime_destroy(handle)
    }

    func process(_ event: Event) throws -> [Action] {
        let buffer = ime_process(handle, event.rawValue, event.scalar)
        defer { ime_buffer_destroy(buffer) }

        guard let bytes = buffer.data, buffer.len > 0 else {
            throw EngineError.invalidBuffer
        }

        let data = Data(bytes: bytes, count: buffer.len)
        let response = try JSONDecoder().decode(Response.self, from: data)
        guard response.ok else {
            throw EngineError.rejected(response.error ?? "unknown_error")
        }
        return response.actions ?? []
    }
}
