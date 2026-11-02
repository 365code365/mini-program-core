const store = require('../../utils/store.js');
const T = require('../../utils/time.js');
const WHO = require('../../utils/who.js');
const chart = require('./chart.js');

Page({
  data: { metric: 'weight', baby: {}, latest: null, list: [], sexText: '' },

  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 1 });
    this.load();
  },

  load() {
    const baby = store.profile();
    const recs = store.list(store.K.growth).slice().sort((a, b) => (a.date < b.date ? -1 : 1));
    this._recs = recs;
    this._baby = baby;
    const list = recs.slice().reverse().map((r) => ({
      id: r.id, date: r.date, age: T.age(baby.birthday, T.parse(r.date)).label,
      weight: r.weight, height: r.height, head: r.head || '-'
    }));
    let latest = null;
    if (recs.length) {
      const r = recs[recs.length - 1];
      const m = T.age(baby.birthday, T.parse(r.date)).monthFloat;
      latest = {
        date: r.date, weight: r.weight, height: r.height,
        w: WHO.evaluate(baby.sex, 'weight', m, r.weight),
        h: WHO.evaluate(baby.sex, 'height', m, r.height)
      };
    }
    this.setData({ baby, list, latest, sexText: baby.sex === 'girl' ? '女孩' : '男孩' }, () => this.draw());
  },

  switchMetric(e) {
    this.setData({ metric: e.currentTarget.dataset.m }, () => this.draw());
  },

  draw() {
    const baby = this._baby, metric = this.data.metric;
    const points = (this._recs || []).filter((r) => Number(r[metric]) > 0).map((r) => ({
      m: T.age(baby.birthday, T.parse(r.date)).monthFloat, v: Number(r[metric])
    }));
    const ageM = T.age(baby.birthday).monthFloat;
    const xmax = Math.min(24, Math.max(12, Math.ceil(ageM / 3) * 3 + 3));
    wx.createSelectorQuery().select('#curve').fields({ node: true, size: true }).exec((res) => {
      const r = res && res[0];
      if (!r || !r.node) return;
      chart.draw(r.node, r.width, r.height, { dpr: wx.getWindowInfo().pixelRatio, metric, sex: baby.sex, xmax, points });
    });
  },

  add() { wx.navigateTo({ url: '/pages/growth-add/growth-add' }); },

  onItem(e) {
    const { id, date } = e.currentTarget.dataset;
    wx.showActionSheet({
      itemList: ['删除 ' + date + ' 的记录'],
      itemColor: '#f0506e',
      success: () => {
        store.remove(store.K.growth, id);
        wx.showToast({ title: '已删除', icon: 'none' });
        this.load();
      }
    });
  }
});
