// swift-tools-version:5.9
//
// macOS 版 SDK（AppKit）。业务工程加依赖：
//
//   .package(path: "../mini-program-core/sdk/macos")      // 本地路径
//   import MiniRender                                       // 与 iOS 同一个模块名、同一套 API 形状
//
// 前提：先跑 `bash tools/build-mobile.sh macos` 生成 MiniRender.xcframework（Apple 芯片；MAC_UNIVERSAL=1 再带 Intel）。
// 示例 App（PC 微信式的「灵犀 for Mac」）在 example/。
import PackageDescription

let package = Package(
    name: "MiniRenderMac",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "MiniRender", targets: ["MiniRender"])
    ],
    targets: [
        // Rust 引擎的静态库（含 C ABI 头与 module.modulemap）
        .binaryTarget(name: "MiniRenderFFI", path: "MiniRender.xcframework"),
        .target(
            name: "MiniRender",
            dependencies: ["MiniRenderFFI"],
            path: "Sources/MiniRender",
            linkerSettings: [
                // QuickJS / openh264 是 C/C++；<video> 的声音走系统音频（cpal → CoreAudio / AudioUnit）
                .linkedLibrary("c++"),
                .linkedLibrary("z"),
                .linkedFramework("Metal"),
                .linkedFramework("QuartzCore"),
                .linkedFramework("CoreGraphics"),
                .linkedFramework("CoreAudio"),
                .linkedFramework("AudioToolbox"),
                .linkedFramework("AudioUnit"),
                .linkedFramework("CoreFoundation"),
                .linkedFramework("Security"),
                .linkedFramework("SystemConfiguration"),
            ]
        ),
    ]
)
