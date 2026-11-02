# play-match3 萌果消消乐（竖屏）
可玩示例。8×8 棋盘，六种水果（苹果、橙子、葡萄、柠檬、青梨、蓝莓）。在水果上往相邻方向滑一下就交换，也可以先点一个、再点相邻的一个；凑不成三连会弹回原位。6 秒没动会提示一步。

- 特效块：横 / 竖 4 连出条纹（消整列 / 整行），L / T 型出炸弹（消 3×3），5 连出彩虹球（与哪种水果交换就消掉全盘这种，两个彩虹球互换清全盘）。被波及的特效块接着炸（连锁展开）。
- 连锁：消完下落补位后又凑成的三连算连击，分数按连击倍数翻（每个水果 10 × 连击数），音高沿五声音阶往上走。
- 关卡：12 关，每关限步数、要收集指定数量的某几种水果；收齐即过关，剩余步数每步 +60。过关 1 星，分数过两条星线（步数 × 110、× 150）各加 1 星，HUD 的分数条上标着星线。死局自动洗牌。
- 存档（`wx.setStorageSync`）：`m3.progress`（解锁到第几关、每关星数与最高分）、`m3.records`（本机前 10 名成绩，排行榜用）、`m3.muted`。
- 排行榜：开放数据域（`game.json` 的 `openDataContext: "open"`）。主域 `wx.getOpenDataContext().postMessage` 把我的总分和本机记录发过去，`open/index.js` 调 `wx.getFriendCloudStorage` 拉好友托管数据画进 sharedCanvas，主域把 sharedCanvas 当 WebGL 纹理贴出来（`js/gl2d.js` 里加的 `image()`）。过关时主域 `wx.setUserCloudStorage` 写总分（key `m3_total`，`wxgame` 格式）。宿主没接关系链时 `getFriendCloudStorage` 走 fail，开放数据域改画本机记录，标题写「本机排行」并注明原因。

## 用到的能力
- WebGL 1 画 2D：`js/gl2d.js`（拷自 play-snake）。静止的水果录成一个图层（签名 = 静止格子的 id 哈希），只有在动的那几个走精灵；HUD、关卡格、按钮都录成图层，内容变了才重录。dpr ≥ 2 时不开 MSAA（无头同一局帧中位 19~23ms → 9~13ms）；圆的段数按物理像素算。
- 补间：`js/tween.js`（交换 inOutQuad、不成立回弹 outBack、消除缩放 inBack、下落 outBounce、特效块弹出 outBack），补间对象放池子复用。新掉进来的水果用 `gl.scissor` 裁在棋盘里。
- 特效：`js/fx.js` 果汁碎屑、飘字、条纹光带、炸弹光圈，都预分配。
- 音效：`js/base.js` 的 WebAudio 预合成 AudioBuffer（连击音高递增、特效滑音、过关琶音）；`wx.vibrateShort`（特效块爆炸）。
- `wx.onHide` 暂停主循环，`wx.onShow` 恢复。

## 无头试玩
```bash
./target/release/mini-app-window sample-game/play-match3 --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-match3_play
```
`?auto=1`：棋盘一空闲就挑消得最多 / 能出特效的一步走；结算后打开排行榜一秒，再进下一关（输了就重来）。加 `&hold=1` 停在第一次结算面板（截图用）。
`GameGlobal.__play()` 返回界面、关卡、剩余步数、分数、目标进度、交换 / 特效 / 最大连击 / 洗牌次数、解锁与星数、排行榜来源（`opendata` / `local-main`）。

上架：`python3 tools/mini-store/upload_samples.py play-match3`（`project.config.json` 是 `compileType: "game"`，商店信息在 `store.meta.json`）。
