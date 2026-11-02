# play-piano 钢琴块（竖屏）
可玩示例。四条轨上的黑块跟着曲子落下，按到黑块就弹出那个音；漏掉一块（黑块下沿出了屏幕）或者按到白键就结束。
黑块高度等于音符时值，休止是两块之间的空当，踩着判定线（蓝线）按就是原曲的节奏。判定：差 70ms 以内「完美」3 分、160ms 以内「很好」2 分、其余「不错」1 分；
连续 10 个「很好」以上每块多 1 分。曲子弹完从头再来一遍，每一遍快 12%，每弹 10 块再快 3%（封顶 ×2.6）。PC：D F J K（或 1~4）对应四条轨，空格开始，P 暂停。

内置 6 首公版曲目（`js/songs.js`，旋律 + 伴奏两条声部）：小星星、两只老虎、欢乐颂、铃儿响叮当、D 大调卡农、致爱丽丝。每首的最高分存在 `piano.best`，上次弹的曲子存在 `piano.last`，静音开关存在 `piano.muted`。

用到的能力：
- **WebAudio 合成钢琴音色**（`js/piano.js`）：每个音在 JS 里按 7 次谐波（含弦的非谐性）+ 起音 4ms + 快慢两段指数衰减合成 PCM，编成 16 位 WAV 交给 `decodeAudioData` 解成 AudioBuffer；解码完成之前用同一段 PCM `createBuffer` + `copyToChannel` 顶上。每次按键只建一个 `BufferSource`。30 个音在菜单 / 倒计时的空闲帧里一帧合成一个。
- **OscillatorNode + GainNode 包络**：倒计时提示音、失败音（两个小二度振荡器叠在一起），`setValueAtTime` / `linearRampToValueAtTime` / `exponentialRampToValueAtTime` 做包络。
- **按音频时钟对齐**：谱面位置每帧按 `currentTime` 的增量推进；倒计时四拍一次用 `start(when)` 排进音频时钟，第一块的下沿正好在第四拍压到判定线；伴奏提前 0.3s 按「格 → 音频时刻」换算后 `start(when)` 排程。画面、伴奏、判定用同一个时间源，掉帧时黑块跳一下但拍子不漂。上下文没在跑（无声设备）时退回帧时间（`__play().clock` 显示 `audio` / `frame`）。
- `wx.onHide` 时 `suspend()` 音频上下文（时钟停，谱面跟着停），回来点一下继续，继续前留 1 秒准备（伴奏排程同样顺延）；多点触控（每根手指按下各判一次）、`wx.onKeyDown`、`wx.vibrateShort`、存档。

画面走 WebGL（`js/gl2d.js`，与 play-snake 同一份）。性能：无头 CPU 后端（swiftshader）上一帧的时间几乎全在填充像素，整屏铺一块渐变 / 遮罩一次约 5ms，
所以底色交给清屏色，弹层只画面板不铺整屏遮罩（菜单去掉整屏渐变后 pump 中位 15ms → 9ms）；JS 侧每帧不到 1ms，合成一个音的那几帧 ~60ms（只在开头 30 帧）。

无头试玩：`./target/release/mini-app-window sample-game/play-piano --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-piano_play`
（自动试玩按到点按键、带 ±60ms 抖动，弹 28~57 块后交替地故意漏一块 / 按一次白键，2.4 秒后换下一首；日志里 `[piano] … 结束` 一行给出块数、分数、速度与音频时钟）。

上架到 mini-store：`python3 tools/mini-store/upload_samples.py play-piano`。
