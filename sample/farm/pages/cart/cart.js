const { goodsById } = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: { list: [], editing: false, allChecked: false, total: '0.00', count: 0, freightTip: '' },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 2, badge: store.cartCount() });
    this.load();
  },
  load() {
    const list = store.cart().map((c) => {
      const g = goodsById(c.goodsId);
      const info = g && g.sku.map[c.skuKey];
      if (!info) return null;
      return { key: c.goodsId + '#' + c.skuKey, goodsId: c.goodsId, skuKey: c.skuKey, qty: c.qty, checked: c.checked && info.stock > 0, off: info.stock <= 0,
        name: g.name, cover: g.cover, skuText: store.skuText(g, c.skuKey), price: info.price, stock: info.stock };
    }).filter(Boolean);
    this.apply(list);
  },
  // 只保存 goodsId / skuKey / qty / checked，价格每次按商品数据现算
  apply(list) {
    const on = list.filter((x) => x.checked && (!x.off || this.data.editing));
    const sum = on.filter((x) => !x.off).reduce((s, x) => s + x.price * x.qty, 0);
    const pool = list.filter((x) => this.data.editing || !x.off);
    this.setData({
      list, count: on.length, total: store.money(sum), allChecked: pool.length > 0 && pool.every((x) => x.checked),
      freightTip: sum === 0 ? '' : sum >= 49 ? '已包邮' : `差 ¥${store.money(49 - sum)} 包邮`
    });
    store.saveCart(list.map((x) => ({ goodsId: x.goodsId, skuKey: x.skuKey, qty: x.qty, checked: x.checked })));
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ badge: store.cartCount() });
  },
  toggleOne(e) {
    const list = this.data.list.slice();
    const it = list[e.currentTarget.dataset.i];
    if (it.off && !this.data.editing) { wx.showToast({ title: '该规格已售罄', icon: 'none' }); return; }
    it.checked = !it.checked;
    this.apply(list);
  },
  toggleAll() {
    const v = !this.data.allChecked;
    this.apply(this.data.list.map((x) => Object.assign({}, x, { checked: (this.data.editing || !x.off) ? v : false })));
  },
  step(e) {
    const list = this.data.list.slice();
    const it = list[e.currentTarget.dataset.i];
    const q = it.qty + Number(e.currentTarget.dataset.d);
    if (q < 1) {
      wx.showModal({ title: '移出购物车', content: `不要「${it.name}」了吗？`, success: (r) => { if (r.confirm) this.apply(list.filter((x) => x !== it)); } });
      return;
    }
    if (q > Math.min(20, it.stock)) { wx.showToast({ title: '已达购买上限', icon: 'none' }); return; }
    it.qty = q;
    this.apply(list);
  },
  toggleEdit() { this.setData({ editing: !this.data.editing }); this.apply(this.data.list); },
  removeChecked() {
    const n = this.data.list.filter((x) => x.checked).length;
    if (!n) { wx.showToast({ title: '请选择要删除的商品', icon: 'none' }); return; }
    wx.showModal({
      title: '删除商品', content: `确定删除选中的 ${n} 种商品吗？`, confirmColor: '#e4572e',
      success: (r) => { if (r.confirm) { this.apply(this.data.list.filter((x) => !x.checked)); if (!this.data.list.length) this.setData({ editing: false }); } }
    });
  },
  checkout() {
    if (!this.data.count) { wx.showToast({ title: '请选择要结算的商品', icon: 'none' }); return; }
    wx.navigateTo({ url: '/pages/checkout/checkout?mode=cart' });
  },
  openGoods(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
