// swift-tools-version:5.9
//
// Swift Package。业务工程直接加依赖即可：
//
//   .package(url: "https://<你的仓库>/mini-render.git", from: "0.1.0")
//   // 或本地路径调试：.package(path: "../mini-program-core/sdk/ios")
//
// 前提：先跑 `bash tools/build-mobile.sh ios` 生成 MiniRender.xcframework
// （真机 arm64 + 模拟器 arm64，头文件里带 module.modulemap，Swift 才能 import）。
import PackageDescription

let package = Package(
    name: "MiniRender",
    platforms: [.iOS(.v13)],
    products: [
        .library(name: "MiniRender", targets: ["MiniRender"])
    ],
    targets: [
        // Rust 引擎的静态库（含 C ABI 头）
        .binaryTarget(name: "MiniRenderFFI", path: "MiniRender.xcframework"),
        // Swift 封装：集成方只用 MiniProgramView
        .target(
            name: "MiniRender",
            dependencies: ["MiniRenderFFI"],
            path: "Sources/MiniRender",
            linkerSettings: [
                // QuickJS / openh264 需要 C++ 运行时；网络走 ureq，需要系统库
                .linkedLibrary("c++"),
                .linkedLibrary("z"),
            ]
        ),
    ]
)
