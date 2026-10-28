# play-3d-runner 3D 跑酷（竖屏）
可玩的 3D 示例。左右滑换道、上滑跳、下滑铲（PC：方向键）；矮栏要跳、高栏要铲、火车只能换道；金币 +10，跑 2 米 +1；越跑越快（最高约 120 km/h）。
用 three.js r160，经 `js/adapter.js`（最小 weapp-adapter，拷自 `sample-game/_lib/adapter.js`）跑在小游戏里；界面文字是 `js/hud.js`（正交相机 + CanvasTexture，内容不变不重传）。

**库不入库**：先跑 `bash tools/fetch-play-libs.sh`（npm 固定版本、sha256 写死）把 `libs/` 拉下来，再运行或上传。

无头试玩：`./target/release/mini-app-window sample-game/play-3d-runner --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play/play-3d-runner`
（无头 WebGL 走 SwiftShader，先跑一次 `bash tools/fetch-angle.sh`）

上架到 mini-store：`python3 tools/mini-store/upload_samples.py play-3d-runner`。`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`（分类「游戏」）。
