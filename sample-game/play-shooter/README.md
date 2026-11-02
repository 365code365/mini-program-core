# play-shooter 星际战机（竖屏）
可玩示例。按住屏幕拖动战机（相对位移，手指不挡飞机），一直自动开火。敌机三种：侦察机（成串蛇形 / V 字俯冲，第 4 波起偶尔放一枪）、
战斗机（飞到半空悬停、三连发瞄准）、轰炸机（慢、硬、十向环形弹）；每 5 波来一个首领（扇形 13 发 / 双臂螺旋 / 五连发瞄准轮换，半血后加快，血量逐个首领加）。波次无尽，敌弹速度、血量随波次涨。
击落掉道具：**双**（火力升级，1→2→3 级：单发 / 双发 / 双发 + 两侧斜射，14 秒降一级）、**盾**（护盾 8 秒，挡子弹、撞机）、**炸**（炸弹，最多 3 颗：清屏上所有敌弹，敌机吃 30 点伤害，首领吃 8% 血）。
三条命，被击中掉一级火力、2.4 秒无敌、清掉身边的子弹。最高分存档（`shooter.best`），声音开关存档（`shooter.muted`）。
PC：方向键 / WASD 移动，空格放炸弹，P 暂停。右上角暂停键：继续 / 声音开关 / 结束本局。

用到的能力：
- WebGL 批量绘制：`js/gl2d.js` 拷自 play-snake，另加 `bake` / `stamp`（离屏 2D 画布画一次的小图拷进同一张图集，之后每颗子弹 / 粒子 6 个顶点、可染色、可叠加混合）。敌机、首领、战机录成精灵（一架一次 drawArrays），星空三层图层只改偏移滚动。一帧 13~18 次 draw，几百颗子弹 + 几百个粒子。
- 对象池：子弹 160 + 敌弹 420 + 敌机 40 + 粒子 800 + 冲击波 / 飘字 / 出场表，启动时建好（`js/world.js`），帧里不分配；HUD 文字按值变化才重新拼。
- 背景音乐 `wx.createInnerAudioContext`（`audio/bgm.wav`，8 秒 Am-F-C-G 循环，11kHz 16bit，176KB），开局 play、暂停 / 切后台 pause。
- 音效 WebAudio 预合成 AudioBuffer（`js/base.js`，本份加了 `'noise'` 波形做爆炸声）。
- `wx.onHide` 暂停世界与 BGM，`wx.onShow` 倒数 3 秒自动继续（点「继续」跳过）；`wx.vibrateShort`（中弹、炸弹、击破首领）；safeArea / 胶囊按钮避让。

性能（无头 CPU 后端 swiftshader）：每帧时间主要在填充率上 —— 一块全屏混合的渐变四边形 ~3.7ms、三团大星云 ~2.3ms，所以背景只用清屏色 + 两团小星云。
自动试玩每 50ms 规划一次走位（每帧都算要多 1~2ms）。同一时段与 play-snake 背靠背跑 1800 帧：本包 pump 中位 6.3ms，play-snake 8.4ms。

无头试玩：
```bash
./target/release/mini-app-window sample-game/play-shooter --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-shooter_play
./target/release/mini-app-window sample-game/play-shooter --query 'auto=1&wave=5' --frames 900 --snapshot target/_play-shooter_boss   # 直接打首领
```
`?auto=1`：自动驾驶（在战机周围 21 个候选点里挑未来 0.12 / 0.28 秒最安全、又对得上敌机的那个飞过去，躲不开就放炸弹），死了 1.5 秒后自动重开。
`?wave=N`：从第 N 波开始（调试首领）。`__play()` 返回状态、分数、波次、命数、炸弹、火力、各池占用数、首领血量、本帧 draw 次数。

上架：`python3 tools/mini-store/upload_samples.py play-shooter`。`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`（分类「游戏」），图标 `icon.png`。
