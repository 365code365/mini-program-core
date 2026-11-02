# play-tetris 俄罗斯方块（竖屏）
可玩示例。10×20 棋盘（上面另有 2 行看不见的出生区），7 种方块按 7-bag 出（每 7 块里各一次）。
SRS 旋转 + 踢墙（I 与其余方块各一张 5 偏移的踢墙表）、幽灵块（落点虚影）、暂存（每块只能换一次）、3 块预览、
锁定延迟 0.5 秒（在地面上挪动 / 旋转会重置，最多 15 次）、消行先闪白再收缩（0.36 秒）后真删行、碎块粒子 + 飘字。
等级 = 1 + 行数 / 10，下落速度按 Tetris Guideline 的公式 `(0.8 - (等级-1)×0.007)^(等级-1)` 秒一格。
计分：单 / 双 / 三 / 四消 100 / 300 / 500 / 800 × 等级，连击每次 +50×连击数×等级，连续四消 ×1.5，软降每格 1 分、硬降每格 2 分。

操作：
- 触摸：点一下顺时针旋转；左右拖动按格跟手移动（一次拖动可连挪几格）；快速下滑硬降；按住不动 0.3 秒开始软降，抬手停；上滑暂存；
  也可以直接点「暂存」框暂存、点「暂停」按钮暂停。
- 键盘（`wx.onKeyDown` / `onKeyUp`）：← → 移动（按住 0.16 秒后每 0.05 秒连移）、↑ / X 顺时针、Z 逆时针、↓ 软降、空格硬降、C / Shift 暂存、P / Esc 暂停。

存档：最高分 `tetris.best`；进行中的一局（棋盘、队列、暂存、分数）`tetris.save`，每锁 5 块、暂停、切后台（`wx.onHide`）时存，
下次打开开始页给「继续上局」；结束时清掉。

用到的能力：WebGL 2D 批量绘制（`js/gl2d.js`，拷自 play-snake：不变的背景、已锁定的格子、暂存 / 预览各录成图层，棋盘没变就不重录）、
触摸手势（按下 / 移动 / 抬起自己判点按、拖动、快滑、长按）、键盘、`wx.vibrateShort`（消行 medium、四消 heavy）、
`wx.setPreferredFramesPerSecond`（玩的时候 60 帧，开始页 / 暂停 / 结束降到 30 帧）、WebAudio 预合成音效（`js/base.js`）、
`wx.setStorageSync`、`wx.onHide` 暂停。

文件：`js/tetris.js` 规则（不碰画面与输入）、`js/view.js` 画面、`js/fx.js` 粒子与飘字（对象池，帧里不分配）、
`js/ai.js` 自动试玩的落点选择（枚举朝向 × 列，按总高度 / 消行 / 空洞 / 凹凸度打分）、`js/base.js` 输入 / 音效 / 存档小底座。

无头试玩（`?auto=1`：AI 每 0.05 秒按一下旋转 / 左右 / 暂存 / 硬降去执行选好的落点，每 6 块有 1 块用软降；`&chaos=1` 随机落点，几十秒堆到顶，覆盖结束 / 新纪录 / 重开）：

```bash
./target/release/mini-app-window sample-game/play-tetris --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-tetris_play
./target/release/mini-app-window sample-game/play-tetris --query 'auto=1&chaos=1' --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-tetris_chaos
```

`GameGlobal.__play()` 返回状态、分数、行数、等级、已落块数、最高分、局数、暂存 / 预览，以及 `stats`（四消、暂存、硬降、软降格数、踢墙、移动、旋转次数）。

性能（无头 CPU 后端，swiftshader）：每帧时间主要花在填像素上。最初整屏铺渐变 + 棋盘外框整块叠在井下面，一个像素填两三次，中位帧 ~15ms（同时段 play-snake ~8.5ms）；
改成清屏色当背景、渐变只铺顶部标题条、外框画四条边之后，中位 ~9.6ms（机器负载高时 13~15ms，同负载下 play-snake 9~17ms）。

上架：`python3 tools/mini-store/upload_samples.py play-tetris`（`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`）。
