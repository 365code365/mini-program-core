# 示例补功能 · 2026-11-02（拾光咖啡起头，25 个示例）

用户要求：「把这些小程序都优化一遍，现在很多小程序功能还是不完整，点咖啡的那些功能都是好多都是不全」。
冻结名单（`.kiro/steering/00-scan-policy.md`）里的示例不在本轮范围。

## 这个目录里有什么

| 文件 | 用途 |
|---|---|
| `brief.md` | 派给子任务的统一说明：「功能完整」的标准、引擎限制、验证方法 |
| `audit.py` | 静态盘点：占位文案（S）、缺处理函数（D）、跳到不存在的页面（N）、无回调的菜单 / 弹窗（A）、自定义 tabBar 没同步选中态（B），以及有误报的 E / T |
| `zz_sample_probe.rs` | 脚本驱动的探针（走 `tests/sample_harness` 的真指针链路）。**不在 `tests/` 里**：每个集成测试是一个 75MB 的二进制，用的时候临时拷回去，用完删掉 |
| `flows/<app>.txt` | 每个示例一份点击流程（共 24 份，带 `expect` 断言），本轮在最终二进制上全部跑通 |

## 怎么重跑

```bash
cp .agents/tasks/sample-polish-2026-11-02/zz_sample_probe.rs tests/
cargo test --release --no-run --test zz_sample_probe
BIN=$(ls -t target/release/deps/zz_sample_probe-* | grep -v '\.d$' | head -1)
PROBE_APP=coffee PROBE_SCRIPT=.agents/tasks/sample-polish-2026-11-02/flows/coffee.txt $BIN --nocapture
rm tests/zz_sample_probe.rs   # 用完删
python3 .agents/tasks/sample-polish-2026-11-02/audit.py coffee
```

- 截图写到 `target/_polish_shots/`（`clean-target.sh` 会清）。
- `ride` 的券有夜间券（22:00–05:00 才可用），晚上跑要钉白天：`MINI_FAKE_NOW=<epoch 毫秒>`。
- 探针命令表在 `zz_sample_probe.rs` 文件头。

## 做示例时撞到的引擎问题

已修（本轮，均有单测）：

1. 遮罩与底部面板是兄弟节点时，点面板任何地方都冒泡到遮罩的 close（`hit_test.rs` 只按包围盒找祖先）。
2. 页面滚过之后，点正常流的卡片会被底部固定输入条「抢走」成聚焦（`pointer.rs` 用了全局 `hit_test`）。

没修（示例里绕开了，最小复现见各条）：

| 现象 | 复现 | 期望（微信） |
|---|---|---|
| 用户打过字的 `<input>`，`setData` 改 `value` 画面不更新 | `<input value="{{code}}" bindinput="onCode"/>` 打字后 `setData({code:''})` | 立即按 value 重画；绕法：给 input 一个随提交递增的 `id` |
| 负 margin 叠在兄弟上的卡片，一次点击派给两个 | `.header{padding-bottom:120rpx}` 绑 A，下一个兄弟 `margin-top:-90rpx` 里绑 B，点重叠区 A、B 都触发 | 只派给最上面的 |
| fixed 里套 fixed，`boundingClientRect` 的 top 是 0 | 全屏 fixed 遮罩里放 `position:fixed;bottom:0;height:300px` 的面板 | top = 视口高 − 300 |
| tab 页里 `position:fixed;bottom:0` 的面板盖在自定义 tabBar 上，点击给了 tabBar | 面板 507–667，点 (330, 645) 切 tab | tab 页视口不含 tabBar，或点击顺序与绘制一致 |
| 定宽 `view` 里的裸文字不换行 | `<view style="width:200px">一段长文字</view>` | 换行；绕法：放进 `<text>` |
| 中文句号「。」可以出现在行首 | 300px 宽里放刚好一行半的中文 | 避头规则：不在「。」前断行 |
| 内容为空的 `<text>` 整个丢掉 | `<text>{{''}}</text><text>右</text>` 在 space-between 里，右边那个跑到最左 | 宽 0 的弹性项 |
| `▶`（U+25B6）画成彩色 emoji | `<text>▶ 12.6万</text>` | 单色字形 |
| `{{str[0]}}` 对字符串取下标是空的 | `<text>{{item.name[0]}}</text>` | 首字 |
| wx API 回调里抛的异常没有任何日志 | `wx.showActionSheet({ success() { undefinedVar } })` | 打 `[Error]`（微信报 thirdScriptError）；行程规划的「分享」就是这样静默失灵的 |
| `showModal` 的按钮文字超过 4 个汉字时弹窗不出现，也没有日志 | `confirmText: '同意并开通'` | 方向与微信一致（fail），建议打一条 warn |
