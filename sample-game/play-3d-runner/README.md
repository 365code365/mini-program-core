# play-3d-runner 3D 跑酷（竖屏）
可玩的 3D 示例。左右滑换道、上滑跳、下滑铲（PC：方向键）；矮栏要跳、高栏要铲、火车只能换道；金币 +10，跑 2 米 +1；越跑越快（最高约 120 km/h）。
用 three.js r160，经 `js/adapter.js`（最小 weapp-adapter，拷自 `sample-game/_lib/adapter.js`）跑在小游戏里；界面文字是 `js/hud.js`（正交相机 + CanvasTexture，内容不变不重传）。

**美术**：楼房立面、地铁车厢 / 车头、道床碎石、枕木、站台地砖、警示条纹、金币、远景天际线、封面由豆包 Seedream 4.5 生成（`tools/gen-runner-art.py`：生成 → 裁边 / 无缝化 → `art/`，约 1MB）。
模型是代码拼的（`js/models.js`：胶囊小人带膝肘两节关节、棒球帽和背包；三组材质的地铁；带警示灯的路障），贴上 AI 贴图；`art/` 缺了或加载失败就退回纯色。
重新生成：`ARK_API_KEY=… python3 tools/gen-runner-art.py`（原图缓存在 `target/runner-art/raw/`，只补缺的）；只改加工：`--process-only`；重画：`--redo facade_a,coin`。

**手感**：换道是临界阻尼弹簧（ω=20/s，0.19s 走完 90%，起步与停下都是渐变，连滑只改目标不丢速度）；角色按横向速度侧倾、转身，相机用更软的弹簧晚半拍跟、带一点侧滚；
跑 / 跳 / 铲 / 撞的姿势按权重平滑混合（`js/pose.js`）；相机在背后，所以跑步动作重点放在背后看得见的：重心左右压、胯肩反向扭、小腿后踢、手臂外张、落地屈膝缓冲、背包晚半拍晃（层级见 `js/hero.js`），速度越快视角越广（60°→70°）。

**库不入库**：先跑 `bash tools/fetch-play-libs.sh`（npm 固定版本、sha256 写死）把 `libs/` 拉下来，再运行或上传。

无头试玩：`./target/release/mini-app-window sample-game/play-3d-runner --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play/play-3d-runner`
（无头 WebGL 走 SwiftShader，先跑一次 `bash tools/fetch-angle.sh`）

上架到 mini-store：`python3 tools/mini-store/upload_samples.py play-3d-runner`。`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`（分类「游戏」）。
