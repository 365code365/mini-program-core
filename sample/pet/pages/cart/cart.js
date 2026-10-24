const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');
const ADDRS = [
  { name: '林小满', phone: '186****2031', text: '浙江省杭州市余杭区五常街道 西溪诚园 7 幢 2 单元 1102' },
  { name: '林小满', phone: '186****2031', text: '浙江省杭州市西湖区文三路 477 号 华星科技大厦 12F（公司）' }
];

Page({
  data: { items: [], addr: ADDRS[0], sum: 0, total: 0, freight: 0, needMore: 0, checkedCount: 0, allChecked: false },
  onShow() {
    if (!cart.get().length) {
      // 第一次进来：放两件示例商品，避免空页（真机演示用）
      if (!wx.getStorageSync('pet_cart_seeded')) {
        cart.add('g1', '1.5kg', 1);
        cart.add('g3', '套装 A', 2);
        wx.setStorageSync('pet_cart_seeded', true);
      }
    }
    this.refresh();
  },
  refresh() {
    const raw = cart.get();
    const items = raw.map(x => ({ key: x.id + x.spec, id: x.id, spec: x.spec, qty: x.qty, checked: x.checked, g: data.findGoods(x.id) }));
    const on = items.filter(x => x.checked);
    const sum = Math.round(on.reduce((a, x) => a + x.g.price * x.qty, 0) * 10) / 10;
    const freight = sum > 0 && sum < 99 ? 8 : 0;
    this.setData({
      items, sum, freight, needMore: Math.round((99 - sum) * 10) / 10,
      total: Math.round((sum + freight) * 10) / 10,
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
  inc(e) { const i = e.currentTarget.dataset.index; this.write(r => { r[i].qty += 1; }); },
  dec(e) {
    const i = e.currentTarget.dataset.index;
    if (this.data.items[i].qty > 1) return this.write(r => { r[i].qty -= 1; });
    wx.showModal({ title: '移出购物车', content: '确定不要「' + this.data.items[i].g.name + '」了吗？', success: (res) => { if (res.confirm) this.write(r => { r[i].qty = 0; }); } });
  },
  pickAddr() {
    wx.showActionSheet({ itemList: ADDRS.map(a => a.text.slice(9, 25) + '…'), success: (r) => this.setData({ addr: ADDRS[r.tapIndex] }) });
  },
  checkout() {
    if (!this.data.checkedCount) return wx.showToast({ title: '请先选择商品', icon: 'none' });
    wx.showModal({
      title: '确认支付', content: '共 ' + this.data.checkedCount + ' 件商品，实付 ¥' + this.data.total, confirmText: '微信支付',
      success: (r) => {
        if (!r.confirm) return;
        const orders = wx.getStorageSync('pet_orders') || [];
        orders.unshift({ id: 'O' + Date.now(), count: this.data.checkedCount, total: this.data.total });
        wx.setStorageSync('pet_orders', orders);
        this.write(raw => raw.forEach(x => { if (x.checked) x.qty = 0; }));
        wx.showToast({ title: '支付成功', icon: 'success' });
      }
    });
  },
  goMall() { wx.switchTab({ url: '/pages/mall/mall' }); }
});
