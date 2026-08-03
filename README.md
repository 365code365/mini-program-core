**English** | [简体中文](README.zh-CN.md)

# mini-render

**A self-rendering mini-program engine, written from scratch in Rust — no WebView, no mapping to native platform widgets, no Skia.**

Hand it a WeChat mini-program source tree (WXML / WXSS / JS / `app.json`) and a writable pixel
buffer, and it draws the UI and runs the interactions. The alignment target is WeChat's
**Skyline** renderer: native rendering, no DOM.

The four screens below were rendered by the engine itself (plain WXML + WXSS + data, 375×667 @2x,
no browser involved):

| | | | |
|:---:|:---:|:---:|:---:|
| <img src="doc/gallery/28_ecommerce_home.png" width="180"/><br/>E-commerce home | <img src="doc/gallery/36_news_feed.png" width="180"/><br/>News feed | <img src="doc/gallery/42_food_order.png" width="180"/><br/>Food ordering | <img src="doc/gallery/60_form_controls.png" width="180"/><br/>Form controls |

All 65 are in [`doc/场景画廊.md`](doc/场景画廊.md) — they double as byte-reproducible regression
baselines.

Landing page: **<https://365code365.github.io/mini-program-core/>** (English by default;
the header has an EN / 中文 switch, or link straight to
[`?lang=zh`](https://365code365.github.io/mini-program-core/?lang=zh)).

> Documentation under `doc/` is written in Chinese. This README is the English entry point;
> [README.zh-CN.md](README.zh-CN.md) is the Chinese original and the two are kept in sync.

---

## Contents

- [Core overview](#core-overview)
  - [What it is, and what it is not](#what-it-is-and-what-it-is-not)
  - [How one frame gets drawn](#how-one-frame-gets-drawn)
  - [Eight design decisions that matter](#eight-design-decisions-that-matter)
  - [Where it stands today (measured)](#where-it-stands-today-measured)
  - [Scope: what it does, and what it deliberately will not do](#scope-what-it-does-and-what-it-deliberately-will-not-do)
  - [Good fit / poor fit](#good-fit--poor-fit)
  - [Stack and dependencies](#stack-and-dependencies)
- [Run it in 30 seconds (desktop)](#run-it-in-30-seconds-desktop)
- [Embedding it in an app (SDK)](#embedding-it-in-an-app-sdk)
- [Prebuilt mobile packages](#prebuilt-mobile-packages)
- [Feature coverage](#feature-coverage)
- [Testing and regression](#testing-and-regression)
- [Repository layout](#repository-layout)
- [Documentation](#documentation)

---

## Core overview

### What it is, and what it is not

In one line: **mini-program source in, pixels out**. Nothing in between — no browser engine, no
platform widgets, no third-party graphics library.

```
input                       engine (pure Rust library)                output
────────────────────────────────────────────────────────────────────────────
my-mini-app/                                                    ┌──────────┐
├── app.json      ─┐                                            │  RGBA    │
├── app.wxss       │   parse → logic layer → template eval       │  pixel   │
├── pages/*.wxml   ├─▶  → style resolution → flexbox layout ───▶ │  buffer  │
├── pages/*.wxss   │      → in-house 2D rasterizer               └──────────┘
├── pages/*.js    ─┘      → layer composition (page/tabBar/fixed)      │
└── images, assets                                                     ▼
                                                       window / UIView / SurfaceView / PNG
```

The artifact is an ordinary Rust library (`cdylib` / `staticlib` / `rlib`) and it asks for exactly
one thing: **a writable pixel buffer**. The same code therefore runs on Android, iOS, Windows,
macOS and Linux, and it can emit PNGs in a headless CI with no screen at all.

How it differs from the usual approaches:

| Approach | How it works | This engine |
|---|---|---|
| WeChat's WebView render layer | WXML → DOM, handed to a browser engine for layout and paint | ❌ No DOM, no browser engine |
| React Native / Weex | JS description → mapped onto platform widgets (`UIView` / `android.view`) | ❌ No widget mapping (not at the mercy of per-platform widget differences) |
| Flutter | Dart + Skia, self-rendered | ✅ Closest in spirit, but this is **Rust + an in-house rasterizer**, no Skia dependency |
| Electron / Tauri | Ship or reuse a browser | ❌ No bundled browser (artifact is ~34 MB, not ~100 MB) |
| WeChat **Skyline** | Native rendering with self-drawn components | ✅ The alignment target: semantics, feel and event model all follow it |

**Why self-rendering is worth it.** The pain with WebView-based stacks is not "can it render" but
that **you do not control it**: the engine version follows the OS, the same CSS lays out
differently across Android builds, first paint waits for the engine to boot, scrolling and gesture
feel are decided by the engine, and every native capability needs a bridge. Self-rendering turns
all of that into your own code: **one copy of the layout and paint logic for every platform**, with
frame scheduling, gesture arbitration and memory ceilings in your hands. The only remaining
cross-platform difference comes from system fonts. To remove even that, bundle a font
(`assets/SourceHanSansSC-Regular.otf` + `TextRenderer::from_file`) — but read
[the memory section](#scope-what-it-does-and-what-it-deliberately-will-not-do) first: a bundled
font costs **more** memory, not less. The price is real too: CSS coverage has to be filled in one
property at a time (see [Scope](#scope-what-it-does-and-what-it-deliberately-will-not-do)), and
performance is spent CPU cycle by CPU cycle.

### How one frame gets drawn

Eight stages, each with its own diagnostic switch (`MINI_*`, listed in the
[architecture doc](doc/架构与实现原理.md#帧成本是怎么压下来的)):

| # | Stage | What happens | Key implementation |
|---|---|---|---|
| ① | **Parse** | WXML → node tree; WXSS → stylesheet | Hand-written parsers; a CSS selector engine (tag / class / `#id` / `*` / attribute / descendant / child, with specificity ordering) — selectors are **compiled at parse time** |
| ② | **Logic layer** | Runs `App` / `Page` / `Component`; `setData` produces a data snapshot | QuickJS (rquickjs); CommonJS modules, `Promise`/`async`, timers, microtasks pumped every frame |
| ③ | **Template eval** | `{{ }}` expressions plus `wx:if` / `wx:for` expansion → render tree | In-house expression engine; `class`/`style` bound to arrays or objects are joined per CSS semantics |
| ④ | **Style resolution** | Matched CSS + inline `style` → computed style | Inheritance semantics (`color` / `font-size` / `font-weight` / `line-height` / `letter-spacing` …); shorthands and longhands land in a fixed precedence order |
| ⑤ | **Layout** | Flexbox box-model solve | taffy 0.12; **text nodes carry a custom measure function**, so wrapping and min/max-content come from real glyph widths; a second correction pass (`reflow`) cleans up overflow that is only knowable after layout |
| ⑥ | **Paint** | Layer by layer into an RGBA canvas | In-house 2D rasterizer: scanline + even-odd fill, 4× supersampled anti-aliasing, rounded corners via cubic Bézier (K = 0.5523), fontdue glyphs with multi-font fallback, Apple `sbix` color emoji, two-tier image filtering (bilinear + area), linear/radial gradients, masked `box-shadow`, clip stack |
| ⑥' | **Animation** | `@keyframes` / `transition` / `wx.createAnimation` | Evaluated **at paint time** against a global clock: `transform` / `opacity` / colors affect painting only and never trigger reflow → an animated frame costs about the same as a static one |
| ⑦ | **Composition** | Normal flow → tabBar → `position: fixed` overlay | Stacking order matches the browser (a full-screen scrim dims the tabBar too); the overlay uses viewport coordinates, its own canvas, and repaints on demand |
| ⑧ | **Present** | Blit to a window / hand to the host / save a PNG | Desktop uses softbuffer; mobile hands the RGBA to a host View; headless calls `save_png` directly |

Between the logic layer and the render layer there is exactly one channel — the **data snapshot**
(`setData` → `Arc<Value>`). No DOM, no virtual-node diffing, no GC pressure.

### Eight design decisions that matter

These are the "why is it written this way" answers. Every mistake behind them is written up in
[`doc/踩坑记录.md`](doc/踩坑记录.md).

1. **Text measurement drives layout; widths are never estimated.** A `TextMeasure` context is
   attached to taffy leaves so real glyphs participate in layout. That is what lets two CSS
   behaviours hold at once: "wrap at the container width inside a fixed-width box" and "a shrinking
   container gets pushed out by its content". Counter-example: a `+4px` "safety margin" per text box
   once made every content-sized badge, tag and pill a ring wider than the browser's.
2. **Animation repaints, never reflows.** Changing `width`/`height` in `@keyframes` has no effect;
   `transform` / `opacity` / colors do. What you buy is "an animated frame is as cheap as a static
   one".
3. **`setData` repaints only what changed.** The old and new render trees are walked in parallel to
   collect bounding boxes of nodes whose text/attributes or geometry changed, and those become the
   clip rects. The invalidation for the home page's per-second countdown is **45×33 pixels**, not
   the whole screen. When in doubt it falls back to a full frame — leaving stale pixels is far worse
   than painting once too often.
4. **Page scrolling does not repaint.** The page canvas is full-page height in content
   coordinates, so scrolling just presents a different slice (2–3 ms per frame). Only the moment you
   scroll past the already-painted band does it paint again.
5. **Scroll feel follows iOS/WeChat.** Rubber-band damping `1 - 1/(x·0.55/d + 1)`, and bounce-back
   is a **critically damped spring with initial velocity** (not a fixed-duration easing); inertia
   hitting a boundary does not stop dead, it hands its momentum to the spring.
6. **Gesture ownership is not decided on touch-down.** It waits for the first meaningful movement
   (4 px), locks to the dominant axis, then hands off to the outer scroller on "still pushing at the
   boundary". Swiping vertically inside a horizontal card list scrolls the page; swiping
   horizontally inside a vertical list moves nothing.
7. **The host layer lives in the lib and is shared by desktop and mobile.** Page stack, overlays,
   touch state machine, gesture arbitration, picker sheet and pixel composition all live in
   `src/host/`; only frame scheduling is per-platform. The two paths are guarded by
   `tools/sdk-parity.sh` (per-pixel) and `src/tests/engine_input_tests.rs` (input behaviour).
8. **No GPU, no per-element bitmap cache.** The premise is "give me a pixel buffer and I will
   draw"; the complexity of bitmap caching against animation and the clip stack is not worth it.
   Anti-aliasing was not switched to subsampling either — measured against Chrome the difference
   went *up*, from 4.7% to 5.3%.

### Where it stands today (measured)

**Performance** (375×667 @2x, i.e. 750×1334 physical pixels, written pixel by pixel on the CPU):

| Scenario | Before | Now |
|---|---|---|
| Shop home, full frame (carousel + per-second countdown + skeleton animation) | 49.6 ms | **6.1 ms** |
| Shop home, steady-state frame interval (144 Hz screen, 6.94 ms budget) | 6.9–25 ms jitter | **6.7–7.2 ms** |
| Home page slowest frame / peak frame interval | 17.9 / 24.9 ms | **11.4 / 11.4 ms** |
| Cost of rebuilding after one `setData` | 32 ms | **4 ms** |
| Drag frame (tea-app home, 60-frame average) | 7.17 ms | **3.74 ms** |
| Remote image, second open (disk cache) | 6132 ms | **11 ms** |
| First frame "build tree + style" (app with three Chinese font stacks) | 3093 ms | **955 ms** |

**Cross-renderer consistency** (same source: native rendering on one side, compiled to HTML and
screenshotted in Chrome on the other, compared per pixel):

| Sample | Pages | Changed-pixel ratio vs Chrome |
|---|---|---|
| `sample-app` (shop) | 15 | **4.55%** |
| `news-app` (news) | 6 | **6.33%** |

What remains is concentrated in bold glyph shapes and subpixel text positioning. That number is a
**regression criterion, not a marketing line** — every rendering change is checked against it first.

**Real projects that actually run** (not hand-made demo scaffolding):

| Mini-program | Size | Notes |
|---|---|---|
| `sample/tea-app` | 36 pages | **Output of uni-app compiled to mp-weixin**: a 259 KB Vue 3 runtime plus 60 CommonJS modules, pages defined via the `Component()` constructor |
| `sample/real-sample` | 8 pages | WeChat's official demo |
| `sample/sample-app` | 15 pages | Full shop flow: cart → order confirmation → checkout → orders/shipping, with cross-page state in `wx.storage` |
| `sample/news-app` | 6 pages | Full news flow: horizontal channel swiping, live font-size changes, comments, persisted bookmarks |

**Regression suite** (all green on every change; commands in
[Testing and regression](#testing-and-regression)):

| Criterion | Size |
|---|---|
| Unit tests | lib **483** + bin **9**; including 41 pure host-logic cases (gesture arbitration 13 / edge-swipe back 12 / picker sheet 10 / touch state machine 6) and 7 SDK pointer-path cases |
| Scene gallery | **65** images, byte-reproducible under a fixed animation clock |
| Partial-repaint check | Incremental repaint vs forced full frame, **byte-identical** across 5 scenes |
| SDK vs desktop | sample 15/15, news 6/6 **pixel-identical** |
| Pointer and overlay layers | 6 deterministic interaction snapshots + 2 gesture assertions + a **no-op guard** |
| Compiler warnings | **0** (`cargo check --all-targets`) |

### Scope: what it does, and what it deliberately will not do

**Implemented** (full table in [Feature coverage](#feature-coverage)):

- **24 component tags** (containers / text and media / form controls, including `swiper`,
  `scroll-view`, `picker-view`);
- **Events**: six binding prefixes (`bind` / `catch` / `capture-bind` / `capture-catch` /
  `mut-bind` / `bind:`) with a complete capture-bubble chain; touch sequences follow WeChat
  semantics (slop cancels tap, 350 ms longpress, a `touchcancel` is synthesized when scrolling
  takes over);
- **Templates**: `wx:if` / `wx:elif` / `wx:else` / `wx:for`, `block`, the expression engine,
  `class`/`style` bindings, and `usingComponents` from page JSON (custom components with style
  isolation and their own data scope);
- **Styles**: selectors with specificity cascade, inheritance, `@import`, CSS variables, `rpx`,
  `calc()`, gradients, shadows, `transform` / `transition` / `@keyframes`;
- **APIs**: 39 `wx.*` entries (`request` with `abort`, the full storage set persisted across
  launches, Toast/Loading/Modal, five routing APIs, the full device-info set, `createAnimation`,
  `createCanvasContext`);
- **Lifecycles**: 28 hooks across three levels (App 7 / Page 13 / Component 5 + `pageLifetimes` 3);
  page `onShow`/`onHide`/`onResize` cascade to the components inside the page;
- **Interaction**: inertial scrolling with critically damped bounce, gesture arbitration, edge-swipe
  back, pull-to-refresh, the picker bottom sheet, partial repaint;
- **Media**: Canvas 2D (a complete 2D context plus a device-resolution backing buffer), `<video>`
  (MP4 demux + H.264 software decode + audio).

**Deliberately out of scope** (a decision, not a backlog item):

| Not doing | Why |
|---|---|
| GPU rendering | The premise is "a pixel buffer is enough"; only pure CPU can produce images where there is no GPU (CI, embedded) |
| Parsing `.wxapkg` / signature checks | Subpackages, staged rollout and integrity belong to the host; the engine reads a **directory** |
| Compiling `<web-view>` / `<map>` into the engine | In WeChat these are native-layer components. The engine computes position and parameters; the host places the widget with `WKWebView` / `android.webkit.WebView` / ArkUI `Web()` ([rationale](doc/原生App集成指南.md)) |
| Remote fonts via `wx.loadFontFace` | Fonts come from the system. The API exists and calls back `success` (otherwise it would stall the framework's `onLaunch`), but nothing is downloaded |
| Accommodating code that deviates from WeChat semantics | Skyline is the only alignment target. Where usage disagrees with WeChat, WeChat wins |

**Still missing** (13 items, each with its impact, in section 6 of
[`doc/引擎测试说明.md`](doc/引擎测试说明.md)): `wx:key` reuse semantics, `template` / `slot` /
`wxs`, `scroll-view` scroll events, `<video>` playback events, cloud APIs such as `wx.login`, and
edge-swipe back in the mobile SDK.

**The biggest known weakness is memory**: a 600 MB–1 GB peak, almost entirely fonts. Measured
breakdown from `cargo run --release --example mem_report` (RSS delta, macOS):

| Stage | Delta |
|---|---|
| System primary font (Hiragino Sans GB, ~29k glyphs, regular face) | **+303 MB** |
| Bold face of the same collection (lazy, on the first bold glyph) | +8–150 MB (depends on the face sizes in the collection) |
| One more font collection (`Songti.ttc`, 63.8 MB, used by apps that ask for a Song face) | **+138–156 MB** |
| Bundled `SourceHanSansSC-Regular.otf` (~65k glyphs, single face) | **+405 MB** |
| Drawing 1200 distinct Chinese characters (glyph **bitmap** cache) | +12–31 MB |
| One 750×4000 page canvas | ≈0 |

Two conclusions worth remembering: **the cost scales with glyph count, not file size** (the 16.5 MB
Source Han Sans is more expensive than the system font because it has over twice the glyphs); and
**the glyph bitmap cache is not the problem** (tens of MB, already byte-capped). So "bundling a
single-face font saves memory" is false — measured, it costs more. Bundle a font for
**cross-platform identity**, not for memory.

The root cause is that fontdue expands the geometry of every glyph in the font at load time. All
caches are byte-capped (images 64 MB / glyphs 24 MB / two font files, LRU) and `trim_memory()` is
there for hosts to call on a memory warning, but the only real fix for the peak is a lazy font
backend (`swash` / `cosmic-text`). **The code surface of that swap is small** (every fontdue call
is in `src/text.rs`: five `Font::from_bytes`, four `metrics`, two `rasterize`); the hard part is
that it changes the anti-aliasing of every glyph, so 65 gallery images + 21 full-page frames + 7
interaction snapshots all need new baselines. That makes it its own round of work, not a refactor.

### Good fit / poor fit

**Good fit**

- You want to embed a mini-program container in your own app without inheriting WebView version
  drift and first-paint cost;
- You need rendering that is **controllable and reproducible**: layout and rasterization are one
  codebase on every platform, and with the same fonts the output is pixel-identical (cross-OS
  differences come only from system fonts; a bundled font removes them at a memory cost, see above);
- You need batch image generation with no screen: marketing long-images, server-rendered cards, UI
  regression baselines;
- You want one mini-program source tree to also ship as H5 (the built-in compiler; this repo uses
  it as the reference implementation for cross-checking);
- You want the rendering details in your own hands: frame scheduling, gesture feel, memory
  ceilings, diagnostic logs.

**Poor fit**

- You need the full web platform (arbitrary CSS, third-party H5 SDKs, `<iframe>`) — use a WebView;
- Heavy 3D or lots of filters — CPU rasterization will not hold up; use a GPU stack;
- You expect "any production mini-program runs with zero adaptation" — there are still 13 known
  gaps, and complex apps need item-by-item verification;
- Severely memory-constrained devices — the peak is high until the font backend is swapped.

### Stack and dependencies

Dependencies track current stable releases (`winit` excepted: 0.31 is beta-only, so the stable line
is still 0.30).

| Dependency | Version | Purpose |
|---|---|---|
| [taffy](https://github.com/DioxusLabs/taffy) | 0.12 | Flexbox / block layout solving (this is what makes `box-sizing` actually work) |
| [rquickjs](https://github.com/DelSkayn/rquickjs) | 0.12 | QuickJS bindings: the JS runtime for the logic layer |
| [fontdue](https://github.com/mooman219/fontdue) | 0.9 | Glyph rasterization |
| [image](https://github.com/image-rs/image) | 0.25 | Image decoding (PNG / JPEG / GIF frames / WebP …, the crate's default decoder set) |
| [ureq](https://github.com/algesten/ureq) | 3.3 | `wx.request` and remote images (must use `http_status_as_error(false)` — in WeChat 4xx/5xx still go to `success`) |
| [symphonia](https://github.com/pdeljanov/Symphonia) | 0.6 | Audio demux and decode (AAC / MP3 / MP4) |
| [openh264](https://github.com/ralfbiedert/openh264-rs) | 0.9 | H.264 software decode (optional feature `h264`) |
| [winit](https://github.com/rust-windowing/winit) + [softbuffer](https://github.com/rust-windowing/softbuffer) | 0.30 / 0.4 | Desktop window and software framebuffer (optional feature `desktop`) |
| [rodio](https://github.com/RustAudio/rodio) | 0.22 | Desktop audio playback (optional feature `audio`) |
| [jni](https://github.com/jni-rs/jni-rs) | 0.21 | Android JNI entry (compiled only for android targets) |

Project size: **240 `.rs` files / ~53k lines** (29 of them test files). Any single file over 500
lines gets split (exceptions must state the reason in the file header). Outside the FFI boundary
(`extern "C"`) the whole library has **no `unsafe`** and no `static mut`: global state always goes
through `RwLock` / `OnceLock` / `thread_local`.

---

## Run it in 30 seconds (desktop)

```bash
# needs Rust (https://rustup.rs)
cargo build --release

# pick a sample mini-program interactively
./target/release/mini-launcher

# or name one directly
./target/release/mini-app-window sample-app
./target/release/mini-app-window sample/tea-app --route pages/index/index
```

Headless image output (for CI / regression):

```bash
./target/release/mini-app-window sample-app --route pages/index/index \
    --settle 0 --time 2 --snapshot target/shot
```

Common action flags (executed in the order written, repeatable): `--touch x,y` /
`--swipe x1,y1,x2,y2` / `--drag px×frames` / `--type text` / `--key enter` / `--wheel` /
`--wait seconds` / `--frames N`. Full list via `--help`. Diagnostic switches
(`MINI_SCROLL_LOG` / `MINI_FPS` / `MINI_FONT_LOG` …) are documented in
[`doc/引擎测试说明.md`](doc/引擎测试说明.md) and
[`doc/架构与实现原理.md`](doc/架构与实现原理.md#帧成本是怎么压下来的).

The list of sample mini-programs, live browser preview (`mini-devserver`), compiling to an HTML
project, and Rust / C examples of using the engine as a library are in
[`doc/示例与调试.md`](doc/示例与调试.md).

---

## Embedding it in an app (SDK)

Integrators only deal with **one View**. Details in [`sdk/README.md`](sdk/README.md) and
[`doc/原生App集成指南.md`](doc/原生App集成指南.md).

### 1. Build the libraries

```bash
bash tools/build-mobile.sh android   # → sdk/android/src/main/jniLibs/<abi>/libmini_render.so
bash tools/build-mobile.sh ios       # → sdk/ios/MiniRender.xcframework (device + simulator)
bash tools/build-mobile.sh check     # cross-compile check only
```

The script already carries the three **mandatory** arguments (miss any one and it will not build;
the reasons are in the script's comments):

| Argument | Why |
|---|---|
| `--no-default-features` | Turns off desktop dependencies: `winit` requires an activity backend feature on android targets or compilation fails; `arboard` (clipboard) is desktop-only; `rodio` wants a system audio device |
| `--features bindgen` | `rquickjs-sys` ships no pregenerated C bindings for iOS/Android, so they must be generated on the spot |
| `IPHONEOS_DEPLOYMENT_TARGET` | rustc links against iOS 10 by default while the C dependencies are built against a newer SDK; the mismatch shows up as a missing `___chkstk_darwin` |

Feature flags:

| Feature | Default | Notes |
|---|---|---|
| `desktop` | ✅ | Desktop window (winit + softbuffer + clipboard) |
| `audio` | ✅ | Built-in audio playback (sound for `<video>`) |
| `h264` | ✅ | Built-in H.264 software decode. **Turn it off on mobile**: the system player's hardware decode is cheaper on battery, and openh264 does not build for the iOS simulator target |
| `bindgen` | ❌ | Turn on when cross-compiling |

### 2. Package it as a dependency (recommended)

**Android (AAR → Maven)**

```bash
bash tools/build-mobile.sh android
cd sdk/android && ./gradlew publishToMavenLocal
```

In your app project:

```kotlin
repositories { mavenLocal() }                       // or your internal Maven
implementation("dev.minirender:mini-render:0.1.0")
```

**iOS (Swift Package / CocoaPods)**

```swift
// Package.swift
.package(path: "../mini-program-core/sdk/ios")      // or .package(url: ..., from: "0.1.0")
```

```ruby
# Podfile
pod 'MiniRender', :path => '../mini-program-core/sdk/ios'
```

### 3. Integration code

Android:

```kotlin
val mini = MiniProgramView(this)
setContentView(mini)
mini.open(File(filesDir, "my-mini-app"))          // the unpacked mini-program directory

override fun onBackPressed() {
    if (!mini.goBack()) super.onBackPressed()      // exit only once the page stack is empty
}
override fun onResume() { super.onResume(); mini.onHostResume() }
override fun onPause()  { super.onPause();  mini.onHostPause() }
```

iOS:

```swift
let mini = MiniProgramView(frame: view.bounds)
view.addSubview(mini)
mini.open(appDir: unpackedDir)

if !mini.goBack() { navigationController?.popViewController(animated: true) }
```

Both Views already handle: a dedicated render thread (the engine is not thread-safe), size and dpr
conversion, the three touch phases (including historical points / `coalescedTouches`, nothing
dropped), on-demand frames (idle costs no battery), the sandbox directory, lifecycles and memory
warnings.

### 4. Only two things are yours to decide

1. **Where the mini-program package comes from**: download / verify / unpack into the sandbox. The
   engine reads a **directory**; it does not parse `.wxapkg` and does no signature checking —
   subpackages, staged rollout and integrity belong to the host.
2. **Who hosts native-layer components**: in WeChat, `<web-view>` / `<video>` / `<map>` live in the
   native component layer (topmost; only `cover-view` can cover them). The engine computes position
   and parameters, and you place the widget with `WKWebView` / `android.webkit.WebView` /
   ArkUI `Web()`. **Compiling wry into the engine is not recommended** (no HarmonyOS backend, and
   Android would require changing the host Activity); the rationale is in the integration guide.

### Artifact size (measured, arm64)

| | |
|---|---|
| Android `.so` (stripped) | 34.4 MB |
| iOS static library `.a` | 184 MB (shrinks a lot through dead-code elimination once linked into an app) |

That is on the large side, for two known reasons that are not optimized yet: `page_loader.rs` uses
`include_str!` to bundle a sample mini-program as a fallback (a production SDK should drop it), and
the symphonia / openh264 decoders are linked in whole (mobile builds can disable `h264`).

---

## Prebuilt mobile packages

Built from this repository with `tools/build-mobile.sh`. Every release attaches the mobile
artifacts, so you do not need the Android NDK or Xcode to integrate:

| Asset | Contents | Consumed by |
|---|---|---|
| `mini-render-android-<version>.zip` | `libmini_render.so` for `arm64-v8a`, `armeabi-v7a`, `x86_64` (stripped) plus the Kotlin `MiniProgramView` / `MiniEngine` sources and the Gradle module | Drop into `src/main/jniLibs/`, or build the AAR with `./gradlew publishToMavenLocal` |
| `mini-render-ios-<version>.zip` | `MiniRender.xcframework` (device arm64 + simulator arm64), the C headers with `module.modulemap`, `Package.swift`, `MiniRender.podspec` and the Swift `MiniProgramView` | Swift Package (`.package(url:)`) or CocoaPods (`pod 'MiniRender'`) |

Releases: <https://github.com/365code365/mini-program-core/releases>

Each asset ships a `SHA256SUMS` next to it; verify before use:

```bash
shasum -a 256 -c SHA256SUMS
```

Prefer building them yourself (you pick the feature flags):

```bash
bash tools/build-mobile.sh android   # → sdk/android/src/main/jniLibs/<abi>/libmini_render.so
bash tools/build-mobile.sh ios       # → sdk/ios/MiniRender.xcframework
bash tools/package-mobile.sh         # → dist/mini-render-{android,ios}-<version>.zip + SHA256SUMS
```

Releases are produced by CI: pushing a `v*` tag runs
[`.github/workflows/release-mobile.yml`](.github/workflows/release-mobile.yml), which builds
Android on an ubuntu runner and iOS on a macOS runner, then attaches both zips and a `SHA256SUMS`
to the release. Note the zips are not byte-reproducible (zip records timestamps), which is why every
build publishes its own checksum file.

`h264` is **off** in these builds: mobile should hand playback to the system player (hardware
decode, better battery, respects audio focus), and openh264 does not build for the iOS simulator
target. `<video>` still takes part in layout and hit testing, it just produces no picture frames.

---

## Feature coverage

| Category | Supported |
|---|---|
| Components (24 tags) | Containers: view / block / scroll-view / swiper / swiper-item<br/>Text and media: text / rich-text / icon / image / video / canvas<br/>Forms: button / input / textarea / switch / slider / progress / checkbox / checkbox-group / radio / radio-group / picker / picker-view / picker-view-column |
| Events | touchstart/move/end/cancel, longpress/longtap, tap, change, focus/input/blur/confirm, image load/error, page onPageScroll / onReachBottom / onPullDownRefresh; six binding prefixes (bind / catch / capture-bind / capture-catch / mut-bind / bind:) with the full capture-bubble chain |
| WXML | wx:for / wx:if / elif / else / for-item / for-index, block, expression engine, class and style bindings, custom components (usingComponents) |
| WXSS | tag / class / id / attribute / pseudo-class selectors, descendant and child combinators, specificity cascade, `:first/last/only-child`, `:nth-child(An+B)`, `:active`, `@import`, `@keyframes`, CSS variables, rpx, gradients, shadows, transform, transition/animation |
| wx.\* | request (with abort), the full storage set, showToast/showLoading/showModal, pull-to-refresh, createAnimation, createCanvasContext, the full device-info set, five routing APIs, app-level listeners (onError etc.) |
| Interaction | Inertial scrolling with critically damped bounce, gesture arbitration (axis lock / inner-outer handoff), edge-swipe back, pull-to-refresh, picker sheet, partial repaint (damage regions) |

**Icons** use WeUI's official vector data (`success` is a filled circle with a check-shaped hole
punched out, not a white-stroked check).

Known gaps (`wx:key` / `template` / `slot` / `wxs`, `scroll-view` scroll events, `video` playback
events, unimplemented APIs such as `wx.login` …) are listed in
[section 6 of `doc/引擎测试说明.md`](doc/引擎测试说明.md) — 13 items, each with its impact.

---

## Testing and regression

```bash
rm -rf target/mini-storage                 # ① mandatory, see below
cargo test --release                       # ② lib 483 + bin 9
bash tools/damage-check.sh                 # ③ incremental repaint == full frame (byte-identical)
cargo run --release --example gallery      # ④ 65 scene images
bash tools/tab-click-check.sh              # ⑤ tabBar clicks across three apps
bash tools/sdk-parity.sh                   # ⑥ mobile SDK vs desktop window, pixel-identical
bash tools/interaction-check.sh target/_ia # ⑦ pointer and overlay layers (picker/Modal/press/gesture)
bash tools/clean-target.sh                 # ⑧ cleanup (mandatory, see below)
```

For changes under `site/` (the GitHub Pages landing page):

```bash
python3 tools/site-i18n-check.py    # EN/ZH keys aligned, no leftover Chinese, same structure
python3 tools/site-render-check.py  # real headless Chrome: English by default, ?lang=zh switches
```

The page keeps **one copy of the markup**: English lives in the HTML, Chinese in
`site/assets/i18n.js`. The first check also runs in the Pages workflow, because a missing key
leaves that one paragraph silently in English and the page still loads fine.

For rendering changes, add per-page snapshots and a comparison against the HTML reference:

```bash
bash tools/snapshot-all.sh sample-app target/fin2_sample --settle 0 --time 2
./target/release/examples/compare --all --rust-from target/fin2_sample --out target/fin2_cmp
python3 tools/pixdiff.py a.png b.png       # changed ratio + bounding box
```

Current baselines: sample-app **4.55%**, news-app **6.33%** (changed-pixel ratio against the
compiled HTML).

### Two hard rules

**Clean `target/` when you are done.** Every screenshot and comparison tool writes into
`target/<a name it picked>`; one regression round leaves dozens of directories, and it has been
measured at **26 GB**.

```bash
bash tools/clean-target.sh            # test output + target/debug (keeps release, no rebuild)
bash tools/clean-target.sh --targets  # also cross-compile target dirs (iOS/Android, ~6 GB)
bash tools/clean-target.sh --all      # plus the cargo cache
bash tools/clean-target.sh --dry      # show what would be deleted first
```

Baseline directories (`target/fin2_*`) go too — one `snapshot-all.sh` regenerates them.

**Run `rm -rf target/mini-storage` before taking snapshots.** Headless clicks persist data into
`target/mini-storage/<app>.json`; the next launch reads it back and the page content changes, which
once made the cart page look like it regressed from 1.51% to 10.05%. Also, `sample-app` baselines
must use `--settle 0` (the home page has a delayed floating layer, and `--settle ≥ 0.5` produces a
93.9% false difference).

Details in [`.kiro/steering/30-test-and-cleanup.md`](.kiro/steering/30-test-and-cleanup.md).

---

## Repository layout

```
src/
├── parser/          WXML / template and expression engine
│   ├── expr/        expressions in 5 pieces: lexer / recursive-descent parser / eval / JS value semantics
│   └── wxss/        WXSS: `parser` (text→rules) + `selector` (selector engine) + stylesheet queries
├── renderer/        renderer
│   ├── components/  tree building and painting for 24 tags (incl. WeUI icon data, SVG path parsing)
│   │   ├── registry.rs  **the single registration point** for tag → component behaviour (build/draw/leaf/attribution/state/clip)
│   │   ├── base/      shared base: types / text metrics / wrapping / event attrs / style application / box painting
│   │   ├── canvas/    canvas 2D context in 7 pieces: state / shapes / paths / text / gradients / context manager
│   │   └── video/     video in 5 pieces: MP4 container / H.264 decode / audio track / playback state machine
│   └── wxml_renderer/  layout cache / paint scheduling / hit testing / animation / partial-repaint invalidation
├── host/            **host layer (platform-agnostic)**: page stack, overlays, touch state machine, pixel composition
│   ├── picker_sheet/  bottom picker sheet in 5 pieces: state and geometry / build / paint / region cascade / pointer loop
│   └── engine.rs      MiniEngine — the core of the mobile SDK
├── js/              QuickJS bindings; `prelude/*.js` is the logic-layer prelude split by domain
│                    (module / console / storage / ui / route / device / network …)
├── runtime/         MiniApp (logic-layer driver, timers, bridge events)
├── ui/              interaction manager (`interaction/` in 6 pieces: element table / hit test / input editing / pointer / animation), scroll controller
├── compiler/html/   compile to an HTML project (the reference implementation for cross-checking)
│                    `transpile/` in 4 pieces (CSS / tags / component internals / emit),
│                    `runtime/*.js` is the browser-side reactive runtime (9 pieces, include_str! in order)
├── ffi_app.rs       app-level C ABI (mr_app_*)
├── ffi_jni.rs       Android JNI entry
└── bin/             desktop host
    ├── window.rs      now just struct + new() + main() (used to be 1877 lines doing four jobs)
    └── app_window/    split by responsibility: winit_app (winit glue) / frame (frame gate) /
                       frame_render (compose and present) / page_host (page stack routing) /
                       scroll_host / tabbar_host / overlay_host / headless_run

sdk/                 Android (AAR) / iOS (XCFramework + SPM + CocoaPods)
tools/               build and regression scripts
examples/            gallery, cross-renderer comparison, memory/font probes, SDK parity
sample/              sample mini-programs (sample-app / news-app / tea-app …)
doc/                 documentation and scene images
```

**The desktop host and the mobile SDK share `src/host/`**; only frame scheduling differs.
`tools/sdk-parity.sh` guards that boundary per pixel (currently sample 15/15, news 6/6 identical).

Project convention: any `.rs` over 500 lines gets split (exceptions state the reason in the file
header), see [`.kiro/steering/`](.kiro/steering/). Module responsibilities and the renderer's
internal division of labour are in
[`doc/架构与实现原理.md`](doc/架构与实现原理.md#分层结构).

---

## Documentation

All documents under `doc/` are in Chinese.

| Document | Contents |
|---|---|
| [`doc/架构与实现原理.md`](doc/架构与实现原理.md) | How a frame is drawn, layering and module split, how frame cost was driven down, partial repaint, scroll feel and touch alignment, dependency versions and taffy adaptation |
| [`doc/示例与调试.md`](doc/示例与调试.md) | Sample mini-programs, each `examples/` target, browser preview, compiling to HTML, video/Canvas, Rust/C/mobile code samples |
| [`doc/场景画廊.md`](doc/场景画廊.md) | 65 real renders (also the regression baselines) and how the assets are generated |
| [`doc/踩坑记录.md`](doc/踩坑记录.md) | 13 classes of "what was once wrong": symptom → root cause → current approach → regression criterion |
| [`doc/引擎测试说明.md`](doc/引擎测试说明.md) | Capability matrix, T1–T9 test checklists and criteria, measured memory breakdown, 13 known gaps |
| [`doc/原生App集成指南.md`](doc/原生App集成指南.md) | Minimal integration, the host's nine jobs, C ABI, iOS/Android/HarmonyOS steps, acceptance checklist |
| [`doc/触控对齐测试报告.md`](doc/触控对齐测试报告.md) | Item-by-item measurements of swipe/tap/longpress/gesture against WeChat |
| [`doc/实现组件说明.md`](doc/实现组件说明.md) | Per-component implementation status |
| [`sdk/README.md`](sdk/README.md) | Three-line SDK integration and the three conventions you must know |

## Biggest known TODOs

Three, in priority order:

1. **Memory peak** (600 MB–1 GB, 98% fonts) — needs a lazy font backend; details in
   [Scope](#scope-what-it-does-and-what-it-deliberately-will-not-do);
2. **`setData` still rebuilds the whole layout tree** (~11 ms for the home page's per-second frame,
   right at the edge of the 144 Hz budget) — needs incremental tree patching; the infrastructure
   (`FramePlan` / damage regions / `damage-check.sh`) is already in place;
3. **Edge-swipe back is not wired up in the mobile SDK** — it needs a viewport snapshot at the
   moment a page gets covered, which is host-side frame management and is best done together with
   partial repaint in the SDK.

## License

MIT. The bundled icon vector data comes from Tencent's official open-source WeUI (MIT).
