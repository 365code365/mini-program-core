---
inclusion: always
---

# 测试与回归：怎么证明改动没坏东西，以及跑完必须清理

## 每次改动后必须跑的七项

```bash
. "$HOME/.cargo/env"
rm -rf target/mini-storage                 # ① 必做，见下方「两个坑」
cargo build --release && cargo build --release --examples
cargo test --release                       # ② 单测
bash tools/damage-check.sh                 # ③ 增量重绘 == 整帧重绘（逐字节）
cargo run --release --example gallery      # ④ 65 张场景图
bash tools/tab-click-check.sh              # ⑤ 三个 app 的 tabBar 点击（必须 --touch）
bash tools/sdk-parity.sh                   # ⑥ 移动端 SDK 与桌面窗体逐像素一致（静态页）
bash tools/interaction-check.sh target/_ia # ⑦ 指针层与覆盖层（picker/Modal/按压/手势）
bash tools/clean-target.sh                 # ⑧ 收尾清理（见下）
```

涉及渲染/样式/布局时，再加逐页快照 + 双端对比：

```bash
bash tools/snapshot-all.sh sample-app      target/fin2_sample --settle 0 --time 2
bash tools/snapshot-all.sh sample/news-app target/fin2_news   --settle 0 --time 2
./target/release/examples/compare --all --rust-from target/fin2_sample --out target/fin2_cmp
```

判据与逐项清单见 `doc/引擎测试说明.md`。

## 改指针层 / 覆盖层时的两条额外要求

**① 静态像素守不住输入。** `snapshot-all` 只出静态页、`damage-check` 只走 `--eval`，
两者都绕过指针链路。`bash tools/interaction-check.sh <目录> [基线目录]` 是唯一的判据：
picker 弹面板、Modal 按钮按压/命中、按压态、长按、手势归属。它自带**空转守卫** ——
坐标落空一像素，面板就没弹出来，快照退化成普通页面、前后自然一致，于是「全绿但没测到」
（`damage-check` 第一版就这么空转过）。守卫要求「弹出 vs 没弹出」至少差 20% 像素。

**② 时间相关的场景不要比像素。** 拖动惯性、面板入场动画、每秒 setData 的倒计时都按
墙上时钟走：同一个二进制跑两次 `--drag 8x30` 实测差 **24%** 像素。所以能比像素的场景
一律「等动画落定 + 避开倒计时页面」，手势归属这类改用 `MINI_SCROLL_LOG` 的日志断言。

**③ SDK 的输入要单独测。** 桌面窗体与 `MiniEngine` 共用 `host::input`，但共用是靠
`src/tests/engine_input_tests.rs` 守着的（直接驱动引擎跑手势/tabBar/picker/Modal）。
这两条链路分叉过一次而没人发现：引擎那份是简化版，静态截图一模一样。

## 跑完必须清 target/

**测试产物不清会无限堆积**：每个截图/对比工具都往 `target/<自己起的名字>` 里写，
跑一轮回归多出几十个目录，实测涨到过 **26GB**（364 个临时目录 + 12GB 的 debug 构建）。

**有一页的基线跨天就失效。** `news-app` 的 `pages/hot/hot` 里有签到日历，`hot.js` 用
`new Date()` 高亮**今天**。`--time` 只固定 CSS 动画时钟，不动 JS 的 `Date` —— 所以隔天再比
就会在日历那一格上差 0.1% 左右（实测 155×55 像素的方框）。看见这一页单独不一致、
而且差异正好落在日历上，先看基线是哪天生成的，别急着找渲染的锅。
根治要给无头模式一个可注入的固定日期（尚未做）。

**清 storage 要清两处。** 桌面写 `target/mini-storage/`，而 SDK（`examples/sdk_parity.rs`）
写 `target/sdk-parity/mini-storage/`。只清桌面那处的下场：news-app 的「已读 N 篇」在 SDK 侧
一轮轮攒下去，下次 `sdk-parity` 冒出 0.07% 的差异，位置还正好在文字上 —— 看着像渲染改坏了，
实际是测试数据。`tools/sdk-parity.sh` 已经两处都清，别再手写单独一处。

**基线目录用 `target/baseline_*` 命名**：`clean-target.sh` 对这个前缀留白名单。
本轮踩了两次 —— 交互基线写在 `target/_ia`，回归后清理带走，下次对比只能报「基线缺图」，
而重新生成的基线已经是改动**之后**的了，参照物就永远丢了。

```bash
bash tools/clean-target.sh            # 清测试产物 + target/debug，保留 release 与 baseline_*
bash tools/clean-target.sh --targets  # 再清交叉编译目标目录（iOS/Android，约 6GB）
bash tools/clean-target.sh --all      # 连 cargo 缓存一起清（下次全量重编）
bash tools/clean-target.sh --dry      # 先看会删什么
```

规则：

- **一轮回归结束就清一次**，不要留着"下次可能还用"。
- 基线目录（`target/fin2_sample` / `fin2_news` / `fin2_cmp`）也清掉 ——
  它们一条 `snapshot-all.sh` 就能重新生成，不值得长期占盘。
- 临时对比图落在 `target/` 下，**不要**写进 `doc/`、`sample/` 或仓库根目录。
- `target/debug` 永远可以删：所有脚本都跑 `--release`。
- 编过移动端库之后跑一次 `--targets`：那几个交叉编译目录一共约 6GB，
  而产物已经拷进 `sdk/` 了，中间件留着没用。

## 两个坑（踩过，会让你误判成回归）

1. **跑快照前必须 `rm -rf target/mini-storage`**。无头点击/输入会把数据落到
   `target/mini-storage/<app>.json`，下次启动读回来，页面内容就和基线不同了。
   曾让 sample 的购物车页从 1.51% 假崩到 10.05%。

2. **`sample-app` 的基线快照必须用 `--settle 0`**。`pages/index/index` 有一个延时
   弹出的浮层，`--settle ≥ 0.5` 就会弹出来，与基线产生 93.9% 的**假**差异。

另有一个已知的非确定场景：`doc/gallery/35_gif_frame4.png` 按墙上时钟取 GIF 帧，
连续两次跑就会不一致（约 6.8%）。只有它变、且包围盒落在动图那块时按噪声处理；
**其余任何一张变了都要当真**。

## 点击类回归只认 `--touch`

`--click` 直接调 `handle_click`，绕过整个指针层。曾经就是因为只用 `--click` 测，
漏掉了「按下 tabBar 时提前 return，触摸状态机没启动 → 抬手没有 tap」这条 bug：
无头全绿，真机上 tabBar 完全点不动。

## 改动必须给出数字

不要只说"看起来没问题"。给出：

- `python3 tools/pixdiff.py a.png b.png` 的差异占比 **与包围盒**（包围盒决定是真回归还是时钟噪声）；
- 或 `compare` 报告里的整体百分比（当前基线：sample **4.55%**、news **6.33%**）。
