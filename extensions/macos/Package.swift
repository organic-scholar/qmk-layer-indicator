// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "QmkLayerIndicatorMenuBar",
    platforms: [.macOS(.v13)],
    targets: [
        .executableTarget(
            name: "QmkLayerIndicatorMenuBar",
            path: "Sources/QmkLayerIndicatorMenuBar"
        )
    ]
)
