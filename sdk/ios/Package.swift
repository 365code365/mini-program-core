// swift-tools-version:5.9
//
// Swift Package。业务工程直接加依赖即可：
//
//   .package(url: "https://<你的仓库>/mini-render.git", from: "0.1.0")
//   // 或本地路径调试：.package(path: "../mini-program-core/sdk/ios")
//
// 前提：先跑 `bash tools/build-mobile.sh ios` 生成 MiniRender.xcframework
// （真机 arm64 + 模拟器 arm64，头文件里带 module.modulemap，Swift 才能 import），
// 它顺带跑 tools/fetch-angle-ios.sh 把小游戏 WebGL 用的 ANGLE 动态框架放进 Angle/。
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
        // 小游戏的 WebGL：ANGLE（Metal 后端）的 EGL / GLES 动态框架。引擎运行时 dlopen
        // （src/game/webgl/egl.rs），这里只负责把它们嵌进 App 的 Frameworks/
        .binaryTarget(name: "libEGL", path: "Angle/libEGL.xcframework"),
        .binaryTarget(name: "libGLESv2", path: "Angle/libGLESv2.xcframework"),
        // Swift 封装：小程序用 MiniProgramView，小游戏用 MiniGameView
        .target(
            name: "MiniRender",
            dependencies: ["MiniRenderFFI", "libEGL", "libGLESv2"],
            path: "Sources/MiniRender",
            // 随包中文字体 MiniRender Sans（思源黑体 + Inter，OFL，tools/ios-fonts）：iOS 的苹方是私有轮廓格式，引擎读不了（MiniFonts.swift）
            resources: [.copy("Fonts")],
            linkerSettings: [
                // QuickJS / openh264 需要 C++ 运行时；网络走 ureq，需要系统库
                .linkedLibrary("c++"),
                .linkedLibrary("z"),
                // GPU 后端（wgpu 的 Metal，画到视图的 CAMetalLayer）：静态库不带框架依赖，要宿主链接时带上
                .linkedFramework("Metal"),
                .linkedFramework("QuartzCore"),
                .linkedFramework("CoreGraphics"),
                // 小游戏：音频会话与 wx.createVideo、传感器、罗盘
                .linkedFramework("AVFoundation"),
                .linkedFramework("CoreMotion"),
                .linkedFramework("CoreLocation"),
            ]
        ),
    ]
)
