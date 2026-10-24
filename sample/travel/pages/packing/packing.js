const T = require('../../utils/store.js');
const { PACKING } = require('../../utils/data.js');

Page({
  data: { groups: [], stat: {}, fold: {}, groupNames: PACKING.map(g => g.g), addGroup: 0, newItem: '', allDone: false },
  onLoad(q) {
    this.id = q.id || (T.all()[0] || {}).id;
    if (!this.id) return;
    this.p = T.get(this.id).packing;
    this.p.custom = this.p.custom || {};
    this.render();
  },
  render() {
    const groups = PACKING.map(g => {
      const names = g.items.concat(this.p.custom[g.g] || []);
      const items = names.map(n => ({ n, key: g.g + '|' + n, on: !!this.p.checked[g.g + '|' + n], custom: g.items.indexOf(n) < 0 }));
      const done = items.filter(i => i.on).length;
      return { g: g.g, items, done, total: items.length, full: done === items.length };
    });
    const stat = T.packingStat(this.p);
    this.setData({ groups, stat, allDone: stat.done === stat.total });
  },
  save() {
    const t = T.get(this.id);
    t.packing = this.p;
    T.put(t);
    this.render();
  },
  toggle(e) {
    const k = e.currentTarget.dataset.key;
    if (this.p.checked[k]) delete this.p.checked[k]; else this.p.checked[k] = true;
    this.save();
    if (this.data.allDone) wx.showToast({ title: '全部备齐，出发吧', icon: 'success' });
  },
  // 整组勾选 / 取消
  toggleGroup(e) {
    const gi = Number(e.currentTarget.dataset.i);
    const g = this.data.groups[gi];
    g.items.forEach(it => { if (g.full) delete this.p.checked[it.key]; else this.p.checked[it.key] = true; });
    this.save();
  },
  fold(e) {
    const g = e.currentTarget.dataset.g;
    this.setData({ ['fold.' + g]: !this.data.fold[g] });
  },
  selectAll() {
    this.data.groups.forEach(g => g.items.forEach(it => { this.p.checked[it.key] = true; }));
    this.save();
    wx.showToast({ title: '已全部勾选', icon: 'none' });
  },
  reset() {
    wx.showModal({
      title: '重置清单', content: '取消所有勾选（自定义物品会保留）', confirmText: '重置',
      success: r => { if (r.confirm) { this.p.checked = {}; this.save(); } }
    });
  },
  onGroupPick(e) { this.setData({ addGroup: Number(e.detail.value) }); },
  onNewItem(e) { this.setData({ newItem: e.detail.value.slice(0, 12) }); },
  addItem() {
    const n = this.data.newItem.trim();
    if (!n) return;
    const g = PACKING[this.data.addGroup].g;
    const exist = PACKING[this.data.addGroup].items.concat(this.p.custom[g] || []);
    if (exist.indexOf(n) >= 0) { wx.showToast({ title: g + '里已经有了', icon: 'none' }); return; }
    this.p.custom[g] = (this.p.custom[g] || []).concat([n]);
    this.setData({ newItem: '', ['fold.' + g]: false });
    this.save();
  },
  removeCustom(e) {
    const g = e.currentTarget.dataset.g, n = e.currentTarget.dataset.n;
    this.p.custom[g] = (this.p.custom[g] || []).filter(x => x !== n);
    delete this.p.checked[g + '|' + n];
    this.save();
  }
});
