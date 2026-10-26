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
