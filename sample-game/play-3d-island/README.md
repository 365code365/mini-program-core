# play-3d-island 萌宠小岛（3D 经营，竖屏）

照「真实上线的经营类小游戏」做的完整示例：3D 场景 + 一整套数值系统 + 内购 / 广告 + 复杂交互。

**美术（v3 起）**：材质、图标、宠物与建筑立绘、封面由豆包 Seedream 4.5 生成（`tools/gen-island-art.py`：生成 → 无缝化 / 抠图 / 风格化 → 图集，放在 `art/`）。模型几何仍是代码拼的，贴上 AI 材质（顶点色当染色）；岛上的宠物是 AI 立绘做的广告牌。`art/` 缺了或加载失败就退回 v2 的纯代码美术。声音是代码合成的。

重新生成：`ARK_API_KEY=… python3 tools/gen-island-art.py`（原图缓存在 `target/island-art/raw/`，只补缺的）；只改加工参数：`--process-only`；重画某几张：`--redo pet_fox,tex_brick`。

## 玩法
- 岛上造 10 种建筑：麦田、伐木场、采石场、仓库、集市、面包坊（2 小麦 → 1 面包）、渔港（只能建在海边）、宠物小屋、灯塔、蜂场。
- 建筑往自己的库存里攒产出，满了停产；点建筑（或头顶的图标）收获，图标飞进资源条。全岛资源有上限，仓库加上限。
- 升级：费用按等级指数增长、要施工时间（钻石可跳过）；5 / 10 / 15 级外观变化。岛屿等级解锁新建筑。
- 集市卖货（数量步进器）、扩地（挨着岛的锁定地块，价格递增）。
- 宠物蛋：普通 / 稀有 / 史诗 / 传说四档概率（有概率公示）、50 抽保底、十连保底史诗；宠物住进小屋给对应资源加成（蜂场让加成翻倍）。
- 每日任务（0 点重置）、成就（分阶）、7 日签到（断签重来、可看广告双倍）。
- 离线收益（五折、最多 8 小时，可看广告双倍）、看广告全岛双倍产量 30 分钟（可叠加）。

## 内购与广告（真实 API）
- 钻石包、新手礼包（48 小时限时）、月卡（30 天每天领）走 `wx.requestMidasPayment`；首充双倍。
- 单据状态机：先记单 → 拉支付 → 成功先落「已付」再发货；中途被杀，下次启动补发（`js/systems/monetize.js`）。
- 激励视频 `wx.createRewardedVideoAd`（onLoad / onError / onClose({isEnded})）；分享 `wx.shareAppMessage`；登录 `wx.login`；兑换码用系统键盘 `wx.showKeyboard`（试试 `MINI2026`）。
- 宿主没接支付 / 广告时弹「模拟」确认框（明说是模拟，不悄悄发奖）。示例 App 的 `GameHostApis.kt` 给 Android 接了演示版（确认框支付、5 秒倒计时广告）。

## 交互
单指拖地图（惯性）、双指捏缩放、双指拧转向；点建筑选中（底部面板：产量 / 库存 / 升级费用对比 / 加速 / 旋转）；建造摆放（拖地图挪虚影，绿 / 红底圈表示能不能放）；
界面有页签、可滚动列表（惯性 + 回弹）、弹窗动画、按下态、红点、倒计时、系统 Modal / Loading / Toast。

## 结构（每个文件 < 500 行）
- `js/data/defs.js` 策划配置表；`js/core/` 存档（版本迁移 + 校验和 + 备份）、经济、事件总线；`js/systems/` 任务签到、抽卡、内购广告；
- `js/world/` 程序化模型、地形与水面着色器与昼夜、建筑 / 村民 / 宠物、镜头；
- `js/ui/` 界面工具包（面板 = 离屏画布贴图，脏了才重画、滚动只改 UV）、程序化图标集、3D 缩略图、各个界面；
- `js/actions.js` 玩家操作、`js/input.js` 触摸分发、`js/audio.js` 合成音效与背景音乐、`js/autopilot.js` 自动驾驶。

## 运行
先拉库：`bash tools/fetch-play-libs.sh`（three.js r160，不入库）；无头 WebGL 先 `bash tools/fetch-angle.sh`。
- 自动驾驶 + 20 倍时间：`./target/release/mini-app-window sample-game/play-3d-island --query "auto=1&fresh=1&speed=20" --frames 3600 --eval 'JSON.stringify(__play())' --snapshot target/_play/island`
- 直接打开某个界面截图：`--query "open=shop&sub=deals"`（build / shop / pets / quests / signin / sell / settings）
- 上架：`python3 tools/mini-store/upload_samples.py play-3d-island`
