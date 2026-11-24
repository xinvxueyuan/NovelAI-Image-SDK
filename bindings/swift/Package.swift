// swift-tools-version: 5.9
//
// Swift Package for novelai-image-sdk.
//
// UniFFI emits `NovelAIImageSDK.swift`, a C header and a module map. The native
// library is shipped as a binary target (an XCFramework built in CI), because a
// Swift package cannot compile Rust itself.

import PackageDescription

let package = Package(
    name: "NovelAIImageSDK",
    platforms: [
        .macOS(.v12),
        .iOS(.v15),
    ],
    products: [
        .library(name: "NovelAIImageSDK", targets: ["NovelAIImageSDK"]),
    ],
    targets: [
        // The generated bindings. `generated/` is produced by
        // `cargo xtask bindings --language swift`.
        .target(
            name: "NovelAIImageSDK",
            dependencies: ["NovelAIImageSDKFFI"],
            path: "generated",
            sources: ["NovelAIImageSDK.swift"]
        ),
        // The C header and module map UniFFI emits alongside the Swift file.
        .systemLibrary(
            name: "NovelAIImageSDKFFI",
            path: "generated/NovelAIImageSDKFFI"
        ),
        // The static library, built per-platform in CI and published as an
        // XCFramework artifact attached to the GitHub release. Replace `url`
        // and `checksum` with the values printed by `cargo xtask release`.
        .binaryTarget(
            name: "NovelAIImageSDKRust",
            url: "https://github.com/xinvxueyuan/NovelAI-Image-SDK/releases/download/v0.1.5/NovelAIImageSDK.xcframework.zip",
            checksum: "0000000000000000000000000000000000000000000000000000000000000000"
        ),
    ]
)
