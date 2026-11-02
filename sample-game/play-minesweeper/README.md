# play-minesweeper 扫雷（竖屏）
可玩示例。三档难度：初级 9×9 / 10 雷、中级 16×16 / 40 雷、高级 16×30 / 99 雷（经典高级是 30×16，竖屏竖着放）。
首次点击必不是雷（雷避开第一下点的格子和它周围 8 格，所以第一下一定能展开一片），点到空白自动展开（迭代栈，不递归）。

操作：点按翻开；**长按插旗**（按住 380ms 不动就插，不等抬手，格子上有一圈进度）；**双击已翻开的数字**快速开周围（周围旗数 = 数字时）；
底栏「点按：翻开 / 插旗」切换点按的作用（插旗模式下单击数字就开周围）；**双指捏合缩放**、单指拖动平移，底栏「放大 / 全览」一键切换；
顶栏笑脸重开。PC：右键插旗、滚轮缩放。胜负后亮出全部雷与插错的旗，记最佳用时与胜 / 局数。

用到的能力：
- 2D 画布（`getContext('2d')`）：格子画在离屏画布上，只重画变了的格子（每格从一张 14 种样子的图块表 `drawImage` 一块）；
  每帧按镜头把可见部分 `drawImage`（9 参数）到主画布。画面没变的帧不画（主画布保留上一帧），静止时每帧 JS 几乎为 0。
- 触摸：`wx.onTouchStart / Move / End / Cancel`，长按自己算时长，捏合按 `e.touches` 前两根手指算距离与中点（`js/input.js`）；`wx.onMouseDown`（右键）、`wx.onWheel`。
- 存档：`wx.getFileSystemManager()` 在 `${wx.env.USER_DATA_PATH}/minesweeper/save.json` 存最佳用时、胜负统计和没下完的那一局
  （`accessSync` / `mkdirSync` / `writeFileSync` / `readFileSync` / `statSync`；文件系统不可用时退回 storage）。菜单里「继续上局」接着下。
- `wx.onHide` 存档、`wx.onShow` 重画；WebAudio 预合成 AudioBuffer 音效；`wx.vibrateShort`（插旗）/ `wx.vibrateLong`（踩雷）；safeArea 与胶囊按钮避让。

无头试玩（初级 → 中级 → 高级轮着来，高级放大 1.5 倍、镜头跟着走；推理只用两条单格规则，推不出就按概率猜）：
`./target/release/mini-app-window sample-game/play-minesweeper --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-minesweeper_play`

`GameGlobal.__play()` 给局面与统计（翻开数、旗数、用时、胜 / 负局数、缩放倍数、存档写了几次 / 文件大小、各手势次数）；
`GameGlobal.__probe()` 给屏上第一个没翻开的格子与第一个数字格的中心坐标（无头走查时拿去喂触摸事件）。

上架：`python3 tools/mini-store/upload_samples.py play-minesweeper`（商店信息在 `store.meta.json`，图标 `icon.png`）。
