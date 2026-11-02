// 搜索：历史（storage）、热搜、输入防抖 300ms 出联想词、结果页排序 + 价格区间（两个 slider）/ 成色 / 包邮筛选
const store = require('../../utils/store.js');
const D = require('../../utils/data.js');
const MAXP = 5000;
Page({
  data: {
    kw: '', focus: true, history: [], hot: D.HOT, sugg: [], mode: 'idle', results: [], sort: 'all',
    sorts: [{ k: 'all', t: '综合' }, { k: 'new', t: '最新' }, { k: 'up', t: '价格↑' }, { k: 'down', t: '价格↓' }],
    panel: false, minP: 0, maxP: MAXP, MAXP: MAXP, conds: D.CONDS, condSel: {}, freeOnly: false, cat: '', catName: '', filterCount: 0
  },
  onLoad(q) {
    this.setData({ history: store.get(store.K.history, []) });
    if (q.cat) {
      this.setData({ cat: q.cat, catName: q.name || '', mode: 'result', focus: false });
      wx.setNavigationBarTitle({ title: q.name || '分类' });
      this.compute();
    } else if (q.kw) {
      this.doSearch(decodeURIComponent(q.kw));
    }
  },
  onUnload() { clearTimeout(this._t); },
  onInput(e) {
    const kw = e.detail.value;
    this.setData({ kw });
    clearTimeout(this._t);
    this._t = setTimeout(() => this.suggest(kw), 300);   // 防抖：停手 300ms 才算联想
  },
  suggest(kw) {
    kw = (kw || '').trim().toLowerCase();
    if (!kw) { this.setData({ sugg: [] }); return; }
    const pool = D.WORDS.concat(store.allItems().map(it => it.title.split(' ').slice(0, 3).join(' ')));
    const seen = {}, out = [];
    pool.forEach(w => { if (out.length < 8 && w.toLowerCase().indexOf(kw) >= 0 && !seen[w]) { seen[w] = 1; out.push(w); } });
    this.setData({ sugg: out });
  },
  clearKw() { this.setData({ kw: '', sugg: [], mode: this.data.cat ? 'result' : 'idle', focus: true }); },
  onConfirm(e) { this.doSearch(e.detail.value); },
  onSearchBtn() { this.doSearch(this.data.kw); },
  resetAndSearch() { this.setData({ minP: 0, maxP: MAXP, condSel: {}, freeOnly: false, sort: 'all' }); this.compute(); },
  tapWord(e) { this.doSearch(e.currentTarget.dataset.w); },
  doSearch(kw) {
    kw = (kw || '').trim();
    clearTimeout(this._t);
    if (!kw && !this.data.cat) { wx.showToast({ title: '先输入想找的宝贝', icon: 'none' }); return; }
    if (kw) {
      const h = store.get(store.K.history, []).filter(x => x !== kw);
      h.unshift(kw);
      store.set(store.K.history, h.slice(0, 10));
      this.setData({ history: h.slice(0, 10) });
    }
    this.setData({ kw, sugg: [], mode: 'result', focus: false });
    this.compute();
  },
  clearHistory() {
    wx.showModal({
      title: '清空搜索历史', content: '清空后不可恢复', confirmText: '清空', confirmColor: '#ff4d2d',
      success: r => { if (r.confirm) { store.set(store.K.history, []); this.setData({ history: [] }); } }
    });
  },
  compute() {
    const d = this.data;
    const tokens = d.kw.toLowerCase().split(/\s+/).filter(Boolean);
    const conds = Object.keys(d.condSel).filter(k => d.condSel[k]);
    let list = store.allItems().filter(it => {
      const text = (it.title + ' ' + it.desc).toLowerCase();
      if (tokens.length && !tokens.every(t => text.indexOf(t) >= 0)) return false;
      if (d.cat && it.cat !== d.cat) return false;
      if (it.price < d.minP || (d.maxP < MAXP && it.price > d.maxP)) return false;
      if (conds.length && conds.indexOf(it.cond) < 0) return false;
      if (d.freeOnly && !it.free) return false;
      return true;
    });
    const age = it => it.created ? (Date.now() - it.created) / 86400e3 : it.days;
    if (d.sort === 'new') list.sort((a, b) => age(a) - age(b));
    if (d.sort === 'up') list.sort((a, b) => a.price - b.price);
    if (d.sort === 'down') list.sort((a, b) => b.price - a.price);
    if (d.sort === 'all') list.sort((a, b) => (a.sold - b.sold) || (b.wants - a.wants));
    const filterCount = (d.minP > 0 || d.maxP < MAXP ? 1 : 0) + conds.length + (d.freeOnly ? 1 : 0);
    this.setData({
      filterCount,
      results: list.map(it => ({ id: it.id, title: it.title, cover: it.cover, price: it.price, orig: it.orig, cond: it.cond, free: it.free,
        wants: it.wants, city: it.sellerInfo.city, sold: it.sold, mine: it.mine }))
    });
  },
  onSort(e) { this.setData({ sort: e.currentTarget.dataset.k, panel: false }); this.compute(); },
  togglePanel() { this.setData({ panel: !this.data.panel }); },
  onMin(e) {
    const v = e.detail.value;
    this.setData(v > this.data.maxP ? { minP: this.data.maxP } : { minP: v });
  },
  onMax(e) {
    const v = e.detail.value;
    this.setData(v < this.data.minP ? { maxP: this.data.minP } : { maxP: v });
  },
  toggleCond(e) {
    const c = e.currentTarget.dataset.c;
    this.setData({ ['condSel.' + c]: !this.data.condSel[c] });
  },
  toggleFree() { this.setData({ freeOnly: !this.data.freeOnly }); },
  resetFilter() { this.setData({ minP: 0, maxP: MAXP, condSel: {}, freeOnly: false }); },
  applyFilter() { this.setData({ panel: false }); this.compute(); },
  goDetail(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  cancel() { wx.navigateBack({ fail: () => wx.switchTab({ url: '/pages/index/index' }) }); }
});
