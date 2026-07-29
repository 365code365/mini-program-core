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
  s.vendored_frameworks = 'MiniRender.xcframework'
  s.swift_version       = '5.9'

  # QuickJS / openh264 是 C/C++，需要 C++ 运行时
  s.libraries = 'c++', 'z'
end
