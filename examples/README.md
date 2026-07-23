# Examples 示例说明

本目录包含渲染示例与调试工具。所有场景的 WXML/WXSS/数据代码按主题拆分在
[`gallery_parts/`](./gallery_parts) 下，通过 `include!` 组合，既能一次性全量渲染
（`gallery`），也能按主题单独运行，方便快速定位/测试某一类界面。

> 运行前先安装 Rust 并 `source "$HOME/.cargo/env"`。渲染产物输出到仓库根的 `doc/gallery/`。
> 首次运行建议先执行 `python3 scripts/gen_assets.py` 生成真实图片/图标素材。

## 一键全量

```bash
cargo run --example gallery       # 渲染全部 37 张场景图到 doc/gallery/
```

## 按主题单独运行（一个示例一个文件）

| 示例 | 命令 | 包含场景 |
|------|------|----------|
| 基础页 | `cargo run --example basics` | 登录 / 商品列表 / 商品详情 / 购物车 / 个人中心 / 设置 |
| 社交 | `cargo run --example social` | 聊天 / 动态流 |
| 组件 | `cargo run --example widgets` | 宫格导航 / 表单 / 数据看板 / 图片画廊 |
| 天气 | `cargo run --example weather` | 4 个城市 × 不同天气（AI 城市背景） |
| 商城 | `cargo run --example storefront` | 订单 / 商城首页 / 通讯录 |
| 媒体 | `cargo run --example media` | 音乐播放器 / 标签徽章 |
| 弹窗 | `cargo run --example popups` | 确认弹窗 / 操作面板 / Toast |
| 滑动 | `cargo run --example scroll` | Swiper / 横向滚动 / 纵向长列表 / 底部选择器 / 日历 / 评分 |
| 电商大促 | `cargo run --example ecommerce` | 电商首页（秒杀/瀑布流）/ 优惠券弹窗 |
| 导航 | `cargo run --example navigation` | 底部 TabBar / 自定义搜索栏 / 输入事件 |
| Canvas | `cargo run --example canvas` | Canvas 2D 绘图（柱状图/圆环/路径/变换） |

## 视频与调试

```bash
cargo run --example video_player   # 独立窗口播放 doc/videos/video.mp4（自动循环 + 声音，按刷新率出帧）
cargo run --example video_decode   # 离屏解码抓帧，输出 doc/videos/frame_*.png 验证解码
cargo run --example demo           # 2D 渲染基础示例
```

浏览器可视化调试（点击画面即可操作 UI）：

```bash
cargo run --bin mini-devserver              # 默认加载 sample-app/pages/index
cargo run --bin mini-devserver <页面目录> [端口]
# 打开终端输出的 http://127.0.0.1:9000
```

窗口 App / 启动器：

```bash
cargo run --bin mini-app-window   # 窗口应用（加载 sample-app）
cargo run --bin mini-launcher     # 小程序启动器（扫描 sample 目录）
```

## 目录结构

```
examples/
├── gallery.rs            # 全量渲染入口（include! 组合所有 parts）
├── basics.rs …           # 各主题单独运行入口（薄封装，只调用对应场景）
├── video_player.rs       # 独立视频播放窗口
├── video_decode.rs       # 视频解码抓帧验证
├── demo.rs / demo.c      # 2D 渲染示例（Rust / C FFI）
└── gallery_parts/        # 场景实现（按主题拆分，非独立可执行，供 include!）
    ├── common.rs         # 共享：use / 常量 / render / render_screen
    ├── basic.rs social.rs widgets.rs weather.rs ecom_home.rs
    ├── media.rs overlay.rs scroll.rs promo.rs nav.rs canvas_video.rs
```

新增场景：在 `gallery_parts/<主题>.rs` 里加一个 `fn xxx() { ... render("id", ...); }`，
然后在 `gallery.rs` 的 `main()` 里调用它即可（如需单独运行，再加一个薄入口文件）。

## 单元 / 集成测试

```bash
cargo test              # 全部 203 个用例
cargo test route        # 路由 / 页面栈 / 组件 / 模块 / 异步 / 生命周期
cargo test canvas       # Canvas 2D 上下文与命令
cargo test scroll       # 滚动与惯性
cargo test --lib component_render   # 组件渲染（含文本换行、边框分割线等回归）
```

测试覆盖：表达式引擎、WXSS 选择器（`var()`/`calc()`）、模板控制流、布局与文本换行、
各边独立边框、全组件渲染、Canvas 2D、交互、滚动/惯性、页面栈路由、CommonJS 模块、
Promise、三级生命周期、事件冒泡等。
