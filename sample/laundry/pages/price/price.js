const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const tab = require('../../utils/tab.js');

const TAB_H = 44; // 吸顶分类栏高度（px），滚到分区时让出这一截

Page({
  data: {
    cats: data.CATS,
    sections: data.CATS.map(c => Object.assign({}, c, { items: data.ITEMS.filter(i => i.cat === c.key) })),
    cur: 'wash', qty: {}, sum: { total: 0, count: 0 }, fixedTabs: false
  },

  onShow() {
    this.refresh();
    tab.sync(this, 1);
    // 首页的服务入口切过来时带分类（switchTab 不能带 query）
    const app = getApp();
    const cat = app.globalData.priceCat;
    if (cat) {
      app.globalData.priceCat = '';
      setTimeout(() => this.scrollToCat(cat), 60);
    }
  },

  onReady() { this.measure(); },

  refresh() {
    const qty = store.cart();
    this.setData({ qty, sum: store.cartSummary(qty) });
  },

  // 量一次分类栏与各分区的文档位置，给滚动联动用
  measure(cb) {
    const q = wx.createSelectorQuery();
    q.select('#tabs').boundingClientRect();
    q.selectAll('.sec').boundingClientRect();
    q.selectViewport().scrollOffset();
    q.exec(res => {
      if (!res || !res[0] || !res[1]) return;
      const st = (res[2] && res[2].scrollTop) || 0;
      this.tabsTop = res[0].top + st;
      this.secTops = res[1].map((r, i) => ({ key: data.CATS[i].key, top: r.top + st }));
      cb && cb();
    });
  },

  onPageScroll(e) {
    if (!this.secTops) return;
    const y = e.scrollTop;
    const fixed = y > this.tabsTop;
    let cur = this.secTops[0].key;
    this.secTops.forEach(s => { if (y + TAB_H + 8 >= s.top) cur = s.key; });
    if (fixed !== this.data.fixedTabs || cur !== this.data.cur) this.setData({ fixedTabs: fixed, cur });
  },

  onTab(e) { this.scrollToCat(e.currentTarget.dataset.key); },

  scrollToCat(key) {
    this.setData({ cur: key });
    wx.pageScrollTo({ selector: '#sec-' + key, offsetTop: -TAB_H, duration: 200 });
  },

  onQty(e) {
    const qty = store.setQty(e.currentTarget.dataset.id, e.detail.value);
    this.setData({ qty, sum: store.cartSummary(qty) });
    tab.sync(this, 1);
  },

  clear() {
    wx.showModal({
      title: '清空已选衣物？',
      success: res => {
        if (!res.confirm) return;
        store.clearCart();
        this.refresh();
        tab.sync(this, 1);
      }
    });
  },

  checkout() {
    if (!this.data.sum.count) return wx.showToast({ title: '先选择要洗的衣物', icon: 'none' });
    wx.navigateTo({ url: '/pages/confirm/confirm' });
  }
});
