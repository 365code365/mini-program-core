const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { list: [], allOn: false, total: 0, count: 0, editing: false, recs: [] },
  onShow() { this.refresh(); },
  refresh() {
    const list = store.getCart().map(store.expand);
    const on = list.filter(x => x.checked);
    const inCart = {};
    list.forEach(x => { inCart[x.id] = true; });
    this.setData({
      list,
      allOn: list.length > 0 && on.length === list.length,
      total: on.reduce((s, x) => s + x.sum, 0),
      count: on.reduce((s, x) => s + x.qty, 0),
      recs: data.products.filter(p => !inCart[p.id]).slice(0, 4)
    });
  },
  mutate(fn) { const cart = store.getCart(); fn(cart); store.saveCart(cart); this.refresh(); },
  toggle(e) { const k = e.currentTarget.dataset.k; this.mutate(c => { const x = c.find(i => i.key === k); x.checked = !x.checked; }); },
  toggleAll() { const v = !this.data.allOn; this.mutate(c => c.forEach(x => { x.checked = v; })); },
  minus(e) {
    const k = e.currentTarget.dataset.k;
    const x = store.getCart().find(i => i.key === k);
    if (x.qty <= 1) { this.remove(k); return; }
    this.mutate(c => { c.find(i => i.key === k).qty -= 1; });
  },
  plus(e) { const k = e.currentTarget.dataset.k; this.mutate(c => { const x = c.find(i => i.key === k); if (x.qty < 99) x.qty += 1; }); },
  remove(k) {
    wx.showModal({ title: '移出购物车', content: '确定不要这束花了吗？', confirmColor: '#ef4f76', success: r => {
      if (r.confirm) this.mutate(c => { const i = c.findIndex(x => x.key === k); if (i >= 0) c.splice(i, 1); });
    } });
  },
  onDel(e) { this.remove(e.currentTarget.dataset.k); },
  toggleEdit() { this.setData({ editing: !this.data.editing }); },
  clearOn() {
    if (!this.data.count) { wx.showToast({ title: '请先选择商品', icon: 'none' }); return; }
    wx.showModal({ title: '删除商品', content: '删除选中的 ' + this.data.count + ' 件商品？', confirmColor: '#ef4f76', success: r => {
      if (r.confirm) { this.mutate(c => { for (let i = c.length - 1; i >= 0; i--) if (c[i].checked) c.splice(i, 1); }); this.setData({ editing: false }); }
    } });
  },
  checkout() {
    const on = store.getCart().filter(x => x.checked);
    if (!on.length) { wx.showToast({ title: '请选择要结算的花束', icon: 'none' }); return; }
    wx.setStorageSync('fl_checkout', on);
    wx.navigateTo({ url: '/pages/confirm/confirm?from=cart' });
  },
  goDetail(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
