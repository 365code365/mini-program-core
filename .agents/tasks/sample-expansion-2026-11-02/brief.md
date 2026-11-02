# 示例扩充（2026-11-02）：所有子任务共用的约定

用户要求：① 已有示例小程序把功能做完；② 新增不少于 20 个不同行业的小程序，合起来把引擎支持的组件 / API 覆盖全；
③ 多加几款小游戏，能覆盖的能力都覆盖；④ 做完发布到商店服务器（发布由主任务统一做，子任务**不要上传**）。

## 硬规矩（违反任何一条都算任务失败）

1. **只改分给你的目录**（`sample/<id>/` 或 `sample-game/<id>/`）。不改 `src/`、`tests/`、`tools/`、`sdk/`、`doc/`、`Cargo.*`，
   不改别的示例。另一个会话正在改引擎（`src/`）和 `sample/coffee`，碰了会互相覆盖。
   **冻结名单（用户 2026-11-02 指定）**：bank forum game-2048 marathon mini-hub movie-online music-online music
   news-app photo-wall quiz-game reader real-sample recipe sample-app smart-home stocks tea-app toolbox touch-probe weather
   这些不要改。**被分到了就跳过**，接着做你分组里的下一个，报告里写一句「<id>：冻结，跳过」，别停下来等。
   名单定死，不因有人开始改而解冻（以 `.kiro/steering/00-scan-policy.md`「已固化」一节为准）。
2. **不跑 `cargo build` / `cargo clean` / `tools/regress*.sh` / `tools/clean-target.sh`**，不 `export CARGO_*`。
   引擎二进制 `target/release/mini-app-window` 已经编好，直接用。
   唯一例外：改**已有**示例且 `grep -l '"<id>"' tests/*.rs` 有命中时，收尾跑那几个测试
   （`cargo test --release --test <文件名去掉 .rs>`，等锁慢是正常的，别中断）。
3. **引擎有 bug / 不支持的能力不要去改引擎**：在小程序里用合理写法绕开（或换一种同样覆盖该能力的写法），
   把现象、最小复现（哪页、哪段 wxml/js、期望 vs 实际）写进你的报告「引擎问题」一节。别为了绕开而放弃覆盖清单里的能力，
   实在不行就保留调用 + 做好 fail 回调里的降级，并在报告里写明。
4. 截图、临时脚本、生成素材用的脚本都放在 `target/_<你的id>_*/` 下，**收尾删掉**。不往仓库根、`doc/`、`sample/` 写临时文件。
5. 不读、不遍历与你无关的示例源码（`sample/**` 默认不扫）。参考实现只看下面点名的几个文件。
6. 文案用中文，数据真实可信（行业里真实会出现的商品 / 价格 / 流程），不要 lorem ipsum、不要「测试1」「xxx」。

## 一个小程序包的结构（照 `sample/pet` 的样子）

```
sample/<id>/
  app.json app.js app.wxss  project.config.json  store.meta.json  icon.png
  custom-tab-bar/index.{js,json,wxml,wxss}   # 引擎原生 tabBar 不画 iconPath，有 tabBar 一律用自定义 tabBar + 图片图标
  pages/<页>/<页>.{js,json,wxml,wxss}
  components/<组件>/...   utils/...   images/...
```

- `project.config.json`：`{"appid": "wx<16 位小写十六进制，随机生成>", "projectname": "<id>", "description": "…", "compileType": "miniprogram"}`。
  生成后用 `grep -r <appid> sample sample-game` 确认全仓唯一。
- `store.meta.json`：`{"name", "description"(一句话卖点), "category", "tags"[3~6 个], "developer"(像样的公司名)}`。
  分类沿用：金融 / 工具 / 娱乐 / 美食 / 教育 / 出行 / 生活服务 / 购物 / 资讯 / 医疗健康 / 政务 / 游戏；确实不合适可新开（汽车、体育、文旅…）。
- 商店图标：`python3 tools/mini-store/make_icon.py sample/<id> <1~2 个字> '#起始色' '#结束色'`（144×144）。
- tabBar 图标、商品 / 头像 / 封面图：自己写 Pillow 脚本画（放 `target/_<id>_art/`，跑完删脚本，只留生成的图）。
  图要像样：渐变底 + 简洁的几何插画 / 线条图标，别用纯色块。tabBar 图标 81×81 透明 PNG，选中 / 未选中两张。
  照片类 JPEG 质量 ~78、宽 ≤ 600px；**整包 ≤ 1.5MB**。（文生图接口账户欠费，不可用。）
- 自定义 tabBar 的写法照 `sample/pet/custom-tab-bar/`（节点要同时写 `data-index` 和 `data-path`，引擎靠它直接切页）。
  每个 tab 页 `onShow` 里 `this.getTabBar && this.getTabBar() && this.getTabBar().setData({selected: n})`。
- 跨页数据（购物车、订单、收藏、设置、历史）走 `wx.setStorageSync`，退出重进状态还在。
- 需要宿主的能力（`getLocation`、`chooseImage`、`scanCode`、`makePhoneCall`、`requestPayment`、`login`…）在本引擎桌面端
  默认**立刻走 fail**：照样调用，`fail` 里给出合理降级（模拟支付成功页需用户再确认、定位失败用默认城市、扫码失败弹手动输入框…），
  不能让流程卡死。先 `wx.canIUse('xxx')` 判断也可以。
- 文件 500 行以内（单个 js / wxml / wxss），超了按职责拆组件 / utils。

## 「功能做完」的判据

- 每个看得见的可点元素都有真实效果（跳页、改状态、弹层、持久化）。不留「功能开发中 / 敬请期待」，不留空 `bindtap`。
- 列表有空态、加载态；表单有校验与错误提示；提交类操作有结果页或状态变化；返回再进来数据一致。
- 一个小程序至少 6 页、至少一条完整业务闭环（浏览 → 选择 → 填写 → 提交 → 结果 / 订单 → 订单详情 / 状态变更）。

## 验证（每个包都要做，报告里给数字）

1. 冒烟：`python3 tools/sample-smoke.py <id>` 必须 ✅（静态检查 + 无头逐页跑，日志里任何 JS 异常 / 未实现 API / 文件不存在都算错）。
   警告「截图几乎是纯色」要看一眼是不是真没渲染出来。
2. 交互：用无头窗体走完主流程，看日志无异常、看截图对：
   ```bash
   ./target/release/mini-app-window sample/<id> --route pages/x/x --snapshot target/_<id>_shot --settle 0.5 --time 2 \
       --eval 'setTimeout(()=>console.log("CHK", JSON.stringify({r:getCurrentPages().map(p=>p.route), d:getCurrentPages().slice(-1)[0].data.count})), 2500)' \
       --touch 187,420 --wait 0.8 --touch 300,600 --wait 2
   ```
   - 坐标是逻辑像素（375×667 视口）。`--touch` 走真实指针链路（点击类只认 `--touch`，别用 `--click`）。动作按书写顺序执行。
   - **所有 `--eval` 都在 settle 之后、动作之前跑**，返回值不打印。要看动作之后的状态：在 eval 里 `setTimeout(() => console.log(...), ms)`，
     再用足够长的 `--wait` 让它在动作之后触发（日志里 grep 你的前缀）。eval 里也可以直接调页面方法
     （`getCurrentPages().slice(-1)[0].onSubmit()`）、`wx.navigateTo` 跳到深层页再截图。
   - 截图只截**最后停留的那一页**：`<目录>/<页面路由>/rust.png`，用 read_file 看。刚写出的 PNG 有时要等一会儿才读得到，别反复重拍；
     能用日志里打印的数据判断的就别看图。不给 `--route` 且没有动作时会逐页截全部页面（sample-smoke 就是这么跑的）。
3. 每个包至少看 3 张截图（首页、一个表单页、一个结果 / 详情页），排版不能挤压、溢出、文字截断、重叠。
4. 跑完删 `target/_<id>_*`，以及 `target/mini-storage/<id>.json`。

## 小游戏包的结构（照 `sample-game/play-snake` 的样子）

```
sample-game/<id>/
  game.js  game.json  project.config.json(compileType: "game")  store.meta.json(category: 游戏)  icon.png  README.md
  js/...
```

- 2D 游戏可以拷 `sample-game/play-snake/js/base.js` + `js/gl2d.js`（WebGL 批量画形状 + 文字图集，真机性能已调过，先读 README 里的性能说明），
  也可以直接用 `wx.createCanvas().getContext('2d')`（2D 画布已走 GPU）。3D 游戏参考 `sample-game/play-3d-stack`（three.js + adapter）。
- 必须支持 `?auto=1` 自动试玩（无头跑得起来、能推进到得分 / 结束）和 `GameGlobal.__play = () => ({...状态})`，
  README 写清玩法、用到的能力、无头试玩命令。
- 验证：`./target/release/mini-app-window sample-game/<id> --query auto=1 --frames 1800 --eval 'JSON.stringify(__play())' --snapshot target/_<id>_play`，
  日志无异常、`__play()` 显示真的在玩（分数 / 步数在涨）、帧截图看着对；`python3 tools/sample-smoke.py <id>` ✅。
  看 `⏱ pump N 帧：中位 … p99 …` 这行：中位应明显低于 16ms（无头 CPU 后端，参考 play-snake 中位 ~10ms）。
- 最高分 / 存档用 `wx.setStorageSync`；`wx.onHide` 暂停、`wx.onShow` 恢复；音效优先 WebAudio 预合成 AudioBuffer（见 play-snake 的 base.js）。

## 报告格式（最后一条消息）

```
## <id>：<名称>（新增 / 完善）
- 页面：N 页（列出）；业务闭环：…
- 覆盖的能力：组件 …；API …
- 验证：sample-smoke ✅；交互走查：…（命令要点 + 结果）；e2e：…（如适用）
- 改动（完善类）：补了哪些功能、新增哪些页
- 引擎问题：…（没有就写「无」）
```
