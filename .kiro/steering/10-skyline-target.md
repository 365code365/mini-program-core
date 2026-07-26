---
inclusion: always
---

# 对齐目标：微信小程序 Skyline 渲染引擎

本项目的**唯一对标对象**是微信小程序的 Skyline 渲染引擎。任何「行为该是什么样」的争议，以官方文档为准：

- Skyline 介绍：https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/introduction.html

「差不多能用」不算达标。测试目标必须是**可判定的**：说得出期望值、说得出怎么验证。

## 对齐的四个维度

| 维度 | 达标判据 |
|------|----------|
| **渲染结果** | 同一份源码，原生窗体与编译出的 H5 逐像素对比（`examples/compare`），差异比例不回退 |
| **交互语义** | 事件冒泡/`catch` 截断、`position:fixed` 不穿透、按压态 `:active`/`hover-class`、picker 面板等，与微信一致 |
| **流畅度** | 稳态贴住显示器刷新率（144Hz 上 6.9~7.0ms 帧间隔），**不看平均帧率，看最慢一帧与帧间隔峰值** |
| **API 行为** | `setData` 语义、生命周期顺序、`e.detail` 的**类型**（下标是 number、多列是数组、time/date 是字符串） |

## 写测试目标时的要求

不合格的目标：「让首页流畅一点」「picker 能用」。

合格的目标：
- 「首页稳态帧间隔 ≤7.2ms，最慢一帧 ≤6.9ms，且 `MINI_FPS=1` 的整帧归因里 `需重绘` 为 0」
- 「`picker mode="selector"` 选中第 3 项按确定后，页面 `e.detail.value === 2`（number 而非字符串），且 H5 端同值」
- 「弹窗弹着时点遮罩外的商品卡不触发 `onProductTap`」

## 平均帧率是会骗人的

145FPS 的平均值下面可以藏着「每秒一个 18ms 长帧」——平均看着达标，手上就是每秒一顿。所以：

- 判据用**最慢一帧**和**帧间隔峰值**，不用平均 FPS
- 一帧超过刷新周期（144Hz 即 6.94ms）就是掉帧，必须归因到具体触发源
- Skyline 的关键设计是「逻辑层与渲染层分离 + 只 patch 差异」，本引擎对应的是布局缓存 + `setData` 增量失效（`renderer/wxml_renderer/invalidate.rs`）。凡是让某一帧退回整屏重绘的路径，都要能说清为什么。

## 不引入 GPU

对齐的是**行为与手感**，不是实现方式。本引擎是纯软件光栅化（CPU），不引入 GPU 依赖。因此 Skyline 靠图层合成免费拿到的效果（大面积动画、模糊），这里要靠裁剪、缓存、增量失效达到同等观感；做不到的要明确写下取舍，不要假装支持。
