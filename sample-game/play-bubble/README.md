# play-bubble 泡泡龙（竖屏）
可玩示例。按住屏幕瞄准，瞄准线会算好左右墙的反弹、在终点画出泡泡会停的那一格；松手发射（拖回炮台下方再松手 = 取消）。
三个及以上同色连在一起就消除（每个 10 分），因此连不到顶的泡泡整串掉落（每个 20 分，一次掉 6 个以上翻倍）。
连续几发没消除（底部「下压」那排圆点），顶上就压下来一行；泡泡越过底部红色虚线就输。点左下角「换一个」交换手里和下一颗。
打光全部泡泡过关，按发射次数给 1~3 星和奖励分。PC：← → 调角度，空格发射，↑ / ↓ 换球，P 暂停。

关卡（`js/levels.js`）：前 12 关手调（初始行数 5→9、颜色 3→6、几发不消除就下压 8→4、整片 / 竖条 / 菱形 / 锯齿四种摆法），之后「无尽」关按公式往后加。
摆法用固定种子生成，同一关每次一样。已解锁关卡存 `bubble.level`，最高分存 `bubble.best`。

用到的能力：
- 六边形网格（`js/grid.js`，奇偶行错开）：邻居、落点吸附、同色连通、悬空检测（从第 0 行做 BFS，连不到的掉落）、顶上插一行。
- 发射轨迹（`js/shot.js`）：小步前进 + 墙面镜面反弹；**瞄准线与真正的发射用同一个模拟函数**，画出来的落点就是真落点。
- `wx.createRewardedVideoAd`（`js/social.js`）：输了可以看广告复活一次（清掉最下面 3 行）。建出来就拉广告；`onError` / `show()` 失败（没接宿主、没广告）时按钮改成「免费复活（广告暂不可用）」直接给，`onClose` 里 `isEnded` 为 true 才发奖励，中途关掉提示「看完广告才能复活」。
- `wx.showShareMenu`（转发 + 朋友圈）、`wx.onShareAppMessage` / `wx.onShareTimeline`（标题带当前关卡与分数，query 带 `from=share&level=N`）、`wx.shareAppMessage`（结算页「炫耀一下」、主页「转发给好友」）。
  从分享卡片进来（启动参数 `from=share&level=N`）主页会出现「好友邀请你挑战第 N 关」，已解锁就能直接进那一关。没有宿主时转发走 fail，用 `wx.showToast` 提示。
- WebAudio 预合成短音效（`js/base.js`，与 play-snake 同一份）、`wx.vibrateShort`（下压 / 失败）、`wx.onHide` 暂停、存档。

画面走 WebGL（`js/gl2d.js`，与 play-snake 同一份，只把圆的段数系数从 2.6 提到 3.6：泡泡是主角，半径 18 的圆 12 段看得出棱角）。
网格里的泡泡整片录成一个图层，网格不变就不重录（每帧一次 drawArrays），下压时图层整体滑下来；飞行 / 消除 / 掉落的泡泡用按颜色录好的精灵。
泡泡外圈用环而不是两层实心圆叠，满屏时少填一层像素（无头 CPU 后端的时间主要在填充上）。JS 侧每帧约 1ms；
自动试玩的瞄准（`js/ai.js`）把 110 个角度各模拟一遍再打分，分到 7 帧里做（每帧 16 个），模拟时泡泡还在最低一行下面就不查碰撞。

无头试玩：`./target/release/mini-app-window sample-game/play-bubble --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_play-bubble_play`
（自动试玩挑能凑成消除、顺带掉落最多的落点，四分之一的时候故意「手滑」；输了先复活一次再重打本关，通关后自动转发一次。
`--query "auto=1&level=N"` 直接从第 N 关打；日志里 `[bubble] 第 N 关通关 / 失败` 给出发射次数、星级、总分、广告状态。）

上架到 mini-store：`python3 tools/mini-store/upload_samples.py play-bubble`。
