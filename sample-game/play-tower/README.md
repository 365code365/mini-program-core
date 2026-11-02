# play-tower 塔防守卫（竖屏）
可玩示例。敌人从出兵口沿路走向城堡，点空地建塔、点塔升级或出售（返还 70%），守满 10 波过关；漏一个扣 1 点生命（Boss 扣 5），20 点生命扣光失败。
3 张关卡图（青石关 / 河谷道 / 烽火台，敌人血量 ×1 / 1.3 / 1.6）依次解锁，按剩余生命给 1~3 星。

- 塔（各 3 级）：箭塔（射速快、能打飞行）、炮塔（溅射，打不到飞行）、冰塔（落点一圈减速 35%~55%）、电塔（连锁 3~5 个、无视护甲）。
- 敌人：步兵、快兵、甲兵（护甲直接减伤，至少留 25%）、飞龙（飞行，不走路、直线飞向城堡）、督军（第 5 波）、巨魔王（第 10 波 Boss）。
- 经济：开局 180 金，击杀给赏金，每波开始给波次奖励；一波出完后 12 秒倒计时自动出下一波，提前出兵每剩 1 秒奖 3 金。右上角 ×1 / ×2 / ×3 倍速。
- 寻路：路线不手写拐点，`js/maps.js` 从出兵口沿路砖 BFS 到城堡，再并成拐点折线；每块空地预算「射程内的路砖数」给自动试玩选塔位。
- 索敌：网格分桶（每格一个桶，每步重建，数组复用），塔冷却好了才查射程方框里的桶；候选写进 wasm 内存后调
  `WXWebAssembly` 模块（`wasm/combat.wasm`，源码 `wasm/combat.wat`）的 `pick`（射程内走得最远的）/ `within`（圆内全部，溅射、减速、连锁）/ `dmg`（护甲减伤）。
  开局拿同一组随机输入与 JS 版对一遍，不一致、没有 `WXWebAssembly` 或加载失败都退回同一套算法的 JS 版（`__play().wasm` 是 `wasm` / `js`）。
  combat.wasm 是按 .wat 逐条手编的字节码（302 字节，没有引入 wat 编译器）；改 .wat 要同步重编。
- 画面：WebGL（`js/gl2d.js`，拷自 play-snake，只多了一个 `antialias` 选项），地图录成图层、换关才重录；敌人 / 子弹 / 特效是对象池，模拟步里不分配。
  关了 MSAA：地图铺满大半屏，无头 SwiftShader 上开着每帧中位 17~18ms、关掉 9.5~12ms（同机同时段 play-snake 10~12.7ms）。JS 每帧绘制约 1.8ms、一个模拟步约 0.05ms。
- 存档：`wx.setStorageSync('tower.progress', { unlocked, best: { 关号: { score, stars } } })`。`wx.onHide` 自动暂停。
- 音效：WebAudio 预合成 AudioBuffer（`js/base.js`），同一种音 70ms 内只响一次；漏怪 `wx.vibrateShort`、失败 `wx.vibrateLong`。

PC：空格出兵 / 继续，P 暂停。
无头试玩：`./target/release/mini-app-window sample-game/play-tower --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-tower_play`
（auto=1：`js/auto.js` 按「箭、炮、箭、冰、电…」的顺序在覆盖路砖最多的空地建塔、钱够就升级，能出兵就提前出兵，3 倍速；结束后赢了下一关、输了重来）
`__play()` 给出 map / wave / lives / gold / kills / towers / levels / enemies / wasm / wasmCalls / unlocked。
上架到 mini-store：`python3 tools/mini-store/upload_samples.py play-tower`。`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`（分类「游戏」）。
