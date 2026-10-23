# 宿主能力：选图 / 选文件 / 补全示例 API（2026-10-23）

## 结论（调查）
- hostCall 通道已有：host.js `__installHostApis` → 事件 `hostCall` → 宿主 `mr_app_host_reply` → `__resolveHostCall`。
- chooseImage 等在 system.js 的默认 fail 名单；Flutter 示例的 chooseImage 返回假路径 `/fake/photo_N.jpg`。
- 示例 demo 宿主页：chooseMedia / chooseLocation 故意不点名（立刻 fail）。
- 桌面窗体不装 hostApis、丢弃 hostCall；Cargo 里没有文件对话框依赖。
- `<image src>` 认 `wxfile://` 与绝对路径；FileSystemManager / saveFile 只认 `wxfile://` 与包内路径。
- 引擎图片解码不处理 EXIF 方向 → 宿主交来的照片要宿主侧摆正。
- 示例里静态 grep 出来的 `wx.*` 只有 chooseImage 走默认 fail；onAppShow/onAppHide 已有（app-hooks.js）。

## 设计
- 回包里 `file://` 开头的字符串 = 宿主交给小程序的文件：引擎导入 `wxfile://tmp/`（已在 tmp 目录里的只换名不拷贝）。
- hostCall 事件加 `tempDir`（宿主可直接写进去，零拷贝）与 `files`（args 里 wxfile:// → 真实路径）。
- host-media.js：chooseImage/chooseMedia/chooseVideo 参数归一 + 移动端「拍照 / 从手机相册选择」操作菜单 + 回包补字段。
- 桌面：rfd（仅 macOS/Windows）+ `MINI_PICK_FILES` 无头注入。
- Flutter 插件内置 chooseImage/chooseMedia/chooseVideo/chooseMessageFile/saveImageToPhotosAlbum/saveVideoToPhotosAlbum/openDocument（Kotlin + Swift 原生选择器）。
- 示例：其余默认 fail 名单里的 API 给演示实现；demo 宿主页加按钮、显示选到的图。

## 进度
- [x] 引擎（runtime/app_host.rs、wxfile.rs、host.js、host-media.js；tests/host_media.rs 5/5）
- [x] 桌面（app_window/host_calls.rs；rfd =0.17.2 仅 macOS/Windows；无头 MINI_PICK_FILES 验过）
- [x] Flutter Dart（media.dart、controller builtinHostApis；flutter test 34/34）
- [x] Kotlin（MediaPicker.kt / MediaImport.kt / FileProvider；gradle 编过；真机 PLY110 相册选 2 张 → 页面显示）
- [x] Swift（MediaPicker.swift；本机无 Xcode，只用 Mac Catalyst + Flutter 桩做了 typecheck，iOS 13/14 目标都过）
- [x] 示例（host_apis.dart 补齐默认 fail 名单其余 API；demo 宿主页重排）
- [x] 文档（集成指南、Flutter README、两份 README 的 MINI_PICK_FILES）
- [x] 全量回归：target/test_batches/20261023-161956-host-media —— 976 测试过；damage 的 slot 是 GIF 墙钟帧（单独重跑 5/5 一致）；
  gallery / H5（4.57% / 6.36%）与 15:50 那一批相同

## 顺带发现（不是本次引入）
- 真机 GPU 后端上 showActionSheet 的文字全是空白（feedback 页「普通」也一样）；CPU 后端正常。
- wxfile://tmp 从不清理（微信冷启动会清）。
