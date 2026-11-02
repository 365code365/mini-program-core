# play-3d-maze 3D 迷宫（竖屏）
可玩的 3D 示例。每关用 DFS（回溯法，显式栈）随机挖一张迷宫，再打通几面墙带出环路；小球从左上角出发，
收集全部宝石后右下角的出口亮起，赶在倒计时结束前滚进出口过关。10 关，迷宫从 6×6 长到 15×15，宝石 4~8 颗；
时限按「从起点贪心走遍宝石再到出口」的 BFS 步数 ×1.7 + 12 秒定（保证走得完）。按剩余时间给 1~3 星。

- 操作：屏幕下半部按住拖动 = 浮动虚拟摇杆；「重力」按钮开 `wx.startDeviceMotionListening` + `wx.onDeviceMotionChange`，
  第一次读数当放平基准，倾 22° 满速（宿主不支持时 fail 回调里 toast 提示并留在摇杆）；「俯视 / 近景」切相机高度；PC 方向键 / WASD，P 暂停，空格开始 / 下一关。
- 存档：`wx.setStorageSync('maze.progress', { unlocked, best: { 关号: { score, time, stars } } })`，标题页可选已解锁的关、看每关星级与最佳。
- 生命周期：`wx.onHide` 暂停游戏并停传感器，`wx.onShow` 恢复（开着重力的话重开并重新取基准）。
- 音效：WebAudio 预合成 AudioBuffer（`js/base.js`）；拿宝石 `wx.vibrateShort`、超时 `wx.vibrateLong`。

性能（draw call 少）：3D 一帧 5~7 次 draw（`__play().calls3d`）。全部墙砖合并成一个 BufferGeometry，只生成露在外面的面
（相邻两块墙之间的面不生成）；路砖合并成一块、顶点色画棋盘格；宝石是一个 InstancedMesh；材质全程复用、换关只换几何体
（不 dispose 材质，避免换关重编着色器）。没用 `BufferGeometryUtils`：它在 examples/jsm，要另带 addons。
关掉了 MSAA：2 倍像素比下锯齿不明显，无头 SwiftShader 上开着中位 17.4ms、关掉 9.4ms（墙和地面铺满全屏，填充率是大头）。
小地图是一块 2D 面板纹理（`js/hud.js` 的 panel），只在拿到宝石 / 出口打开时重画，玩家点是每帧挪的小方块。

three.js r160（只带 `build/three.cjs` 一个文件，约 1.25MB，整包约 1.3MB）经 `js/adapter.js`（最小 weapp-adapter）跑；界面文字是 `js/hud.js`。
**库不入库**：`libs/` 在 .gitignore 里，由 `bash tools/fetch-play-libs.sh` 拉（需要把本包加进脚本的目标列表），再运行或上传。

无头试玩：`./target/release/mini-app-window sample-game/play-3d-maze --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-3d-maze_play`
（auto=1：BFS 找最近的宝石、再去出口，过关自动下一关；开局试一次重力感应，不支持就静默回摇杆。无头 WebGL 走 SwiftShader，先跑一次 `bash tools/fetch-angle.sh`）
`__play()` 给出 state / level / gems / need / time / limit / cleared / score / tilt（off·on·fail·unsupported）/ calls3d / tris。
上架到 mini-store：`python3 tools/mini-store/upload_samples.py play-3d-maze`。`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`（分类「游戏」）。
