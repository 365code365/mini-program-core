# Mini Render

用 Rust 从零实现的轻量级**微信小程序渲染引擎**：内置 2D 光栅渲染、Flexbox 布局、WXML/WXSS 解析、完整 CSS 选择器与 `{{ }}` 表达式引擎，以及基于 QuickJS 的 JavaScript 运行时（App / Page / Component / 模块系统 / Promise），持续向微信 **Skyline** 渲染实现对齐。

> 纯 Rust、无系统 UI 依赖；核心库可编译为 **Android / iOS / Windows / macOS / Linux** 原生库，供各端集成。

---

## 目录

- [特性总览](#-特性总览)
- [快速开始](#-快速开始)
- [浏览器调试预览](#-浏览器调试预览)
- [场景画廊](#-场景画廊34-个真实渲染)
- [视频与 Canvas](#-视频与-canvas)
- [支持的组件](#-支持的组件)
- [CSS 支持](#-css-支持)
- [小程序生命周期](#-小程序生命周期)
- [架构](#-架构)
- [素材图片生成](#-素材图片生成)
- [多平台 SDK 编译](#-多平台-sdk-编译)
- [使用示例](#-使用示例)
- [测试](#-测试)
- [项目结构](#-项目结构)

---

## ✨ 特性总览

| 能力 | 说明 |
|------|------|
| 🎨 **2D 渲染引擎** | 纯 Rust；抗锯齿描边/圆角/图片、Alpha 混合、阴影、渐变、双线性采样 |
| 📐 **Flexbox 布局** | 基于 [Taffy]；含文本按容器宽度自动换行的二次布局修正 |
| 🧩 **30+ 组件** | view/text/image/button/input/scroll-view/swiper/**canvas**/**video** 等 |
| ⚡ **QuickJS 运行时** | 完整 JS，App/Page/Component、Behavior、CommonJS 模块、Promise 微任务 |
| 🎯 **CSS 选择器引擎** | 标签/类/`#id`/`*`/属性/组合器/特异性/`@import`/`var()`/`calc()` |
| 📄 **WXML 模板** | `wx:if/elif/else`、`wx:for`、`<block>`、真正的 `{{ }}` 表达式 |
| ♻️ **完整生命周期** | App / Page / Component 三级钩子 + 组件 `pageLifetimes` 联动 |
| 🎬 **视频播放** | 手写 MP4 解复用 + openh264 解码 + rodio 音频（桌面端 macOS 音频接口） |
| 🖌️ **Canvas 2D** | `wx.createCanvasContext` 全套 API：路径/圆弧/文本/图片/渐变/变换 |
| 👆 **事件系统** | `bindtap` 冒泡、`catchtap` 阻止冒泡、`dataset`、输入事件 |
| 🖱️ **交互** | 滚动惯性/回弹、输入框、勾选/单选/开关/滑块 |
| 🌐 **浏览器调试** | 内置 HTTP 服务，浏览器打开链接即可实时操作 UI |
| ⏱️ **高刷适配** | 事件循环按显示器刷新率出帧（60/120/144Hz），空闲零占用 |
| 🔗 **多端集成** | C FFI（`mr_*`），可编译为 5 大平台原生库 |

---

## 🚀 快速开始

```bash
# 1) 安装 Rust（若未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2) 构建
cargo build --release

# 3) 运行示例
cargo run --example gallery         # 渲染全部 34 个场景到 doc/gallery/
cargo run --bin mini-devserver      # 浏览器调试预览（见下节）
cargo run --example video_player    # 独立视频播放窗口（自动循环 + 声音）
cargo run --bin mini-app-window     # 窗口应用（加载 sample-app）
cargo run --bin mini-launcher       # 小程序启动器（扫描 sample 目录）

# 4) 测试（202 个用例）
cargo test
```

---

## 🌐 浏览器调试预览

无需模拟器，在**浏览器里直接操作小程序 UI**：内置的 HTTP 服务把页面实时渲染成图片推送到浏览器，点击画面即命中事件、调用页面方法、`setData` 后自动重渲染。

```bash
cargo run --bin mini-devserver                    # 默认加载 sample-app/pages/index
cargo run --bin mini-devserver <页面目录> [端口]   # 指定页面与端口（默认 9000）
```

启动后打开终端输出的 `http://127.0.0.1:9000` 即可。仅使用 Rust 标准库网络，无额外依赖。

| 路由 | 作用 |
|------|------|
| `GET /` | 调试页面（画面 + 点击捕获 + 自动刷新） |
| `GET /frame.png` | 当前页面的实时渲染图 |
| `GET /tap?x&y` | 命中事件 → 调用页面方法 → 重渲染 |

---

## 🖼️ 场景画廊（34 个真实渲染）

以下页面均由本引擎真实渲染输出（纯 WXML + WXSS + 数据），一键生成：

```bash
python3 scripts/gen_assets.py   # 首次：生成真实商品图/头像/图标素材
cargo run --example gallery     # 渲染 34 个场景到 doc/gallery/
```

### 电商 · 社交 · 导航

| | | |
|:---:|:---:|:---:|
| <img src="doc/gallery/01_login.png" width="230"/><br/>**登录页** | <img src="doc/gallery/02_product_list.png" width="230"/><br/>**商品列表** | <img src="doc/gallery/03_product_detail.png" width="230"/><br/>**商品详情** |
| <img src="doc/gallery/04_cart.png" width="230"/><br/>**购物车** | <img src="doc/gallery/05_profile.png" width="230"/><br/>**个人中心** | <img src="doc/gallery/06_settings.png" width="230"/><br/>**设置页** |
| <img src="doc/gallery/07_chat.png" width="230"/><br/>**聊天对话** | <img src="doc/gallery/08_feed.png" width="230"/><br/>**动态流** | <img src="doc/gallery/09_grid_menu.png" width="230"/><br/>**宫格导航** |

### 表单 · 数据 · 生活

| | | |
|:---:|:---:|:---:|
| <img src="doc/gallery/10_form.png" width="230"/><br/>**表单填写** | <img src="doc/gallery/11_dashboard.png" width="230"/><br/>**数据看板** | <img src="doc/gallery/12_gallery.png" width="230"/><br/>**图片画廊** |
| <img src="doc/gallery/13_weather.png" width="230"/><br/>**天气** | <img src="doc/gallery/14_orders.png" width="230"/><br/>**订单列表** | <img src="doc/gallery/15_home.png" width="230"/><br/>**商城首页** |
| <img src="doc/gallery/16_contacts.png" width="230"/><br/>**通讯录** | <img src="doc/gallery/17_music.png" width="230"/><br/>**音乐播放器** | <img src="doc/gallery/18_tags.png" width="230"/><br/>**标签与徽章** |

### 弹窗 · 滑动 · 交互

| | | |
|:---:|:---:|:---:|
| <img src="doc/gallery/19_modal.png" width="230"/><br/>**确认弹窗** | <img src="doc/gallery/20_action_sheet.png" width="230"/><br/>**操作面板** | <img src="doc/gallery/21_toast.png" width="230"/><br/>**Toast 提示** |
| <img src="doc/gallery/22_swiper.png" width="230"/><br/>**Swiper 轮播** | <img src="doc/gallery/23_h_scroll.png" width="230"/><br/>**左右滑动** | <img src="doc/gallery/24_v_scroll.png" width="230"/><br/>**上下滑动** |
| <img src="doc/gallery/25_picker.png" width="230"/><br/>**底部选择器** | <img src="doc/gallery/26_calendar.png" width="230"/><br/>**日历** | <img src="doc/gallery/27_rating_steps.png" width="230"/><br/>**评分与物流** |

### 电商大促 · 组件补充

| | | |
|:---:|:---:|:---:|
| <img src="doc/gallery/28_ecommerce_home.png" width="230"/><br/>**电商首页**（轮播/秒杀/瀑布流） | <img src="doc/gallery/29_coupon_popup.png" width="230"/><br/>**优惠券弹窗** | <img src="doc/gallery/30_tabbar.png" width="230"/><br/>**底部 TabBar** |
| <img src="doc/gallery/31_search_nav.png" width="230"/><br/>**自定义搜索栏** | <img src="doc/gallery/32_input_events.png" width="230"/><br/>**输入与事件** | <img src="doc/gallery/33_video.png" width="230"/><br/>**视频播放器** |
| <img src="doc/gallery/34_canvas.png" width="230"/><br/>**Canvas 2D 绘图** | | |

---

## 🎬 视频与 Canvas

### 视频

`<video>` 组件自带 MP4 解复用（区分音/视频轨道）、H.264 解码（openh264）、音频解码与播放（symphonia + rodio，桌面端默认 macOS 音频接口）。支持 `autoplay` / `loop` / `muted` / `controls`。

```bash
cargo run --example video_player    # 独立播放窗口（自动循环，按刷新率出帧）
cargo run --example video_decode    # 离屏抓帧验证解码（输出 doc/videos/frame_*.png）
```

```html
<video src="doc/videos/video.mp4" autoplay="true" loop="true" controls="true" object-fit="cover" />
```

### Canvas 2D

`wx.createCanvasContext(id)` 返回完整的 2D 上下文，命令经桥接在 native 侧光栅化：

```javascript
const ctx = wx.createCanvasContext('demo');
ctx.setFillStyle('#07c160');
ctx.fillRect(20, 20, 120, 80);
ctx.beginPath();
ctx.arc(100, 200, 50, 0, 2 * Math.PI);
ctx.setStrokeStyle('#ff3b30'); ctx.setLineWidth(6); ctx.stroke();
ctx.setFontSize(24); ctx.setTextAlign('center'); ctx.fillText('Hello', 100, 300);
ctx.save(); ctx.translate(200, 200); ctx.rotate(0.6); ctx.fillRect(-40, -40, 80, 80); ctx.restore();
ctx.draw();
```

支持：`fillRect/strokeRect/clearRect`、`beginPath/moveTo/lineTo/arc/quadraticCurveTo/bezierCurveTo/rect/closePath`、`fill/stroke`、`fillText/strokeText`、`drawImage`、`setFillStyle/StrokeStyle/LineWidth/LineCap/LineJoin/FontSize/TextAlign/TextBaseline/GlobalAlpha`、`save/restore`、以及 `translate/rotate/scale` 完整仿射变换、线性/径向渐变。

---

## 🧩 支持的组件

| 分类 | 组件 |
|------|------|
| 基础 | `view` `text` `image` `icon` `rich-text` |
| 表单 | `button` `input` `textarea` `checkbox` `checkbox-group` `radio` `radio-group` `switch` `slider` `progress` `picker` `picker-view` `picker-view-column` |
| 容器 | `scroll-view` `swiper` `swiper-item` |
| 媒体 | `video` `canvas` |

---

## 🎯 CSS 支持

- **选择器**：`view`、`.class`、`#id`、`*`、`.a.b`（复合）、`.a .b`（后代）、`.a > .b`（子）、`[type="primary"]` 属性选择器、伪类；按 (id, class, tag) 计算特异性并叠加书写顺序。
- **取值**：`rpx`/`px`/`%`/`vw`/`vh`/`em`/`rem`、`#rgb`/`#rrggbb`/`#rrggbbaa`/`rgb()`/`rgba()`/命名颜色/渐变、`var(--x, fallback)`、`calc(a + b)`（同单位）、`@import`。
- **布局**：`display` `flex-*` `justify-content` `align-*` `width/height/min/max` `padding/margin`（1–4 值简写、`auto` 居中）`position` `top/right/bottom/left` `gap`。
- **外观/文本/变换**：`background` `color` `border`（宽度精确、抗锯齿）`border-radius`（四角）`box-shadow` `opacity` `overflow`；`font-size` `font-weight` `text-align` `text-decoration` `line-height` `letter-spacing` `white-space` `text-overflow`；`transform`（translate/scale/rotate）`z-index`。
- **文本换行**：`display:block` / `width:100%` 的文本按容器宽度自动换行，盒子高度随行数增长（二次布局修正）。

---

## ♻️ 小程序生命周期

三级生命周期钩子均已接入（JS 注册 + Native 分发）：

| 层级 | 钩子 |
|------|------|
| **App** | `onLaunch` · `onShow` · `onHide` · `onError` · `onPageNotFound` · `onUnhandledRejection` · `onThemeChange` |
| **Page** | `onLoad` · `onShow` · `onReady` · `onHide` · `onUnload` · `onPullDownRefresh` · `onReachBottom` · `onPageScroll` · `onResize` · `onTabItemTap` · `onShareAppMessage` · `onShareTimeline` · `onAddToFavorites` |
| **Component** | `created` · `attached` · `ready` · `moved` · `detached`；`pageLifetimes`：`show` · `hide` · `resize` |

- 页面 `onShow/onHide/onResize` 会**联动**页面内组件的 `pageLifetimes`；
- 页面 `onUnload` 自动回收其组件实例并触发各组件 `detached`。

---

## 🏗️ 架构

```
┌─────────────────────────────────────────────────┐
│                    Mini App                       │
│  ┌──────────────────────────────────────────┐    │
│  │           JavaScript (QuickJS)            │    │
│  │  App · Page · Component · Behavior        │    │
│  │  require/module · Promise · setData 路径   │    │
│  └──────────────────────────────────────────┘    │
│                      ↕ Bridge                     │
│  ┌──────────────────────────────────────────┐    │
│  │              Native (Rust)                │    │
│  │  Canvas 渲染 · Taffy 布局 · 事件系统       │    │
│  │  WXML 解析 · WXSS 选择器引擎 · 模板引擎     │    │
│  └──────────────────────────────────────────┘    │
│                      ↕ FFI (mr_*)                 │
│  ┌──────────────────────────────────────────┐    │
│  │  Host: Android / iOS / Windows /          │    │
│  │        macOS / Linux / C / C++            │    │
│  └──────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

模块按职责细分，便于扩展维护：`parser/`（wxml/wxss/expr/template）、`renderer/`（wxml_renderer + `components/*` 每组件独立文件 + `style_parse` CSS 取值解析）、`layout/`、`js/`（runtime/api/bridge）、`ui/`（交互/滚动）、`runtime/`、`bin/`（各可执行程序）。

---

## 🖌️ 素材图片生成

画廊中的商品图/Banner/头像等**真实照片**由火山引擎「豆包·文生图」生成，播放器/TabBar 图标由 PIL 以 4x 超采样绘制为**抗锯齿透明 PNG**：

```bash
python3 scripts/gen_assets.py
# -> doc/gallery/assets/*.jpg          （AI 生成的商品/封面/头像/Banner）
# -> doc/gallery/assets/icons/*.png    （PIL 绘制的透明矢量图标）
```

`<image>` 支持本地文件与网络 URL，`mode` 支持 `scaleToFill/aspectFit/aspectFill/widthFix` 等；透明 PNG 不会被套上不透明底框，仅在加载失败时回退占位符。

---

## 📦 多平台 SDK 编译

`crate-type = ["cdylib", "staticlib", "rlib"]`，可产出动态库(.so/.dylib/.dll)、静态库(.a)与 Rust 库。仓库 `scripts/` 提供各平台构建脚本。

| 平台 | 脚本 | 产物 |
|------|------|------|
| macOS | `scripts/build-macos.sh` | `libmini_render.dylib` / `.a`（arm64+x86_64 通用） |
| Linux | `scripts/build-linux.sh [target]` | `libmini_render.so` / `.a` |
| Windows | `scripts/build-windows.ps1` | `mini_render.dll` + 导入库 `.lib` |
| Android | `scripts/build-android.sh` | 4 个 ABI 的 `libmini_render.so`（jniLibs 结构） |
| iOS | `scripts/build-ios.sh` | `MiniRender.xcframework` |

> 移动端需先启用 rquickjs 的 `bindgen`（`*-apple-ios`/`*-linux-android` 无预置绑定，需 libclang），并将桌面端依赖（winit/softbuffer/rodio/openh264/arboard/ureq）改为可选特性、以 `--no-default-features` 构建精简核心库。详见脚本内注释。

重新生成 C 头文件：

```bash
cargo install cbindgen
bash scripts/gen-header.sh   # -> include/mini_render.h
```

---

## 💡 使用示例

### Rust — 渲染 WXML/WXSS 到图片

```rust
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::{Canvas, Color};
use serde_json::json;

fn main() {
    let wxml = WxmlParser::new(r#"<view class="card"><text class="t">{{title}}</text></view>"#).parse().unwrap();
    let ss = WxssParser::new(r#".card{padding:24rpx;background-color:#fff;border-radius:16rpx;} .t{font-size:34rpx;color:#333;}"#).parse().unwrap();
    let mut r = WxmlRenderer::new(ss, 375.0, 667.0);
    let mut canvas = Canvas::new(375, 667);
    canvas.clear(Color::from_hex(0xF5F5F5));
    r.render(&mut canvas, &wxml, &json!({ "title": "Hello Mini" }));
    canvas.save_png("out.png").unwrap();
}
```

### Rust — 驱动小程序逻辑层

```rust
use mini_render::runtime::MiniApp;

fn main() -> Result<(), String> {
    let mut app = MiniApp::new(375, 667)?;
    app.init()?;
    app.define_module("utils/util", "exports.double = x => x * 2;")?;
    app.load_script(r#"
        const util = require('utils/util');
        Page({ data: { count: 0 }, inc() { this.setData({ count: util.double(this.data.count + 1) }); } });
    "#)?;
    app.eval("__currentPage.inc()")?;
    println!("data = {}", app.eval("__getPageData()")?); // {"count":2}
    Ok(())
}
```

### C / C++ — 通过 FFI 使用 2D 渲染

```c
#include "mini_render.h"
int main(void) {
    Canvas* c = mr_canvas_new(375, 667);
    mr_canvas_clear(c, 245, 245, 245, 255);
    mr_canvas_draw_circle(c, 70, 80, 30, 74, 144, 217, 255, 0, 0);
    mr_canvas_save_png(c, "card.png");
    mr_canvas_free(c);
    return 0;
}
```

```bash
cargo build --release
clang examples/demo.c -Iinclude -Ltarget/release -lmini_render -o demo_c
DYLD_LIBRARY_PATH=target/release ./demo_c
```

### Android / iOS

`libmini_render.so` / `MiniRender.xcframework` 暴露 C 接口，在 NDK / Swift 侧渲染后把 RGBA 拷入 `Bitmap` / `CGContext`：

```c
Canvas* c = mr_canvas_new(w, h);
mr_canvas_clear(c, 255, 255, 255, 255);
size_t need = (size_t)w * h * 4; uint8_t* buf = malloc(need);
mr_canvas_get_pixels(c, buf, need);   // RGBA8888 -> AndroidBitmap / UIImage
free(buf); mr_canvas_free(c);
```

---

## 🧪 测试

覆盖表达式引擎、WXSS 选择器（含 `var()`/`calc()`）、模板控制流、布局与文本换行、全组件渲染、Canvas 2D、交互、滚动/惯性、页面栈路由、组件模型、CommonJS 模块、Promise、生命周期、事件冒泡等：

```bash
cargo test          # 202 个用例
cargo test route    # 路由/页面栈/组件/模块/异步/生命周期
cargo test canvas   # Canvas 2D 上下文与命令
cargo test scroll   # 滚动与惯性
```

---

## 📁 项目结构

```
mini-render/
├── src/
│   ├── canvas.rs / color.rs / geometry.rs / paint.rs / path.rs / text.rs
│   ├── ffi.rs                  # C FFI (mr_*)
│   ├── event.rs                # 事件系统
│   ├── bin/                    # mini-app / mini-app-window / mini-launcher / mini-devserver
│   ├── js/                     # QuickJS runtime / api（App/Page/Component）/ bridge
│   ├── parser/                 # wxml / wxss（选择器引擎）/ expr / template
│   ├── renderer/               # wxml_renderer / components/*（含 style_parse、canvas、video）
│   ├── ui/                     # interaction / scroll_controller / scroll_cache
│   ├── runtime/                # MiniApp 应用运行时
│   └── tests/                  # 单元/集成测试
├── examples/                   # gallery / video_player / video_decode / demo
├── scripts/                    # 各平台 SDK 构建脚本 + gen_assets.py 素材生成
├── assets/                     # 字体资源
├── include/mini_render.h       # C 头文件
├── doc/                        # 文档、画廊图片、视频素材
└── sample-app/                 # 示例小程序
```

---

## 📄 License

MIT

[Taffy]: https://github.com/DioxusLabs/taffy
