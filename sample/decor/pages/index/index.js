const D = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: { styles: D.STYLES, style: '全部', left: [], right: [], shown: {}, count: 0, ongoing: null },

  onLoad() { this.build(); },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 0 });
    const p = store.projects().find((x) => !x.finished);
    this.setData({ ongoing: p ? { id: p.id, stage: store.STAGES[p.stage], community: p.community, percent: store.percent(p) } : null });
    this.markFav();
  },
  onUnload() { this.unobserve(); },

  // 瀑布流：按卡片高度往矮的那一列放
  build() {
    const favs = store.get(store.K.favCases, []);
    const list = D.CASES.filter((c) => this.data.style === '全部' || c.style === this.data.style);
    const left = [], right = [];
    let hl = 0, hr = 0;
    list.forEach((c) => {
      const d = D.designer(c.designer);
      const card = { id: c.id, title: c.title, cover: c.cover, h: c.h, style: c.style, budget: c.budget, area: c.area, likes: c.likes, dname: d.name, avatar: d.avatar, fav: favs.indexOf(c.id) >= 0 };
      if (hl <= hr) { left.push(card); hl += c.h + 150; } else { right.push(card); hr += c.h + 150; }
    });
    this.setData({ left, right, count: list.length, shown: {} }, () => this.observe());
  },

  // 卡片滚进视口才渐显（IntersectionObserver，observeAll 一次盯住所有卡片）
  observe() {
    this.unobserve();
    this._io = this.createIntersectionObserver({ observeAll: true, thresholds: [0.05] });
    this._io.relativeToViewport({ bottom: 40 }).observe('.case', (res) => {
      const id = (res.dataset && res.dataset.id) || res.id;
      if (!id || this.data.shown[id] || res.intersectionRatio <= 0) return;
      this.setData({ ['shown.' + id]: true });
    });
  },
  unobserve() { if (this._io) { this._io.disconnect(); this._io = null; } },

  markFav() {
    const favs = store.get(store.K.favCases, []);
    const mark = (arr) => arr.map((c) => Object.assign({}, c, { fav: favs.indexOf(c.id) >= 0 }));
    if (this.data.left.length) this.setData({ left: mark(this.data.left), right: mark(this.data.right) });
  },

  pickStyle(e) {
    const style = e.currentTarget.dataset.s;
    if (style === this.data.style) return;
    this.setData({ style }, () => this.build());
  },

  open(e) { wx.navigateTo({ url: '/pages/case/case?id=' + e.currentTarget.dataset.id }); },
  goBudget() { wx.switchTab({ url: '/pages/budget/budget' }); },
  goLayout() { wx.navigateTo({ url: '/pages/layout/layout' }); },
  goBooking() { wx.navigateTo({ url: '/pages/booking/booking' }); },
  goDesigners() { wx.switchTab({ url: '/pages/designers/designers' }); },
  goProgress() { wx.navigateTo({ url: '/pages/progress/progress?id=' + this.data.ongoing.id }); }
});
