const B = require('../../utils/budget.js');
const store = require('../../utils/store.js');
const pie = require('../../utils/pie.js');

// slider（面积）+ radio-group（档次）+ switch（选配）任一变动都重算总价、重画饼图
Page({
  data: {
    area: 90, grades: B.GRADES, grade: 1, options: B.OPTIONS,
    on: { custom: true, soft: true, smart: false, heat: false },
    res: { items: [], wan: '0', perSqm: 0 }, saved: 0
  },

  onLoad() {
    const last = store.get(store.K.budgets, [])[0];
    if (last) this.setData({ area: last.area, grade: last.grade, on: last.on });
  },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 2 });
    this.setData({ saved: store.get(store.K.budgets, []).length });
    this.recalc();
  },

  recalc() {
    const res = B.calc(this.data.area, this.data.grade, this.data.on);
    this.setData({ res }, () => this.draw());
  },

  draw() {
    wx.createSelectorQuery().select('#pie').fields({ node: true, size: true }).exec((r) => {
      const n = r && r[0];
      if (!n || !n.node) return;
      pie.draw(n.node, n.width, n.height, wx.getWindowInfo().pixelRatio, this.data.res.items, { label: '预计总价', value: this.data.res.wan + '万' });
    });
  },

  onAreaChanging(e) { this.setData({ area: e.detail.value }); },
  onArea(e) { this.setData({ area: e.detail.value }); this.recalc(); },
  onGrade(e) { this.setData({ grade: Number(e.detail.value) }); this.recalc(); },
  onSwitch(e) {
    this.setData({ ['on.' + e.currentTarget.dataset.k]: e.detail.value });
    this.recalc();
  },

  save() {
    const list = store.get(store.K.budgets, []);
    const g = B.GRADES[this.data.grade];
    list.unshift({ ts: Date.now(), area: this.data.area, grade: this.data.grade, gradeName: g.name, on: this.data.on, wan: this.data.res.wan });
    store.set(store.K.budgets, list.slice(0, 10));
    this.setData({ saved: Math.min(10, list.length) });
    wx.showToast({ title: '方案已保存到「我的」', icon: 'none' });
  },

  book() {
    const w = Number(this.data.res.wan);
    const lo = Math.floor(w / 5) * 5, hi = lo + 5;
    const range = lo + '~' + hi + ' 万';
    wx.navigateTo({ url: '/pages/booking/booking?area=' + this.data.area + '&budget=' + encodeURIComponent(range) });
  }
});
