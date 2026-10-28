# play-3d-bowling 3D 保龄球（竖屏）
可玩的 3D 示例。左右拖动摆好位置，往前一划把球扔出去（划得越快球越快，斜着划斜着走，划个弧线带拐弯；PC：左右键 + 空格）；十局制标准计分（全中 / 补中 / 第十局加投）。
用 three.js r160 + cannon-es 0.20（刚体物理），经 `js/adapter.js`（最小 weapp-adapter，拷自 `sample-game/_lib/adapter.js`）跑在小游戏里；界面文字是 `js/hud.js`（正交相机 + CanvasTexture，内容不变不重传）。

**库不入库**：先跑 `bash tools/fetch-play-libs.sh`（npm 固定版本、sha256 写死）把 `libs/` 拉下来，再运行或上传。

无头试玩：`./target/release/mini-app-window sample-game/play-3d-bowling --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play/play-3d-bowling`
（无头 WebGL 走 SwiftShader，先跑一次 `bash tools/fetch-angle.sh`）

上架到 mini-store：`python3 tools/mini-store/upload_samples.py play-3d-bowling`。`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`（分类「游戏」）。
