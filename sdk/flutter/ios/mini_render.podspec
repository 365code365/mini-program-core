#
# mini_render 的 iOS 部分（Flutter 插件）。
#
# 前提：先在仓库根目录跑 `bash tools/build-mobile.sh ios`：它产出
# sdk/ios/MiniRender.xcframework，并在本目录放一个指过去的软链 MiniRender.xcframework
# （CocoaPods 的 vendored 路径不能越出 podspec 所在目录）。
#
Pod::Spec.new do |s|
  s.name             = 'mini_render'
  s.version          = '0.1.0'
  s.summary          = '在 Flutter 里打开微信小程序（mini-render 引擎）'
  s.description      = <<-DESC
mini-render 是 Rust 自绘的小程序渲染引擎（对标 Skyline）。这个插件把它接进 Flutter：
引擎跑在专用线程，画面经 FlutterTexture（CVPixelBuffer，32BGRA）上屏。
                       DESC
  s.homepage         = 'https://github.com/365code365/mini-program-core'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'mini-render' => 'dev@example.com' }
  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.dependency 'Flutter'
  s.platform = :ios, '13.0'
  s.vendored_frameworks = 'MiniRender.xcframework'
  s.frameworks = 'CoreVideo', 'QuartzCore'
  # QuickJS / openh264 是 C/C++，需要 C++ 运行时
  s.libraries = 'c++', 'z'
  # XCFramework 只有 arm64 真机 + arm64 模拟器两片
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES', 'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386 x86_64' }
  s.swift_version = '5.9'
end
