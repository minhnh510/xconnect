import SwiftUI

struct ContentView: View {
    @ObservedObject var store: RuntimeConfigStore

    var body: some View {
        HStack(spacing: 24) {
            Form {
                Section("Runtime Config") {
                    TextField("API base URL", text: $store.apiBaseURL)

                    TextField("TURN username mode", text: $store.turnUsernameMode)

                    SecureField("TURN secret", text: $store.turnSecret)

                    TextField("TLS pin mode", text: $store.tlsPinMode)
                }

                Section("TURN_URIS") {
                    TextEditor(text: $store.turnURIsText)
                        .font(.system(.body, design: .monospaced))
                        .frame(minHeight: 140)
                }

                Section {
                    Button("Reset to bundled defaults") {
                        store.resetToBundledDefaults()
                    }
                }
            }
            .frame(minWidth: 420)

            VStack(alignment: .leading, spacing: 12) {
                Text("Effective Environment")
                    .font(.title3.weight(.semibold))

                Text("Bundled defaults are initialized from `RuntimeConfig.plist` and mirrored here as env-style values.")
                    .foregroundStyle(.secondary)

                ScrollView {
                    Text(store.effectiveConfig.envSnippet)
                        .font(.system(.body, design: .monospaced))
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(16)
                }
                .background(.quinary)
                .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .padding(24)
        .frame(minWidth: 920, minHeight: 560)
    }
}
