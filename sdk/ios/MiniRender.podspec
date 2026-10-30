# CocoaPods 用。业务工程 Podfile：
#
#   pod 'MiniRender', :path => '../mini-program-core/sdk/ios'     # 本地
#   pod 'MiniRender', :git => 'https://<你的仓库>/mini-render.git' # 远端
#
# 前提：先跑 `bash tools/build-mobile.sh ios` 生成 MiniRender.xcframework。
Pod::Spec.new do |s|
  s.name         = 'MiniRender'
  s.version      = '0.1.0'
  s.summary      = '自绘小程序渲染引擎（不基于 WebView）'
  s.description  = <<-DESC
    用 Rust 从零实现的小程序渲染引擎：自己解析 WXML/WXSS、自己算布局、自己光栅化，
    逻辑层是 QuickJS。集成方只用 MiniProgramView 一个类。
  DESC
  s.homepage     = 'https://github.com/your-org/mini-render'
  s.license      = { :type => 'MIT' }
  s.author       = { 'mini-render' => 'dev@example.com' }
  s.platform     = :ios, '13.0'
  s.source       = { :git => 'https://github.com/your-org/mini-render.git', :tag => s.version.to_s }

  s.source_files        = 'Sources/MiniRender/**/*.swift'
  # 随包中文字体（思源黑体，OFL）：iOS 的苹方是私有轮廓格式，引擎读不了（MiniFonts.swift）
  s.resource_bundles    = { 'MiniRenderFonts' => ['Sources/MiniRender/Fonts/*'] }
  # Angle/：小游戏 WebGL 用的 ANGLE 动态框架（引擎运行时 dlopen，build-mobile.sh 调 tools/fetch-angle-ios.sh 拉）
  s.vendored_frameworks = 'MiniRender.xcframework', 'Angle/libEGL.xcframework', 'Angle/libGLESv2.xcframework'
  s.swift_version       = '5.9'

  # QuickJS / openh264 是 C/C++，需要 C++ 运行时
  s.libraries = 'c++', 'z'
  # GPU 后端（wgpu 的 Metal，画到视图的 CAMetalLayer）：静态库不带框架依赖；
  # 小游戏：AVFoundation（音频会话、wx.createVideo）、CoreMotion / CoreLocation（传感器、罗盘）
  s.frameworks = 'Metal', 'QuartzCore', 'CoreGraphics', 'AVFoundation', 'CoreMotion', 'CoreLocation'
end
