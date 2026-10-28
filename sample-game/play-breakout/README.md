# play-breakout 打砖块（竖屏）
可玩示例。手指在屏幕任意处左右拖动挡板（相对拖动），点一下发球；PC：方向键移动、空格发球、P 暂停。
画面走 WebGL 1：`js/gl2d.js` 把矩形 / 圆角矩形 / 圆与椭圆 / 多边形 / 粗线 / 描边 / 渐变拆成三角形攒成一批画，文字在离屏 2D 画布写好拷进同一张图集纹理（不打断批次、内容不变不重传；描边字用 16 方向偏移的 fillText，不用 strokeText）。四个 2D 游戏共用这份（各拷一份，包自包含）。
性能（一加 PLY110 真机，每帧都记）：每帧时间几乎全在 JS 拼顶点上，所以不变的东西录成图层 / 精灵（`R.layer` / `R.sprite`，录一次、每帧一次 drawArrays，位置缩放透明度走 uniform）；文字按单个字符进图集；音效在 `js/base.js` 里预合成成 AudioBuffer 播（现建 Oscillator 一次 ~25ms，就是每次响一声掉一帧）。
砖块：彩色一下碎、灰色硬砖两下、深色钢砖打不碎。道具：W 加长、M 三球、S 减速、+ 加命。五关（`js/levels.js`）循环，每轮加速。切后台自动暂停。
用到：多点触控里的相对拖动、子步长碰撞、粒子、渐变、WebAudio、onHide。
无头试玩：`./target/release/mini-app-window sample-game/play-breakout --query auto=1 --frames 3600 --eval 'JSON.stringify(__play())' --snapshot target/_play/breakout`

上架到 mini-store（服务器要先部署支持小游戏的版本）：`python3 tools/mini-store/upload_samples.py play-breakout`。
`project.config.json` 是 `compileType: "game"`（微信开发者工具按小游戏打开），商店信息在 `store.meta.json`（分类「游戏」），图标 `icon.png`。
