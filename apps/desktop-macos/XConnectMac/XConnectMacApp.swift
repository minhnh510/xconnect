import SwiftUI

@main
struct XConnectMacApp: App {
    @StateObject private var store = RuntimeConfigStore()

    var body: some Scene {
        WindowGroup("XConnect Runtime Config") {
            ContentView(store: store)
        }
    }
}
