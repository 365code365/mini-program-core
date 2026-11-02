// 加购抛物线小球：外层水平匀速（linear），内层竖直先上抛再加速落下（ease-out → ease-in），两段合成一条抛物线。
// 用 wx.createAnimation 的 step 串两段；小球每次用 wx:if 重新创建，动画从起点开始，不用先复位
function point(e) {
  const t = (e.changedTouches && e.changedTouches[0]) || (e.touches && e.touches[0]);
  if (t && t.clientX !== undefined) return { x: t.clientX, y: t.clientY };
  return { x: (e.detail && e.detail.x) || 300, y: (e.detail && e.detail.y) || 300 };
}

function fly(page, e, to, done) {
  const from = point(e);
  const dx = to.x - from.x, dy = to.y - from.y;
  page.setData({ ball: { on: true, x: from.x - 9, y: from.y - 9 }, ballX: {}, ballY: {} });
  setTimeout(() => {
    const ax = wx.createAnimation({ duration: 520, timingFunction: 'linear' });
    ax.translateX(dx).step();
    const ay = wx.createAnimation({ duration: 180, timingFunction: 'ease-out' });
    ay.translateY(-70).step();
    ay.translateY(dy).step({ duration: 340, timingFunction: 'ease-in' });
    page.setData({ ballX: ax.export(), ballY: ay.export() });
  }, 16);
  setTimeout(() => {
    page.setData({ 'ball.on': false });
    if (done) done();
  }, 560);
}

// 自定义 tabBar 上的购物车角标跟着变（各 tab 页 onShow 也会刷一次）
function syncTab(page, selected, count) {
  const tb = typeof page.getTabBar === 'function' ? page.getTabBar() : null;
  if (!tb) return;
  const d = { badge: count };
  if (selected !== undefined) d.selected = selected;
  tb.setData(d);
}

module.exports = { fly, point, syncTab };
