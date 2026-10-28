# play-flappy 飞翔小鸟（竖屏）
可玩示例，不是测试夹具。按下屏幕（PC：空格 / 上方向键）往上扑，穿过水管得分，碰到水管或地面结束；最高分存在 `flappy.best`。
画面走 WebGL 1：`js/gl2d.js` 把矩形 / 圆角矩形 / 圆与椭圆 / 多边形 / 粗线 / 描边 / 渐变拆成三角形攒成一批画，文字在离屏 2D 画布写好拷进同一张图集纹理（不打断批次、内容不变不重传；描边字用 16 方向偏移的 fillText，不用 strokeText）。四个 2D 游戏共用这份（各拷一份，包自包含）。
性能（一加 PLY110 真机，每帧都记）：每帧时间几乎全在 JS 拼顶点上，所以不变的东西录成图层 / 精灵（`R.layer` / `R.sprite`，录一次、每帧一次 drawArrays，位置缩放透明度走 uniform）；文字按单个字符进图集；音效在 `js/base.js` 里预合成成 AudioBuffer 播（现建 Oscillator 一次 ~25ms，就是每次响一声掉一帧）。
用到：主画布、rAF、onTouchStart、onKeyDown、WebGL 批量绘制、WebAudio 合成音效、vibrateShort、存储。
无头试玩：`./target/release/mini-app-window sample-game/play-flappy --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play/flappy`

上架到 mini-store（服务器要先部署支持小游戏的版本）：`python3 tools/mini-store/upload_samples.py play-flappy`。
`project.config.json` 是 `compileType: "game"`（微信开发者工具按小游戏打开），商店信息在 `store.meta.json`（分类「游戏」），图标 `icon.png`。
