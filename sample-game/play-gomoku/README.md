# play-gomoku 五子棋（竖屏）
可玩示例。15 路棋盘人机对战，黑先，连成五子（含长连）获胜，下满 225 子算和棋。
手指按下在最近的交叉点上显示准星 + 半透明棋子（手指挡着也看得见落在哪），拖动跟着走，抬手落子。
「悔棋」撤到轮到你为止（AI 正在想就连它这一步一起取消）；「认输」用 `wx.showModal` 确认后记负；对局中点「新局」也先确认（记负）。
难度（初级 / 中级 / 高级）与先后手（我先手 / AI 先手）：还没落子时立刻换一局，下到一半则下一局生效。
连五时用红线 + 圈标出那五子，最后一手标红点。

AI（`workers/ai.js`）：五元组评分 + α-β 剪枝（负极大值）。
- 评分：棋盘上所有连续 5 格的窗口（横竖斜 572 个），窗口里只有一方的子时按子数给分（1 / 12 / 120 / 1500），两方都有就作废；
  下子 / 提子只更新经过该点的至多 20 个窗口，节点估值 O(1)。
- 候选点：已有棋子两格内的空点，按「进攻增益 + 0.9×防守增益」取前 K 个；能连五只走那一步，对方有冲四只考虑堵点；
  搜到底时对方还有冲四就顺着堵下去（不算层数），连续冲四的杀棋不会藏在深度之外。搜索里不分配对象（每层一块 Int16Array）。
- 难度：初级不搜索、前 4 名里随机挑（第一名 50%）；中级 2 层 × 宽 12；高级迭代加深 2 → 4 → 6 层 × 宽 8，限时 1.5 秒，
  超时停在算完的那一层。自对弈（Node，各 10~12 盘，第二手随机）：中级对初级 10:0，高级对中级 10:0（把时限压到模拟本引擎速度时 10:2）。
  同一局面本引擎 QuickJS 比 Node 慢约 80 倍（1.2 万节点：Node ~12ms、本引擎 ~1s）。

AI 跑在 Worker 里（`wx.createWorker('workers/index.js')`，`game.json` 配 `"workers": "workers"`），主线程照常出帧、「思考中」动画不卡；
消息带 id，悔棋 / 新局之后迟到的结果丢掉；`onProcessKilled` 时重建 Worker 重问；没有 `wx.createWorker` 或 Worker 报错时退回主线程
（先画出「思考中」那一帧，下一个定时器里再算）。AI 至少「想」0.35 秒再落子，太快没有对弈感。

存档：战绩按难度分开存 `gomoku.record`（胜 / 负 / 和、当前连胜、最长连胜、最快几手取胜），进行中的对局 `gomoku.save`（每手存、切后台存，打开接着下），
设置 `gomoku.settings`。

用到的能力：`wx.createWorker`（+ `onMessage` / `onError` / `onProcessKilled` / `postMessage`）、2D 画布 `getContext('2d')`（木纹棋盘与黑白棋子先画在离屏画布上，
线性 / 径向渐变、贝塞尔曲线，每帧只 drawImage）、`wx.showModal`（认输 / 放弃对局确认）、`wx.showToast`、WebAudio 预合成音效、`wx.vibrateShort`、
`wx.setStorageSync`、`wx.onHide`。

无头试玩（`?auto=1`：你这一方也交给 AI，初级 / 中级交替；每局第 9 手悔一次棋；结束 1.5 秒后换先后手再来；`&level=1|2|3` 指定 AI 难度）：

```bash
./target/release/mini-app-window sample-game/play-gomoku --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-gomoku_play
```

注意：无头的帧走虚拟时钟（1800 帧 ≈ 游戏里 30 秒，墙上只有一两秒），Worker 按墙上时钟算，所以 `level=3` 在无头里一局只走得了几手；验高级用默认的中级跑，或 `--eval` 在主线程直接调 `require('workers/ai.js').think(…)` 计时。

`GameGlobal.__play()` 返回状态、手数、局数、双方胜负、难度、执子、是否在想、上一手用时 / 节点数、`worker`（`worker` 或 `main`）、悔棋次数、当前难度的战绩。

上架：`python3 tools/mini-store/upload_samples.py play-gomoku`。
