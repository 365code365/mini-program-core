# iOS 随包字体（MiniRender Sans）

iOS 上微信的页面字体栈是 `-apple-system`：西文用 SF，汉字回退到苹方。引擎在 iOS 上照这个来：

| 文字 | 用什么画 | 来源 |
|------|----------|------|
| 西文、数字、半角标点 | 系统的 SF（`/System/Library/Fonts/Core/SFUI.ttf`，glyf 可变字体，wght 400 / 700、opsz 17） | 用户手机上自带，不随包分发 |
| 汉字、全角标点 | MiniRender Sans（本目录 `bake.py` 生成，放在 `sdk/ios/Sources/MiniRender/Fonts/`） | 随包 |
| SF 读不到时的西文 | MiniRender Sans 里的 Inter | 随包 |

苹方读不了：它是苹果私有的 `hvgl` 轮廓（没有 glyf / CFF 表），所以汉字只能拿开源黑体顶替。

## MiniRender Sans 是怎么拼的

`bake.py` 把两份 SIL OFL 1.1 的可变字体按字重取出静态实例，再拼成一份 TrueType：

- 汉字：思源黑体 CN VF，常规取 wght 380，粗体取 580。这两个值是按每字墨量对到苹方 Regular / Semibold 定的。
  另外整体下移 18 单位，让汉字在基线上的位置与苹方一致。
- 西文：Inter VF，常规取 wght 400，粗体取 600，opsz 14，缩到 0.98。
- 行度量（hhea / OS/2）照苹方 SC。

字体名改成 MiniRender Sans，因为「Source」是思源黑体的保留字体名，修改过的版本不能再用它。
两份许可证在 `sdk/ios/Sources/MiniRender/Fonts/LICENSE-*.txt`。

```bash
python3 -m venv /tmp/fv && /tmp/fv/bin/pip install fonttools
/tmp/fv/bin/python tools/ios-fonts/bake.py SourceHanSansCN-VF.otf InterVariable.ttf 380 400 MiniRenderSans-Regular.ttf MiniRenderSans-Regular
/tmp/fv/bin/python tools/ios-fonts/bake.py SourceHanSansCN-VF.otf InterVariable.ttf 580 600 MiniRenderSans-Bold.ttf MiniRenderSans-Bold
```

## 引擎侧的 iOS 口径（`MiniFonts.swift` 设的环境变量）

| 变量 | 值 | 作用（代码） |
|------|----|------|
| `MINI_MAIN_FONT` / `MINI_BOLD_FONT` | 随包的 MiniRender Sans | 主字体与粗体（`text_system.rs`、`text/load.rs`） |
| `MINI_LATIN_FONT` / `MINI_LATIN_BOLD_FONT` | `…/SFUI.ttf@400@17` / `@700@17` | 西文用 SF，并加 SF 的 `trak` 字距（`text/latin.rs`） |
| `MINI_LINE_BOX` | `ios@<屏幕倍率>` | 按 WebKit 的口径定行盒、基线、汉字字宽、emoji 大小（`text/ios_box.rs`） |

不设这些变量时行为不变（桌面、Android 的像素基线不受影响）。

## 对照基准与分数

参考图是 iOS 模拟器上 WKWebView 渲染同一页面编译出的 H5（375×667 @3x，苹方 + SF）。分数是每 48px 方块
在 ±4px 内找最贴的对齐后的平均灰度残差，只算有字的方块（越小越像）。12 页里去掉 sample 首页
（参考图里弹出了延时浮层）后的合计：

| 阶段 | 残差 |
|------|------|
| 思源黑体 Regular / Medium + 原来的行盒 | 13.60 |
| + iOS 行盒（`line-height: normal`）+ MiniRender Sans | 9.86 |
| + 按 WebKit 的半行距放基线、progress 去掉多余的留白 | 7.97 |
| + 汉字按苹方 UI 的字宽（1028 单位 em + trak） | 6.41 |
| + 西文用系统 SF、input 去掉默认内边距、emoji 按 iOS 放大 | 5.66 |

「残差 >12 的方块」占比同期从 36.3% 降到 16.1%。
