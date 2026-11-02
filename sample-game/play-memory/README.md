# play-memory 记忆翻牌（竖屏）
可玩示例。三档难度：初级 4×4、中级 4×5、高级 6×6。每次翻两张，一样就配上，不一样一秒后盖回。
计时、计步；连着配上分数递增（100、150、200…），比标准用时快有速度奖励，按步数给 1~3 星。各档最佳成绩存档（`mem.best`），卡面与音乐开关也存档。

用到的能力：
- 翻牌动画：`js/tween.js` 补间，scaleX 从 1 到 0 再到 1 模拟 3D 翻转，配对成功弹一下、失败抖一下。
- 分包：卡面三套，几何在主包，节气 / 生肖在分包 `themes/`（game.json `subpackages`），选到时 `wx.loadSubpackage` 再 `require`，加载中有进度提示。
- 背景音乐：`wx.createInnerAudioContext` 循环播 `audio/bgm.mp3`，`wx.onHide` 暂停、`wx.onShow` 恢复；音效用 WebAudio 预合成 AudioBuffer（`js/base.js`）。
- 画面走 WebGL：`js/gl2d.js` 拷自 play-snake。

无头试玩：
`./target/release/mini-app-window sample-game/play-memory --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-memory_play`
`?auto=1` 按「先翻没见过的、见过的就去配」的策略自动玩，三档轮着来。`__play()` 给画面、难度、已配对数、步数、分数、连对、已加载的分包卡面与背景音乐状态。

上架：`python3 tools/mini-store/upload_samples.py play-memory`。`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`（分类「游戏」），图标 `icon.png`。
