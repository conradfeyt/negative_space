// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "NegativeSpaceIntelligence",
    platforms: [.macOS(.v13)],
    products: [
        .library(
            name: "NegativeSpaceIntelligence",
            type: .static,
            targets: ["NegativeSpaceIntelligence"]
        ),
    ],
    targets: [
        .target(
            name: "NegativeSpaceIntelligence",
            path: "Sources"
        ),
    ]
)
