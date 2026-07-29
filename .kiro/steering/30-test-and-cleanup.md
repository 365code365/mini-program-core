---
inclusion: always
---

# 测试与回归：怎么证明改动没坏东西，以及跑完必须清理

## 每次改动后必须跑的六项

```bash
. "$HOME/.cargo/env"
rm -rf target/mini-storage                 # ① 必做，见下方「两个坑」
cargo build --release && cargo build --release --examples
cargo test --release                       # ② 单测
bash tools/damage-check.sh                 # ③ 增量重绘 == 整帧重绘（逐字节）
cargo run --release --example gallery      # ④ 65 张场景图
bash tools/tab-click-check.sh              # ⑤ 三个 app 的 tabBar 点击（必须 --touch）
bash tools/sdk-parity.sh                   # ⑥ 移动端 SDK 与桌面窗体逐像素一致
bash tools/clean-target.sh                 # ⑦ 收尾清理（见下）
```

涉及渲染/样式/布局时，再加逐页快照 + 双端对比：

```bash
bash tools/snapshot-all.sh sample-app      target/fin2_sample --settle 0 --time 2
bash tools/snapshot-all.sh sample/news-app target/fin2_news   --settle 0 --time 2
./target/release/examples/compare --all --rust-from target/fin2_sample --out target/fin2_cmp
```

判据与逐项清单见 `doc/引擎测试说明.md`。

## 跑完必须清 target/

**测试产物不清会无限堆积**：每个截图/对比工具都往 `target/<自己起的名字>` 里写，
跑一轮回归多出几十个目录，实测涨到过 **26GB**（364 个临时目录 + 12GB 的 debug 构建）。

```bash
bash tools/clean-target.sh          # 清测试产物 + target/debug，保留 release（不用重编）
bash tools/clean-target.sh --all    # 连 cargo 缓存一起清（下次全量重编）
bash tools/clean-target.sh --dry    # 先看会删什么
```

规则：

- **一轮回归结束就清一次**，不要留着"下次可能还用"。
- 基线目录（`target/fin2_sample` / `fin2_news` / `fin2_cmp`）也清掉 ——
  它们一条 `snapshot-all.sh` 就能重新生成，不值得长期占盘。
- 临时对比图落在 `target/` 下，**不要**写进 `doc/`、`sample/` 或仓库根目录。
- `target/debug` 永远可以删：所有脚本都跑 `--release`。

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
