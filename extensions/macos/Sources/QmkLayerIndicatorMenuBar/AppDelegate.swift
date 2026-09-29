import AppKit

/// Owns the menu bar status item and mirrors the current layer into it. Like
/// the GNOME Shell extension, the item shows the layer icon followed by the
/// alias (falling back to the layer number).
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    /// Monospaced system font, slightly larger than the menu bar size so the
    /// layer label stands out, while keeping every glyph a consistent width.
    private static let labelFont = NSFont.monospacedSystemFont(ofSize: 14, weight: .medium)
    /// Gap between the icon and the label, in points.
    private static let iconLabelSpacing: CGFloat = 3

    private var statusItem: NSStatusItem?
    private let statusMenuItem = NSMenuItem(title: "", action: nil, keyEquivalent: "")
    private let connection = LayerConnection.shared
    /// Title currently drawn into the status item image, so we can skip
    /// recomposing the image when only the connection status changed.
    private var renderedTitle: String?

    func applicationDidFinishLaunching(_ notification: Notification) {
        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        item.button?.imagePosition = .imageOnly

        let menu = NSMenu()
        statusMenuItem.isEnabled = false
        menu.addItem(statusMenuItem)
        menu.addItem(.separator())
        menu.addItem(
            withTitle: "Quit QMK Layer Indicator",
            action: #selector(NSApplication.terminate(_:)),
            keyEquivalent: "q"
        )
        item.menu = menu
        statusItem = item

        connection.onChange = { [weak self] in self?.render() }
        render()
        connection.start()
    }

    private func render() {
        guard let button = statusItem?.button else { return }
        let title = connection.title
        if title != renderedTitle {
            button.image = Self.composedImage(text: title)
            renderedTitle = title
        }
        button.toolTip = connection.status
        button.setAccessibilityLabel(connection.status)
        statusMenuItem.title = connection.status
    }

    /// Draws the icon and label into a single template image with both
    /// vertically centered, so the menu bar tints and aligns them together.
    /// The icon is centered by its box and the label by its cap-height band,
    /// which lines the glyphs up with the icon.
    private static func composedImage(text: String) -> NSImage {
        let icon = LayerIcon.image
        let iconSize = icon.size
        let font = labelFont
        let attributes: [NSAttributedString.Key: Any] = [
            .font: font,
            .foregroundColor: NSColor.black,
        ]

        let textSize = text.isEmpty
            ? .zero
            : (text as NSString).size(withAttributes: attributes)
        let spacing = text.isEmpty ? 0 : iconLabelSpacing
        let lineHeight = ceil(font.ascender - font.descender)
        let height = max(iconSize.height, lineHeight)
        let width = ceil(iconSize.width + spacing + textSize.width)

        let image = NSImage(size: NSSize(width: width, height: height))
        image.lockFocus()
        icon.draw(in: NSRect(
            x: 0,
            y: ((height - iconSize.height) / 2).rounded(),
            width: iconSize.width,
            height: iconSize.height
        ))
        if !text.isEmpty {
            let baseline = height / 2 - font.capHeight / 2
            let originY = (baseline + font.descender).rounded()
            (text as NSString).draw(
                at: NSPoint(x: iconSize.width + spacing, y: originY),
                withAttributes: attributes
            )
        }
        image.unlockFocus()
        image.isTemplate = true
        return image
    }
}
