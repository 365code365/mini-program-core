# play-sudoku 数独（竖屏）
可玩示例。四档难度（入门 / 简单 / 中等 / 困难，约 46 / 38 / 32 / 25 个提示数），题目每次现出、**保证唯一解**：
随机终盘 → 按中心对称成对挖洞 → 每挖一对用求解器数解（位掩码 + 最少候选优先回溯，数到 2 就停），不唯一就填回去（`js/gen.js`、`js/solver.js`）。
出题分帧做（每帧最多算 8ms，进度条照常走）。困难题一般停在 25~28 个提示数（对称挖洞试满 41 对为止）。

操作：点格子选中，点下面的数字键填数（同一个数再点一次 = 擦掉）；工具栏「撤销 / 擦除 / 笔记 / 提示」：
笔记模式下点数字是记候选数（格子里 3×3 小字，和选中格同一个数时高亮）；填数时自动划掉同行同列同宫笔记里的这个数（撤销一并恢复）；
提示一局 3 次（选中格空着或填错了就填它，否则填候选最少的格子）。同行 / 列 / 宫重复的数字标红（按规则算，不偷看答案）；
数字键下面的小字是这个数还剩几个，填满 9 个变灰。填满且正确就完成：对角线波浪 + `wx.showModal`（再来一局 / 返回菜单）。
PC 键盘：1~9（主键盘与小键盘）填数、0 / 退格 / Delete 擦除、方向键移动、N 笔记、Z 撤销、H 提示。

用到的能力：
- 键盘：`wx.onKeyDown`（`js/base.js` 多了一个 `rawkey` 回调，原样给 `key` / `code`）。
- `wx.showModal`（中途开新局、在菜单里覆盖上局进度要确认；完成时的结算）、`wx.showToast`（笔记开关、提示内容、撤销到底、改题目数字等）。
- storage：`sudoku.progress`（题面、答案、当前盘面、笔记、提示次数、用时、最近 60 步撤销栈，每一步都存；`wx.onHide` 也存），
  `sudoku.best` / `sudoku.stats`（各档最佳用时与完成局数）。菜单里「继续上局」接着下。
- WebGL：`js/gl2d.js`（与 play-snake 同一份），网格线录成图层；画面没变的帧不提交绘制（宿主接着显示上一帧），静止时每帧 JS 几乎为 0。
- WebAudio 预合成音效；`wx.vibrateShort`（填出冲突时）。

无头试玩（四档轮着出题、做题；每局先演示记两个笔记、故意填一个冲突数再撤销、用两次提示，然后按「候选最少」逐格填完）：
`./target/release/mini-app-window sample-game/play-sudoku --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-sudoku_play`

`GameGlobal.__play()` 给局面（提示数、已填、冲突数、笔记数、提示剩余、撤销栈长度、用时）、统计（完成局数、操作 / 按键 / toast / modal 次数）
和上一次出题的记录（`gen.clues`、`gen.checks` 校验次数、`gen.ms` 耗时、`gen.unique` 再数一遍解是否唯一）。

上架：`python3 tools/mini-store/upload_samples.py play-sudoku`（商店信息在 `store.meta.json`，图标 `icon.png`）。
