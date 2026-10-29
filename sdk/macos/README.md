# mini-render for macOS

AppKit 版 SDK 与 PC 微信式示例「灵犀 for Mac」。

```
sdk/macos/
├── Package.swift                     # Swift Package（模块名 MiniRender，与 iOS 同名同形状）
├── Sources/MiniRender/
│   ├── MiniProgramView.swift         # ← 集成方只用这个（NSView，CAMetalLayer GPU 合成，失败回退 CPU）
│   ├── EngineSession.swift           # 引擎线程、显示同步、GPU / CPU 上屏（内部）
│   ├── MiniKeyboard.swift            # 输入框 ↔ 系统输入法（拼音组字、方向键、回车）（内部）
│   └── MiniHostCall.swift / MiniProgramAbout.swift  # 软链到 iOS 那份，两端同一份源码
├── MiniRender.xcframework            # bash tools/build-mobile.sh macos 产出（不进仓库）
└── example/                          # 灵犀 for Mac（project.yml → XcodeGen）
```

## 跑起来

```bash
bash tools/mac-debug.sh                     # 编库 → 生成工程 → 编 → 启动
bash tools/mac-debug.sh --no-lib -- -open cinema   # 直接开商店里的「光影电影票」
```

## 三行接入

```swift
let mini = MiniProgramView(frame: window.contentView!.bounds)
mini.autoresizingMask = [.width, .height]
window.contentView!.addSubview(mini)
mini.open(appDir: unpackedDir)
```

透明标题栏（`.fullSizeContentView`）时把标题栏高度给 `mini.topInset`：引擎当状态栏，导航栏铺到最顶上、红绿灯压在上面。

## 示例的样子

照 macOS 版微信三栏：左侧栏（聊天 / 通讯录 / 收藏 / 发现）、中间列表、右边内容。

- **发现 → 小程序**：最近使用、我的常用、小游戏、按分类的全部小程序（mini-store 测试商店），右键收藏 / 删除；
- **聊天**：演示会话里的小程序分享卡片，点了打开；「文件传输助手」里是内置示例；
- 小程序在**手机大小的独立窗口**里开，胶囊「•••」出菜单（复制链接、收藏、重新进入、上传诊断日志、关于），「⊙」关窗口。

## 与 PC 微信的差别

- 同一时刻只开一个小程序（引擎的小程序根目录是进程级的单实例），再开一个就在同一个窗口里换掉；
- 朋友圈 / 视频号是占位；商店小程序没有封面图，小游戏卡片用底色 + 首字。

## 小游戏（game.json 包）

`MiniGameView` 跑微信小游戏，与 iOS / Android 的同名视图同一套事件：

```swift
let v = MiniGameView(gameDir: dir, options: ["appId": "wx123", "adaptiveFps": true])
v.topInset = 28                       // 红绿灯那一行：报成安全区内缩，胶囊摆在它下面
v.onHostEvent = { type, ev in         // menuButton {action: menu | close}、exit、restart、hostCall …
    if type == "menuButton", ev["action"] as? String == "close" { window.close() }
}
window.contentView?.addSubview(v)     // 进了窗口、有尺寸之后才建实例
```

- **方向**：桌面不转屏。`v.gameOrientation`（game.json 的 deviceOrientation）是横屏就把窗口开成横的；
  `wx.setDeviceOrientation` 走 `onOrientationChange`。示例里横屏窗口是 844×390 + 标题栏。
- **输入**：鼠标左键同时报成触摸（id 0）与 `wx.onMouseDown/Move/Up`，右键 / 中键只报鼠标，悬停移动报 `onMouseMove`，
  滚轮报 `wx.onWheel`，键盘报 `wx.onKeyDown / onKeyUp`（`key` / `code` 同 DOM，`code` 按物理键位）；⌘ 组合键留给菜单。
- **胶囊**：原生 `GameCapsule` 摆在引擎给的位置（`mr_game_menu_button`，与 `wx.getMenuButtonBoundingClientRect` 同一个数），
  `wx.setMenuStyle('dark')` 换深色；`"menuButton": false` 不画。
- **WebGL**：ANGLE（Metal 后端）的 `libEGL.dylib` / `libGLESv2.dylib` 要在 App 的 `Contents/Frameworks/` 里
  （引擎在 可执行文件/../Frameworks 里 dlopen）。`bash tools/build-mobile.sh macos` 会先跑 `tools/fetch-angle.sh`，
  示例工程的 postBuildScripts 把它们拷进去并签名。没有的话 `getContext('webgl')` 返回 null，2D 游戏照常。
- **键盘**：`wx.showKeyboard` 在窗口底部浮一条输入框 + 确认键（`GameKeyboard.swift`）。
- **音频**：cpal（CoreAudio）直接出声；macOS 没有 AVAudioSession，`wx.setInnerAudioOption` 只记日志。
- 没做：`wx.createVideo`（桌面上不显示视频）、传感器（Mac 没有）。

示例 `sdk/macos/example`：商店里点开的包是小游戏时，小程序窗口换成 `MiniGameView`（`App/GameWindow.swift`）：
「⊙」关窗口，「•••」弹 PC 微信式菜单（转发 / 收藏 / 重新进入 / 关于），示例宿主接支付 / 激励视频 / 分享 / 登录（演示实现）。
