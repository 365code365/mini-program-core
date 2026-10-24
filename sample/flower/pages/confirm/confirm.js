const store = require('../../utils/store.js');
const CITIES = ['上海市', '北京市', '杭州市', '南京市', '苏州市', '深圳市'];
Page({
  data: {
    lines: [], from: 'cart', cities: CITIES, cityIdx: 0,
    receiver: '苏晚晴', phone: '13917522086', addr: '徐汇区漕溪北路 88 号 圣爱大厦 12 楼',
    sender: '林先生', anonymous: false, callFirst: true, goods: 0, fee: 0, coupon: 20, total: 0
  },
  onLoad(q) {
    // 直接打开（没有结算单）时，用购物车里勾选的商品
    let src = wx.getStorageSync('fl_checkout');
    if (!Array.isArray(src) || !src.length) src = store.getCart().filter(x => x.checked);
    const lines = src.map(store.expand);
    const goods = lines.reduce((s, x) => s + x.sum, 0);
    const coupon = goods >= 199 ? 20 : 0;
    const saved = wx.getStorageSync('fl_receiver');
    if (saved) this.setData(saved);
    this.setData({ lines, from: q.from || 'cart', goods, coupon, total: goods - coupon, anonymous: !!wx.getStorageSync('fl_anonymous') });
  },
  onInput(e) { this.setData({ [e.currentTarget.dataset.f]: e.detail.value }); },
  onCity(e) { this.setData({ cityIdx: Number(e.detail.value) }); },
  onAnon(e) {
    this.setData({ anonymous: e.detail.value });
    wx.setStorageSync('fl_anonymous', e.detail.value);
    if (e.detail.value) wx.showToast({ title: '贺卡与配送员都不会透露你的名字', icon: 'none' });
  },
  onCall(e) { this.setData({ callFirst: e.detail.value }); },
  submit() {
    const d = this.data;
    if (!d.lines.length) { wx.showToast({ title: '没有要结算的商品', icon: 'none' }); return; }
    if (!d.receiver.trim()) { wx.showToast({ title: '请填写收花人姓名', icon: 'none' }); return; }
    if (!/^1\d{10}$/.test(d.phone)) { wx.showToast({ title: '请填写正确的手机号', icon: 'none' }); return; }
    if (d.addr.trim().length < 5) { wx.showToast({ title: '请填写详细地址', icon: 'none' }); return; }
    wx.showModal({ title: '确认支付', content: '实付 ¥' + d.total + '，' + d.lines[0].date + ' ' + d.lines[0].slot + ' 送达' + (d.anonymous ? '（匿名送花）' : ''), confirmText: '支付', success: r => {
      if (!r.confirm) return;
      const now = new Date();
      const p = v => (v < 10 ? '0' + v : '' + v);
      const time = now.getFullYear() + '-' + p(now.getMonth() + 1) + '-' + p(now.getDate()) + ' ' + p(now.getHours()) + ':' + p(now.getMinutes());
      const order = {
        no: 'HY' + now.getFullYear() + p(now.getMonth() + 1) + p(now.getDate()) + p(now.getHours()) + p(now.getMinutes()) + p(now.getSeconds()),
        status: 'paid', time, date: d.lines[0].date, slot: d.lines[0].slot, receiver: d.receiver,
        phone: d.phone.slice(0, 3) + '****' + d.phone.slice(7), addr: d.cities[d.cityIdx] + d.addr, anonymous: d.anonymous, total: d.total,
        items: d.lines.map(x => ({ id: x.id, stems: x.stems, pack: x.pack, qty: x.qty }))
      };
      store.saveOrders([order].concat(store.getOrders()));
      if (d.from === 'cart') store.saveCart(store.getCart().filter(x => !x.checked));
      wx.setStorageSync('fl_receiver', { receiver: d.receiver, phone: d.phone, addr: d.addr, cityIdx: d.cityIdx });
      wx.removeStorageSync('fl_checkout');
      wx.showToast({ title: '支付成功', icon: 'success' });
      setTimeout(() => wx.switchTab({ url: '/pages/orders/orders' }), 800);
    } });
  }
});
