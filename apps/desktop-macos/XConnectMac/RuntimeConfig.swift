import Foundation

struct RuntimeConfig: Equatable {
    var apiBaseURL: String
    var turnURIs: [String]
    var turnUsernameMode: String
    var turnSecret: String
    var tlsPinMode: String

    static let fallback = RuntimeConfig(
        apiBaseURL: "https://api.example.com",
        turnURIs: [
            "turn:turn.example.com:3478?transport=udp",
            "turn:turn.example.com:3478?transport=tcp",
            "turns:turn.example.com:5349?transport=tcp",
        ],
        turnUsernameMode: "shared_secret",
        turnSecret: "replace_with_very_long_random_secret",
        tlsPinMode: "disabled"
    )

    init(
        apiBaseURL: String,
        turnURIs: [String],
        turnUsernameMode: String,
        turnSecret: String,
        tlsPinMode: String
    ) {
        self.apiBaseURL = apiBaseURL
        self.turnURIs = turnURIs
        self.turnUsernameMode = turnUsernameMode
        self.turnSecret = turnSecret
        self.tlsPinMode = tlsPinMode
    }

    init?(plist: [String: Any]) {
        guard let apiBaseURL = plist["API_BASE_URL"] as? String,
              let turnURIs = plist["TURN_URIS"] as? [String],
              let turnUsernameMode = plist["TURN_USERNAME_MODE"] as? String,
              let turnSecret = plist["TURN_SECRET"] as? String,
              let tlsPinMode = plist["TLS_PIN_MODE"] as? String
        else {
            return nil
        }

        self.init(
            apiBaseURL: apiBaseURL,
            turnURIs: turnURIs,
            turnUsernameMode: turnUsernameMode,
            turnSecret: turnSecret,
            tlsPinMode: tlsPinMode
        )
    }

    static func loadFromBundle(_ bundle: Bundle = .main) -> RuntimeConfig {
        guard let url = bundle.url(forResource: "RuntimeConfig", withExtension: "plist"),
              let data = try? Data(contentsOf: url),
              let plist = try? PropertyListSerialization.propertyList(
                  from: data,
                  options: [],
                  format: nil
              ) as? [String: Any],
              let config = RuntimeConfig(plist: plist)
        else {
            return .fallback
        }

        return config
    }

    var envSnippet: String {
        [
            "API_BASE_URL=\(apiBaseURL)",
            "TURN_URIS=\(turnURIs.joined(separator: ","))",
            "TURN_USERNAME_MODE=\(turnUsernameMode)",
            "TURN_SECRET=\(turnSecret)",
            "TLS_PIN_MODE=\(tlsPinMode)",
        ].joined(separator: "\n")
    }
}

final class RuntimeConfigStore: ObservableObject {
    private let bundledConfig: RuntimeConfig

    @Published var apiBaseURL: String
    @Published var turnURIsText: String
    @Published var turnUsernameMode: String
    @Published var turnSecret: String
    @Published var tlsPinMode: String

    init(bundle: Bundle = .main) {
        let config = RuntimeConfig.loadFromBundle(bundle)
        self.bundledConfig = config
        self.apiBaseURL = config.apiBaseURL
        self.turnURIsText = config.turnURIs.joined(separator: "\n")
        self.turnUsernameMode = config.turnUsernameMode
        self.turnSecret = config.turnSecret
        self.tlsPinMode = config.tlsPinMode
    }

    var effectiveConfig: RuntimeConfig {
        RuntimeConfig(
            apiBaseURL: apiBaseURL.trimmingCharacters(in: .whitespacesAndNewlines),
            turnURIs: turnURIsText
                .split(whereSeparator: \.isNewline)
                .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                .filter { !$0.isEmpty },
            turnUsernameMode: turnUsernameMode.trimmingCharacters(in: .whitespacesAndNewlines),
            turnSecret: turnSecret.trimmingCharacters(in: .whitespacesAndNewlines),
            tlsPinMode: tlsPinMode.trimmingCharacters(in: .whitespacesAndNewlines)
        )
    }

    func resetToBundledDefaults() {
        apiBaseURL = bundledConfig.apiBaseURL
        turnURIsText = bundledConfig.turnURIs.joined(separator: "\n")
        turnUsernameMode = bundledConfig.turnUsernameMode
        turnSecret = bundledConfig.turnSecret
        tlsPinMode = bundledConfig.tlsPinMode
    }
}
