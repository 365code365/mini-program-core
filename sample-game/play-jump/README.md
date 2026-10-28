# play-jump 跳一跳（竖屏）
可玩示例。按住屏幕蓄力（小人和脚下的方块一起往下压，最多 1.6 秒），松开起跳，按得越久跳得越远；PC：按住空格。
落在下一个方块上 +1，落在正中心 +2，连续正中心 +4、+6……（最多 +32，带涟漪与音阶）；没落上去就掉下去，游戏结束。方块越往后越小、越远，背景色随分数变化。
画法：WebGL 1（`js/scene.js`）—— 正交等距相机、方向光照、深度测试管遮挡；方块 / 圆柱 / 棋子是网格（`js/gl.js`：矩阵、着色器、车床体），
影子 / 涟漪 / 粒子是半透明层，背景渐变是一张整屏矩形。文字（得分、提示、结算、按钮）在离屏 2D 画布上写好、传成纹理贴上去，
内容不变不重传（`js/labels.js`）。玩法层（蓄力、落点判定、计分、镜头）与 2D 版一致。
主画布要先按 `pixelRatio` 设好宽高再 `getContext('webgl')`（小游戏主画布默认是逻辑尺寸）；
`js/base.js` 的 2D 上下文是用到才拿的，否则一块画布先有了 2D 就拿不到 WebGL。
无头试玩：`./target/release/mini-app-window sample-game/play-jump --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play/jump`

上架到 mini-store：`python3 tools/mini-store/upload_samples.py play-jump`。
`project.config.json` 是 `compileType: "game"`（微信开发者工具按小游戏打开），商店信息在 `store.meta.json`（分类「游戏」），图标 `icon.png`。
