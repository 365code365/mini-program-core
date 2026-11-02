// 分类：左边科室、右边按科室分段的商品，双向联动。
// - 点左边：scroll-into-view 跳到右边对应分段（scroll-with-animation），动画期间暂停「滚动反推左边」，免得左边高亮来回跳
// - 滚右边：bindscroll 的 scrollTop 对照各分段的锚点（selectAll('.sec').boundingClientRect().exec() 一次量出来）反推当前科室
// 加购：抛物线小球落到底部购物车图标，图标用 this.animate 弹一下
const { DEPTS, DRUGS, card } = require('../../utils/drugs.js');
const S = require('../../utils/store.js');
const F = require('../../utils/fly.js');

Page({
  data: { depts: DEPTS, groups: [], active: 0, intoView: '', leftInto: '', h: 500, count: 0, total: '0.00', ball: { on: false }, ballX: {}, ballY: {} },
  onLoad() {
    const win = wx.getWindowInfo();
    // 两栏定高：窗口高 - 底部购物车条（50）- 自定义 tabBar（50）
    this.setData({
      h: win.windowHeight - 100,
      groups: DEPTS.map(d => ({ id: d.id, name: d.name, items: DRUGS.filter(x => x.dept === d.id).map(card) }))
    });
    wx.nextTick(() => setTimeout(() => this.measure(), 50));
  },
  onShow() {
    this.refreshCart();
    F.syncTab(this, 1, this.data.count);
    const app = getApp();
    if (app.globalData.dept) {
      const i = DEPTS.findIndex(d => d.id === app.globalData.dept);
      app.globalData.dept = '';
      if (i >= 0) setTimeout(() => this.jump(i), 60);
    }
  },
  // 量一次各分段相对右栏顶部的位置；图片定高、分段内容不变，所以不用每次滚动都量
  measure() {
    const q = this.createSelectorQuery();
    q.select('.right').boundingClientRect();
    q.selectAll('.sec').boundingClientRect();
    q.select('.right').scrollOffset();
    q.exec(res => {
      const box = res[0], secs = res[1] || [], off = (res[2] && res[2].scrollTop) || 0;
      if (!box || !secs.length) return;
      this.anchors = secs.map(r => r.top - box.top + off);
    });
  },
  refreshCart() {
    const list = S.cartDetail();
    const count = list.reduce((a, c) => a + c.qty, 0);
    const total = list.reduce((a, c) => a + c.qty * c.price, 0);
    this.setData({ count, total: S.money(total) });
  },
  onLeft(e) { this.jump(+e.currentTarget.dataset.i); },
  jump(i) {
    this.lock = Date.now();
    this.setData({ active: i, intoView: 'sec-' + DEPTS[i].id, leftInto: 'l-' + Math.max(0, i - 2) });
  },
  onScroll(e) {
    if (this.lock && Date.now() - this.lock < 500) return;
    if (!this.anchors) { this.measure(); return; }
    const top = e.detail.scrollTop + 8;
    let i = 0;
    for (let k = 0; k < this.anchors.length; k++) if (this.anchors[k] <= top) i = k;
    if (i !== this.data.active) this.setData({ active: i, leftInto: 'l-' + Math.max(0, i - 2) });
  },
  openDrug(e) { wx.navigateTo({ url: '/pages/drug/drug?id=' + e.currentTarget.dataset.id }); },
  addCart(e) {
    const id = e.currentTarget.dataset.id;
    S.addCart(id, 0, 1);
    this.createSelectorQuery().select('#cartIcon').boundingClientRect().exec(res => {
      const r = res[0] || { left: 30, top: this.data.h, width: 40, height: 40 };
      F.fly(this, e, { x: r.left + r.width / 2 - 9, y: r.top + r.height / 2 - 9 }, () => {
        this.refreshCart();
        F.syncTab(this, 1, this.data.count);
        this.animate('#cartIcon', [{ scale: [1, 1] }, { scale: [1.3, 1.3], offset: 0.4 }, { scale: [1, 1] }], 320, () => this.clearAnimation('#cartIcon'));
      });
    });
  },
  goCart() { wx.switchTab({ url: '/pages/cart/cart' }); },
  goSearch() { wx.navigateTo({ url: '/pages/search/search' }); }
});
