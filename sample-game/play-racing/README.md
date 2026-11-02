# play-racing 极速狂飙（横屏）
可玩示例。伪 3D 公路上一直往前开（油门自动），换道躲开前面的慢车，吃金币，压橙黄色加速带冲到 290 km/h，捡红色油桶续油（+30）。
撞车掉速、扣 8 点油、手机长振；冲出路面开不快。油耗尽后滑行停下就结束，得分 = 里程（每 10 m 1 分）+ 金币 × 50，最高分存档。
一圈约 1950 段（满速 30 多秒），每过一圈换一种天色（晴天 / 黄昏），多两辆车，油桶出现得更少。

操作：
- 倾斜手机转向：`wx.startAccelerometer({interval: 'game'})` + `wx.onAccelerometerChange`。横屏时设备 y 轴指向屏幕左边，
  右边往下压 → `y` 变负 → 往右打；`wx.onDeviceOrientationChange` 报 `landscapeReverse`（反着拿）时符号反过来。
- 传感器不可用（`startAccelerometer` 走 fail，本引擎桌面端默认如此）：按住屏幕左 / 右半边转向，左右下角显示两个箭头；
  多根手指按 `identifier` 分别记，两边都按着就抵消。PC：方向键 / A D 转向，空格开始，P 暂停。
- 右上角胶囊左边是暂停键；`wx.onHide` 自动暂停并停掉加速度计，`wx.onShow` 重新开。

用到的能力：横屏 `deviceOrientation: landscape`、safeArea + 胶囊按钮避让（HUD 排在 `safeArea` 与 `getMenuButtonBoundingClientRect` 之间）、
加速度计、`wx.vibrateLong`（撞车）、WebAudio 预合成音效（`js/base.js` 的 `beep`）、`wx.setStorageSync`（`racing.best`）、WebGL（`js/gl2d.js`）。

画面（`js/scene.js`）：每段起点投影成 (x, y, 半宽)，弯道是逐段累加的横向偏移，坡是段端点高度；从近到远算哪些段露在坡顶之上，
再从远到近画路面并紧跟着画这一段上的树 / 车 / 金币（画家算法，近处坡顶自然盖住远处的车）。赛道与路边景物是固定种子生成的（`js/track.js`）。
性能：每帧不分配（投影放预分配的 Float32Array，雾化 8 档颜色开局查表）；远处三段并成一块路面，远处的树 / 车 / 金币按屏上大小降成一两块；
整屏宽的纯色（草条、天空上半、草地底色）用 `gl.scissor` + `gl.clear` 铺 —— 无头 CPU 后端（swiftshader + 4x MSAA）上一帧的大头是填充面积，
十几条整屏宽的草条（69 个顶点）就要 ~5ms，换成裁剪清屏后同一进程逐帧交替量从 10.2ms 降到 7.4ms。真机 GPU 上瓶颈是 JS 拼顶点，所以远山仍录成图层。
本包的 WebGL 上下文关了 MSAA（`antialias: false`，play-snake 开着）：同机背靠背 600 帧，开着 pump 中位 ~20ms、关掉 ~13ms；
路肩斜边在 2x 屏上是半个逻辑像素的台阶，伪 3D 像素风看不太出来。最终（机器负载 25 左右、与 play-snake 背靠背 1800 帧）：本包 9.6~13.0ms，play-snake 12.6~13.6ms。
`js/gl2d.js` 拷自 play-snake，多导出了 `quad`（梯形路面）、`flush`（直接调 gl 清屏前先把攒着的三角形画掉）、`stats`（每帧 draw 次数与顶点数）。

无头试玩：`./target/release/mini-app-window sample-game/play-racing --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-racing_play`
（自动驾驶：哪条车道前方的慢车最远就去哪条，同样空时去有金币 / 油桶 / 加速带的车道）。
调试：`GameGlobal.__sim(秒)` 按固定步长推进并返回 `__play()`；`--eval` 里可以 `__hostDispatch('accelerometerChange', JSON.stringify({x: 0, y: -0.3, z: -0.9}))`
模拟往右倾斜、`__gameInput('touchstart', …)` 模拟按住半屏，再 `__sim(0.5)` 看 `x` 的变化。

上架到 mini-store：`python3 tools/mini-store/upload_samples.py play-racing`。`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`（分类「游戏」），图标 `icon.png`。
