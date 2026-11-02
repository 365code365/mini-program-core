// 选地点：上车点 / 目的地（首页）、行程中修改目的地（change）、设置常用地址（home / company / other，常用地址页来）。
// 地点与距离来自 utils/data.js（坐标按公里算），家 / 公司 / 收藏来自 store
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const order = require('../../utils/order.js');
const TITLES = {
  from: '选择上车点', to: '选择目的地', change: '修改目的地',
  home: '设置家的地址', company: '设置公司地址', other: '添加常用地址'
};
const SLOTS = { home: true, company: true, other: true };

Page({
  data: { type: 'to', saving: false, keyword: '', searchSeq: 0, results: [], history: [], common: [], others: [], nearby: [], here: null },
  onLoad(q) {
    const type = TITLES[q.type] ? q.type : 'to';
    this.setData({ type, saving: !!SLOTS[type] });
    wx.setNavigationBarTitle({ title: TITLES[type] });
  },
  onShow() {
    this.load();
  },
  // 距离从哪儿算：选上车点 / 设常用地址按当前位置，其余按上车点
  base() {
    const t = this.data.type;
    if (t === 'from' || SLOTS[t]) return data.ORIGIN;
    const o = t === 'change' ? order.get() : null;
    return o ? o.from : store.getFrom();
  },
  deco(list) {
    const b = this.base();
    return list.filter(Boolean).map(p => Object.assign({}, store.place(p), { km: data.km(b, p) }));
  },
  load() {
    const pl = store.getPlaces();
    const common = [
      { slot: 'home', tag: '回家', glyph: '家', bg: 'linear-gradient(135deg, #FFB067, #FF7A1A)', p: pl.home },
      { slot: 'company', tag: '去公司', glyph: '司', bg: 'linear-gradient(135deg, #6BA8FF, #3D7BFF)', p: pl.company }
    ].map(c => ({ slot: c.slot, tag: c.tag, glyph: c.glyph, bg: c.bg, set: !!c.p, name: c.p ? c.p.name : '设置' + (c.slot === 'home' ? '家' : '公司') + '的地址' }));
    const b = this.base();
    const nearby = this.deco(data.POIS.filter(p => p.name !== b.name)).sort((x, y) => x.km - y.km).slice(0, 4);
    this.setData({
      common,
      others: this.deco(pl.others || []),
      history: this.deco(store.history()),
      nearby,
      here: this.data.type === 'from' ? Object.assign(store.place(data.ORIGIN), { km: 0 }) : null
    });
    if (this.data.keyword) this.search(this.data.keyword);
  },
  onInput(e) {
    this.search((e.detail.value || '').trim());
  },
  search(kw) {
    const results = kw ? this.deco(data.POIS.filter(p => p.name.indexOf(kw) >= 0 || p.addr.indexOf(kw) >= 0)) : [];
    this.setData({ keyword: kw, results });
  },
  onClear() {
    this.setData({ keyword: '', results: [], searchSeq: this.data.searchSeq + 1 });
  },
  onCommon(e) {
    const slot = e.currentTarget.dataset.slot;
    const p = store.getPlaces()[slot];
    if (!p) {
      wx.navigateTo({ url: '/pages/place/place?type=' + slot });
      return;
    }
    this.choose(p);
  },
  onManage() {
    wx.navigateTo({ url: '/pages/address/address' });
  },
  onHere() {
    this.choose(data.ORIGIN);
  },
  onChoose(e) {
    const src = e.currentTarget.dataset.src;
    const p = this.data[src][Number(e.currentTarget.dataset.idx)];
    if (p) this.choose(p);
  },
  choose(p) {
    const t = this.data.type;
    if (SLOTS[t]) {
      if (!store.setPlace(t, p)) {
        wx.showToast({ title: '其他常用地址最多 5 个，先删掉一个吧', icon: 'none' });
        return;
      }
      wx.showToast({ title: '已保存', icon: 'success' });
      setTimeout(() => wx.navigateBack(), 400);
      return;
    }
    if (t === 'change') {
      const o = order.get();
      if (!o) {
        wx.navigateBack();
        return;
      }
      if (p.name === o.from.name) {
        wx.showToast({ title: '目的地不能和上车点相同', icon: 'none' });
        return;
      }
      const n = order.changeDest(p);
      store.pushHistory(p);
      wx.showToast({ title: '已通知司机，新预估 ¥' + order.estimate(n).pay, icon: 'none' });
      setTimeout(() => wx.navigateBack(), 400);
      return;
    }
    const other = t === 'from' ? store.getTo() : store.getFrom();
    if (other && other.name === p.name) {
      wx.showToast({ title: '上车点和目的地不能相同', icon: 'none' });
      return;
    }
    if (t === 'from') store.setFrom(p);
    else store.setTo(p);
    store.pushHistory(p);
    wx.navigateBack();
  },
  onClearHistory() {
    wx.showModal({
      title: '清空历史记录',
      content: '确定清空全部搜索历史吗？',
      success: r => {
        if (!r.confirm) return;
        store.clearHistory();
        this.setData({ history: [] });
      }
    });
  }
});
