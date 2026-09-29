import Foundation

/// Connects to the qmk-layer-indicator backend WebSocket and tracks the current
/// layer for the menu bar. Mirrors the GNOME Shell client: it renders the
/// configured alias (falling back to the layer number), ignores heartbeat
/// frames, and reconnects on failure.
@MainActor
final class LayerConnection {
    static let shared = LayerConnection()

    /// Text shown next to the icon: the alias, the layer number, or "?" while
    /// the backend is unavailable.
    private(set) var title: String = LayerConnection.waitingTitle
    /// Human-readable description used for the tooltip, menu, and accessibility.
    private(set) var status: String = LayerConnection.waitingStatus
    private(set) var isConnected: Bool = false

    /// Invoked on the main actor whenever `title`/`status` change.
    var onChange: (() -> Void)?

    private static let waitingTitle = "?"
    private static let waitingStatus = "Waiting for QMK Layer Indicator"
    private static let defaultURL = "ws://127.0.0.1:51837"
    private static let reconnectDelay: TimeInterval = 3

    private let url: URL
    private lazy var session = URLSession(configuration: .ephemeral)
    private var task: URLSessionWebSocketTask?
    private var reconnectWork: DispatchWorkItem?
    private var isStopped = false
    private var hasStarted = false

    init() {
        let configured = ProcessInfo.processInfo
            .environment["QMK_LAYER_INDICATOR_WEBSOCKET_URL"] ?? Self.defaultURL
        self.url = URL(string: configured) ?? URL(string: Self.defaultURL)!
    }

    func start() {
        guard !hasStarted else { return }
        hasStarted = true
        isStopped = false
        connect()
    }

    func stop() {
        isStopped = true
        reconnectWork?.cancel()
        reconnectWork = nil
        task?.cancel(with: .goingAway, reason: nil)
        task = nil
    }

    private func connect() {
        setWaiting()
        let task = session.webSocketTask(with: url)
        self.task = task
        task.resume()
        receive(on: task)
    }

    private func receive(on task: URLSessionWebSocketTask) {
        task.receive { [weak self] result in
            Task { @MainActor in
                guard let self, self.task === task, !self.isStopped else { return }
                switch result {
                case .success(let message):
                    self.handle(message)
                    self.receive(on: task)
                case .failure:
                    self.handleDisconnect()
                }
            }
        }
    }

    private func handle(_ message: URLSessionWebSocketTask.Message) {
        let text: String
        switch message {
        case .string(let string):
            text = string
        case .data(let data):
            text = String(decoding: data, as: UTF8.self)
        @unknown default:
            return
        }

        guard
            let data = text.data(using: .utf8),
            let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return }

        if object["type"] as? String == "heartbeat" { return }

        guard
            let layer = object["layer"] as? Int, layer >= 0,
            let alias = object["alias"] as? String
        else { return }

        isConnected = true
        title = alias.isEmpty ? String(layer) : alias
        status = alias.isEmpty
            ? "QMK keyboard layer \(layer)"
            : "QMK keyboard layer \(layer): \(alias)"
        onChange?()
    }

    private func handleDisconnect() {
        task = nil
        setWaiting()
        guard !isStopped else { return }
        scheduleReconnect()
    }

    private func setWaiting() {
        isConnected = false
        title = Self.waitingTitle
        status = Self.waitingStatus
        onChange?()
    }

    private func scheduleReconnect() {
        reconnectWork?.cancel()
        let work = DispatchWorkItem { [weak self] in
            guard let self, !self.isStopped else { return }
            self.connect()
        }
        reconnectWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.reconnectDelay, execute: work)
    }
}
