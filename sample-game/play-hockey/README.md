# play-hockey 桌上冰球（横屏）
可玩示例。拖动球拍击球，打进对方球门得 1 分，先得 7 分获胜；丢球的一方开球。
- 双人同屏：左右两人各用一根手指拖自己半场的球拍，可以同时拖。
- 对战电脑：玩家拿左边的红拍（按屏幕哪里都行，球拍限在自己半场），电脑三档（简单 / 普通 / 困难：球拍最高速、反应间隔、瞄准偏差不同）。
  人机战绩（胜 / 负 / 当前连胜 / 最高连胜）用 `wx.setStorageSync('hockey.stats')` 存档，菜单与结算页上显示。
- PC：方向键 / WASD 推左边的拍，空格开始 / 再来一局，P 暂停。中间的暂停键弹出「继续 / 重新开始 / 返回菜单」。`wx.onHide` 自动暂停并放开所有手指。

用到的能力：
- 多点触控：`wx.onTouchStart / Move / End / Cancel` 按 `changedTouches[i].identifier` 跟踪。每根手指按下时认领它那一侧的球拍（双人模式按 x 分半场），
  之后移动 / 抬起只认这根手指；一只拍已经有手指时，同侧的第二根手指不抢。
- 固定步长物理（`js/physics.js`，240Hz，渲染帧的 dt 进累加器）：球与球拍（质量无穷大、按目标点限速追过去，把自己的速度带给球）、
  球与门柱（半径 0 的圆）都是圆与圆碰撞；球与四边墙反弹，两端墙中间开球门，进了门洞贴着门框走，过门线一个球身算进。球速封顶 1500 px/s。
- 电脑（`js/ai.js`）：球在自己半场就绕到球后对准对方球门推（球离边墙远时也会打边墙反弹），球在对方半场就回防，
  球朝自己来时按上下墙镜面反射预测落点；每来一记新射门重抽一次防守偏差，球越快读得越不准。
- WebAudio 碰撞音：`js/base.js` 预合成 AudioBuffer，按撞击力度分四档音高；撞墙、门柱、进球、获胜各一种；进球 `wx.vibrateShort`。
- 横屏 `deviceOrientation: landscape`、safeArea + 胶囊按钮避让、WebGL（`js/gl2d.js`）。

性能：无头 CPU 后端（swiftshader + 4x MSAA）上一帧的大头是填充面积。冰场原本画成三层整块圆角矩形 + 一层反光渐变，一帧 20~33ms；
现在清屏色就是冰面色，外面两圈护栏拆成上下左右的 `gl.scissor` + `gl.clear` 窄条，四个圆角靠图层里录的「切角」补，同一进程逐帧交替量 8~9ms（见 `js/draw.js` 注释）。
无头 1800 帧 pump 中位（机器负载 25 左右、与 play-snake 背靠背）：本包 8.1~11.6ms，play-snake 12.6~13.6ms。
`js/gl2d.js` 拷自 play-snake，多导出了 `quad`，圆的段数按物理像素算（球拍、球都是大圆，按逻辑像素算在 2x 屏上能看出棱角）。

无头试玩（左右都交给电脑对打，每局结束换一档难度）：
`./target/release/mini-app-window sample-game/play-hockey --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-hockey_play`
调试：`GameGlobal.__start('pvp' | 'ai', 难度 0~2)` 直接开一局，`GameGlobal.__sim(秒)` 按固定步长推进并返回 `__play()`；
`--eval` 里用 `__gameInput('touchstart', JSON.stringify({touches: […], changedTouches: [{identifier: 7, clientX: 150, clientY: 200}]}))` 模拟两根手指。

上架到 mini-store：`python3 tools/mini-store/upload_samples.py play-hockey`。`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`（分类「游戏」），图标 `icon.png`。
