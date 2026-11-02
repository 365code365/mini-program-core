# 示例小程序完善 · 子任务说明（所有子任务共用）

仓库：`/Users/shengye/pojects/mini/mini-program-core`（Rust 写的微信小程序渲染引擎，对标微信 Skyline）。
`sample/<app>/` 是给引擎跑的示例小程序（标准微信小程序：WXML / WXSS / JS / JSON）。用户的要求：
**「把这些小程序都优化一遍，现在很多小程序功能还是不完整」**。你负责这里分给你的几个 app，把它们做成
「这个行业里一个真 App 该有的样子」：每个看得见的入口都能用、核心业务闭环、数据前后一致。

参考实现：`sample/coffee/`（本轮已完成：钱包充值、积分商城、收货地址增删改、订单列表 / 详情 / 取消退款 /
再来一单 / 评价、加料、活动价、连续签到、宿主能力降级）。动手前先读它的
`utils/store.js`、`utils/host.js`、`pages/cart/cart.js`、`pages/order-detail/*`，照它的写法与粒度做。

## 硬性规则

1. **只改分给你的 `sample/<app>/` 目录。** 不碰 `src/`、`tests/`、`tools/`、`doc/`、别的示例；
   尤其不碰 `sample/sample-app`、`sample/news-app`、`sample/tea-app`、`sample/real-sample`（回归基线）。
2. **不跑 `cargo build` / `cargo test` / `tools/*.sh`**（regress、clean-target、snapshot-all……）。
   这台机器有别的会话在用，内存吃紧。只用下面列出的**现成二进制**。
3. 临时文件只放 `target/tmp/zz_samples/<app>/`（`target/` 下别的目录会被别的会话的清理脚本删掉）。
4. 不新增测试文件；不联网（拉演示接口的 app 用 probe 的 `demo` 命令起本机服务）。
5. app 已有的 e2e 测试**必须仍然通过**（你不能改测试）：测试依赖的选择器、data 字段、storage 键、
   文案要保持兼容。改之前先跑一遍拿基线，改完再跑。
6. 工具调用**一条一条串着跑**：同一批里并行发出的调用会同时执行（截图还没写完就去读会读不到）。

## 「功能完整」的标准（逐 app 对照）

- **没有占位**：`即将开放 / 敬请期待 / 演示版暂不支持 / xx 功能即将上线` 一律换成真功能（新页面或页内面板）。
- **没有死按钮**：看起来能点的（按钮、「查看全部」「更多」、卡片、统计数字、菜单项、tab）都要有行为；
  `showActionSheet` / `showModal` 给了选项就要在 success 里真的做事。
- **业务闭环**：按行业补齐主链路，例如 浏览 → 详情 → 选规格 → 购物车 / 预订 → 支付 → 状态推进 →
  订单列表 → 详情 → 取消（钱 / 券 / 积分原路退回）/ 再来一单 / 评价；「我的」里的地址、余额、券、记录、设置、帮助。
- **状态一致**：数据统一放在 `utils/store.js` 这类模块里存 storage；余额、计数、角标在各页一致；
  `wx.getStorageSync` 缺省返回 `''`，数字不能写 `|| 默认值`（0 会被当成没有）。
- **宿主能力降级**：支付 / 打电话 / 地图 / 导入地址 / 扫码 / 选图 / 定位照 `sample/coffee/utils/host.js`：
  先调真 API（微信、接了 hostApis 的宿主 App 里能用）；fail 且 errMsg 不含 `cancel`（引擎里是「当前宿主未实现」）
  就退回页内模拟交互；含 `cancel`（用户取消）就什么都不做。每个 app 拷一份自己的 `utils/host.js`，按需删减。
- **自定义 tabBar**：每个 tab 页的 `onShow` 里
  `if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: <下标> });`
  （微信里每个 tab 页有自己的 tabBar 实例，不写就选中态错；引擎按路由兜底了所以本地看不出来）。
- **表单**校验输入、出错 toast；**空态**、余额不足、库存 / 名额用完等边界有处理。
- **风格**：复用 app 现有的 class、配色、圆角、间距；新页面要像同一个 App。导航栏标题写在页面 json。
- 量力而行：每个 app 优先补「用户一眼就能发现不能用」的地方，再补闭环。不要为了凑功能堆无关页面。

## 引擎的已知限制（写页面时绕开）

- 底部面板与遮罩**兄弟**写法（`<view class="mask" bindtap="close"/><view class="sheet">…`）本轮已修好：
  点面板里的选项不会再冒泡到遮罩。仍可以在面板上写 `catchtap` 保险。
- 直接写在**定宽 `view`** 里的裸文字不换行（会冲出盒子）：长文字放进 `<text>`。定宽圆块里的短标签
  （如 `+10`）用 `<text>` 包起来或 `white-space: nowrap`，否则可能在 `+` 后折行。选中态变粗体的标签加 `white-space: nowrap`。
- 画不出来的字形：`›`(U+203A)、`−`(U+2212)、`✓`(U+2713)、border 透明边拼的三角。箭头用 CSS 画
  （见 `sample/coffee/app.wxss` 的 `.arrow`），勾用旋转的 L 形边框。`★ ¥ × … →` 能画。
- WXML 的 `{{ }}` 里不要写数组字面量（`wx:for="{{[1,2,3]}}"`），放进 data。
- 事件里 `e.currentTarget.dataset` 的值会按字面量还原：`data-x="1"` 拿到的是数字 1、`data-id="12"` 是 12。
  比较一律 `Number(...)` / `String(...)` 统一类型，不要写 `=== '1'`。
- 用户打过字的 `<input>`，之后页面 `setData` 改它的 `value`（比如提交后清空）画面不更新：给这种 input 一个
  随提交递增的 `id`（`id="code{{formSeq}}"`，清空时 `formSeq + 1`），让它重建。
- 负 margin 叠在兄弟节点上的卡片：点重叠区时，被盖住的那个兄弟的 bindtap 也会触发。不要在被盖住的大块区域上绑
  tap（把点击挪到具体的那一行 / 按钮上）。
- 每做完一个 app 就让它处于**可用状态**（app.json 里登记的页面四个文件都在、能跑通），再开始下一个；不要留半成品。
- wx API：未实现的是 `undefined`；用之前看 `src/js/prelude/*.js`（`grep -n "wx.<名字>" src/js/prelude`）。
  宿主能力（requestPayment / makePhoneCall / openLocation / chooseAddress / scanCode / chooseImage /
  chooseMedia / getLocation / chooseLocation …）在本地**立刻 fail**，必须有降级。
- `showActionSheet` 最多 6 项。

## 验证（必须做，结果写进报告）

1. **静态盘点**：`python3 target/tmp/zz_samples/audit.py <app>`，改前改后各一次。S（占位）、D（缺处理函数）、
   N（跳到不存在的页面）、A（无回调的菜单 / 弹窗）、B（tabBar 选中态）必须为 0；剩下的 E / T 逐条确认是误报或
   合理的信息弹窗。
2. **探针脚本**（真指针链路，与真机同一条路）：
   ```bash
   PROBE_APP=<app> PROBE_SCRIPT=target/tmp/zz_samples/<app>/flow.txt \
     ./target/release/deps/zz_sample_probe-682405a4d061062d --nocapture 2>&1 \
     | grep -E "^\[|❌|✅|期望|实际|报错|panicked|      "
   ```
   命令表在 `tests/zz_sample_probe.rs` 文件头（open / smoke / tap / tapn / type / picker / modal / sheet /
   expect / route / toast / shot / waitfor / demo …），完整示例：`target/tmp/zz_samples/scripts/coffee.txt`。
   - 第一行 `smoke`（要参数的页面跟在后面：`smoke pages/detail/detail?id=1`）：每页能开、不报错、不空白。
   - 每个新增 / 修好的功能至少一段**真点击**流程，用 `expect` 断言 data / storage 的结果（不是只看不报错）。
   - 脚本结束自动断言逻辑层零报错。
3. **看图**：每个新增 / 改过的页面 `shot target/tmp/zz_samples/<app>/<名字>.png`，跑完后**另起一步**用读文件工具
   打开 PNG 看：没有文字溢出、豆腐块、错位，风格一致。看到问题就改，改完重拍。
4. **已有 e2e**（如有）：直接跑二进制（不要经过 cargo）：`<二进制> --test-threads=1 2>&1 | tail -5`，
   二进制见 `target/tmp/zz_samples/E2E.md`。必须全部通过。

## 报告（你的最终回复，中文，简洁）

每个 app 一节：
- 补了 / 修了什么（要点列表，说清楚用户能做什么了）、新增页面；
- 验证：探针脚本路径与结果、e2e 结果、audit 改前 → 改后（S/D/N/A/B/E/T 计数）；
- 发现的**引擎**问题（最小复现：哪个 WXML/WXSS 写法、现象、期望），不要自己修；
- 没做完 / 刻意不做的，以及原因。
