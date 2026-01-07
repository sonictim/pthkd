// swift-tools-version: 5.8
import PackageDescription

let package = Package(
    name: "PTHKDui",
    platforms: [.macOS(.v12)],
    products: [
        .library(name: "PTHKDui", type: .dynamic, targets: ["PTHKDui"])
    ],
    targets: [
        .target(name: "PTHKDui", dependencies: [])
    ]
)
