# play-fruit 切水果（竖屏）
可玩示例。水果（西瓜、橙子、苹果、柠檬、猕猴桃、桃子）和炸弹从底下抛上来，手指划过就切成两半：两半沿刀口法线飞开、打着转落下，果汁粒子四溅，木板上留下 3 秒的果渍。
0.28 秒内接连切到 3 个以上算连切，奖励个数分（「4 连切 +4」），连切音一个比一个高。
- **经典模式**：漏掉 3 个水果或切到炸弹结束；越往后抛得越密、一次抛得越多、炸弹越多。
- **限时 60 秒**：炸弹扣 10 分；最后 5 秒读秒。

菜单上划开西瓜进经典、划开橙子进限时（点一下也行）。两种模式各存最高分（`fruit.best.classic` / `fruit.best.timed`），声音开关存档（`fruit.muted`）。右上角暂停键：继续 / 声音开关 / 返回菜单。

用到的能力：
- `wx.onTouchMove` 高频轨迹：每个触摸点进环形缓冲（`js/blade.js`，32 个点、0.16 秒寿命），刀光画成从新到旧变细变淡的带子；
  判定按事件做（每来一个点，拿「上一点 → 这一点」这一段与每个水果做**线段—圆相交**：圆心到线段最近距离 ≤ 半径），快划时一帧里的几段都算。
- WebAudio 预合成音效（刀风、果肉闷响、连切和弦、炸弹、读秒；`js/base.js`，本份加了 `'noise'` 波形）。
- `wx.vibrateShort`：切开水果（80ms 内只震一次）；`wx.vibrateLong`：切到炸弹。
- WebGL：`js/gl2d.js` 拷自 play-snake，另加 `bake` / `stamp`（果汁粒子、果渍、光点烘焙进图集，每个 6 个顶点）；木板录成图层；水果要转，直接用形状画（`js/fruits.js`）。帧里不分配。
- 性能（无头 CPU 后端 swiftshader）：时间主要花在混合填充上。木板不铺半透明整列（40% 屏的混合 ~1.5ms），只画缝和木纹；
  同一时段背靠背 900 帧，pump 中位 6.1~6.6ms（play-snake 8.8~11ms）。
- `wx.onHide` 暂停，`wx.onShow` 倒数 3 秒继续；safeArea / 胶囊按钮避让。

无头试玩：
```bash
./target/release/mini-app-window sample-game/play-fruit --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-fruit_play
./target/release/mini-app-window sample-game/play-fruit --query 'auto=1&mode=timed' --frames 3700 --snapshot target/_play-fruit_over   # 限时一局打到「时间到」
```
`?auto=1`：自动挥刀（挑飞到高处的水果，能串上就挑两个挨着的，算一条离炸弹够远的直线，分 5 帧当成手指划出来，走的是和真手指同一条刀光 / 切割路径），一局结束 1.5 秒后回菜单换另一种模式。
`?mode=timed`：自动试玩先玩限时。`__play()` 返回状态、模式、分数、两种模式最高分、命数、剩余时间、切开数、最大连切、各池占用数。
`GameGlobal.__world` 是同一份世界对象，给 `--eval` 直接造场景用（例：在屏幕中间放一个炸弹再 `__world.slice(...)` 一刀）。

上架：`python3 tools/mini-store/upload_samples.py play-fruit`。`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`（分类「游戏」），图标 `icon.png`。
