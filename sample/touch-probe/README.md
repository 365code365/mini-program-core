# touch-probe · 触控探针小程序

一页三张卡片（上方的头部只显示事件计数与读数，不挂绑定），把所有触摸相关的绑定都写全，用来**观察引擎实际派发了什么**
（每个处理函数只做一件事：把事件对象的关键字段打到 console）。

| 区域 | y（逻辑 px） | 绑定 |
|------|--------------|------|
| `.header` | 0 ~ 120 | 无（计数与读数，在 `.outer` 之外） |
| `.inner` | 120 ~ 320 | `bindtouchstart/move/end/cancel`、`catchtap`、`bindlongpress`，外层还有 `capture-bind:touchstart` |
| `.mask` | 332 ~ 532 | `catchtouchmove`（遮罩锁滚动的标准写法） |
| `.tall` | 544 ~ 1744 | 只为把页面撑高，让页面可以滚动 |

外层 `.outer` 同样绑了 touchstart/move/end/cancel/tap/longpress，于是**捕获与冒泡的顺序、
`catch` 的截断位置、`target` 与 `currentTarget` 的区别**都能一眼看出来。

```bash
# 短按：capture → 冒泡的 touchstart，touchend，然后 tap（被内层 catchtap 截断）
./target/release/mini-app-window sample/touch-probe --route pages/p/p --time 2 --touch 180,220,100 --snapshot target/tp

# 长按 600ms：longpress 内→外冒泡，松手仍有 tap（微信没有 300ms 上限）
./target/release/mini-app-window sample/touch-probe --route pages/p/p --time 2 --touch 180,220,600 --snapshot target/tp

# 上滑：touchmove 链 → 被页面滚动接管后补 touchcancel → 不再有 tap
MINI_SCROLL_LOG=1 ./target/release/mini-app-window sample/touch-probe --route pages/p/p --time 2 \
    --swipe 180,220,180,-80,16 --snapshot target/tp

# 从遮罩起手上滑：catchtouchmove → 手势归属 Blocked，页面滚动保持 0
MINI_SCROLL_LOG=1 ./target/release/mini-app-window sample/touch-probe --route pages/p/p --time 2 \
    --swipe 180,430,180,370,10 --snapshot target/tp
```

卡片里的子节点带与卡片相同的 `data-who`，所以 EV 行里的 `target` 与改版前一致；
每个处理函数打完 EV 行后多一次 `setData`，只刷新页面上的读数。区域高度写死在 `p.wxss`，改高度要同步改这里的坐标。

`./run.sh touch-probe` 可以开窗体手动划，终端会实时打印事件流。
