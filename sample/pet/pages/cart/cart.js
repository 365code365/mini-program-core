const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const FREE_SHIP = 99;
const FREIGHT = 8;
Page({
  data: { items: [], addr: null, sum: 0, total: 0, freight: 0, needMore: 0, checkedCount: 0, allChecked: false },
  onShow() {
    // 首次打开的示例商品在 utils/cart.js 里放（各页角标一致）
    const a = store.currentAddr();
    this.setData({ addr: a ? { name: a.name, phone: store.maskPhone(a.phone), text: store.addrText(a) } : null });
    this.refresh();
  },
  refresh() {
    const raw = cart.get();
    const items = raw.map(x => ({ key: x.id + x.spec, id: x.id, spec: x.spec, qty: x.qty, checked: x.checked, g: data.findGoods(x.id) }));
    const on = items.filter(x => x.checked);
    const sum = store.round2(on.reduce((a, x) => a + x.g.price * x.qty, 0));
    const freight = sum > 0 && sum < FREE_SHIP ? FREIGHT : 0;
    this.setData({
      items, sum, freight, needMore: store.round2(FREE_SHIP - sum),
      total: store.round2(sum + freight),
      checkedCount: on.reduce((a, x) => a + x.qty, 0),
      allChecked: items.length > 0 && on.length === items.length
    });
  },
  write(fn) {
    const raw = cart.get();
    fn(raw);
    cart.save(raw.filter(x => x.qty > 0));
    this.refresh();
  },
  toggle(e) { const i = e.currentTarget.dataset.index; this.write(r => { r[i].checked = !r[i].checked; }); },
  toggleAll() { const v = !this.data.allChecked; this.write(r => r.forEach(x => { x.checked = v; })); },
  inc(e) {
    const i = e.currentTarget.dataset.index;
    if (this.data.items[i].qty >= 99) return wx.showToast({ title: '单件最多买 99 个', icon: 'none' });
    this.write(r => { r[i].qty += 1; });
  },
  dec(e) {
    const i = e.currentTarget.dataset.index;
    if (this.data.items[i].qty > 1) return this.write(r => { r[i].qty -= 1; });
    wx.showModal({ title: '移出购物车', content: '确定不要「' + this.data.items[i].g.name + '」了吗？', success: (res) => { if (res.confirm) this.write(r => { r[i].qty = 0; }); } });
  },
  goGoods(e) { wx.navigateTo({ url: '/pages/goods/goods?id=' + e.currentTarget.dataset.id }); },
  pickAddr() { wx.navigateTo({ url: '/pages/address/address?select=1' }); },
  // 结算：先生成待付款订单（商品移出购物车），再付款；没付成功也留在「待付款」里，30 分钟内可继续支付
  checkout() {
    if (this.paying) return;
    if (!this.data.checkedCount) return wx.showToast({ title: '请先选择商品', icon: 'none' });
    const a = store.currentAddr();
    if (!a) return wx.showToast({ title: '请先添加收货地址', icon: 'none' });
    const items = this.data.items.filter(x => x.checked).map(x => ({ id: x.id, name: x.g.name, img: x.g.img, spec: x.spec, price: x.g.price, qty: x.qty }));
    const o = store.createGoodsOrder({
      items, count: this.data.checkedCount, amount: this.data.sum, freight: this.data.freight, pay: this.data.total,
      addr: { name: a.name, phone: store.maskPhone(a.phone), text: store.addrText(a) }
    });
    this.write(raw => raw.forEach(x => { if (x.checked) x.qty = 0; }));
    this.paying = true;
    host.pay(o.pay, '萌宠家商城 · 共 ' + o.count + ' 件商品').then(ok => {
      this.paying = false;
      if (ok) {
        store.payOrder(o.id);
        wx.showToast({ title: '支付成功', icon: 'success' });
      } else {
        wx.showToast({ title: '订单已保存，30 分钟内可继续支付', icon: 'none' });
      }
      setTimeout(() => wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + o.id }), 600);
    });
  },
  goMall() { wx.switchTab({ url: '/pages/mall/mall' }); }
});
