// 订单确认：收货地址（picker mode="region"）、配送方式、处方状态、运费与满减、备注 → 确认支付（showModal confirmText）
// → wx.requestPayment，演示环境拿不到支付时询问是否模拟支付成功 → 生成订单，购物车与订单用 batchSetStorageSync 一次写回
const { BY_ID } = require('../../utils/drugs.js');
const S = require('../../utils/store.js');

const DELIVERY = [
  { id: 'fast', t: '即时达', s: '约 30 分钟送达', fee: 6, free: 69 },
  { id: 'express', t: '快递配送', s: '次日达', fee: 8, free: 49 }
];

Page({
  data: { from: '', items: [], addr: { name: '', phone: '', region: ['浙江省', '杭州市', '西湖区'], detail: '' }, editAddr: true,
    delivery: DELIVERY, dv: 'fast', remark: '', rx: null, rxNeed: [], sum: {}, errs: {}, paying: false },
  onLoad(q) {
    const from = q.from || 'cart';
    let items = [];
    if (from === 'buy' && getApp().globalData.buyNow) {
      const b = getApp().globalData.buyNow, d = BY_ID[b.id], sp = d.specs[b.si];
      items = [{ key: b.id + '#' + b.si, id: b.id, si: b.si, qty: b.qty, name: d.name, brand: d.brand, img: d.img, type: d.type, spec: sp.s, price: sp.p }];
    } else {
      items = S.cartDetail().filter(c => c.sel);
    }
    items = items.map(c => Object.assign(c, { priceText: S.money(c.price) }));
    const saved = S.get('addr', null);
    this.setData({ from, items, addr: saved || this.data.addr, editAddr: !saved });
    this.refresh();
  },
  onShow() { if (this.data.items.length) this.refresh(); },
  refresh() {
    const ids = this.data.items.map(c => c.id);
    const rxNeed = S.rxPending(ids);
    const hasRx = this.data.items.some(c => c.type === 'RX');
    this.setData({ rxNeed, rx: hasRx ? S.rxFor(ids) : null });
    this.calc();
  },
  calc() {
    const goods = this.data.items.reduce((a, c) => a + c.price * c.qty, 0);
    const dv = DELIVERY.find(x => x.id === this.data.dv);
    const freight = goods >= dv.free ? 0 : dv.fee;
    const discount = goods >= 99 ? 10 : 0;
    const pay = goods + freight - discount;
    this.setData({ sum: { goods: S.money(goods), freight: S.money(freight), discount: S.money(discount), pay: S.money(pay), payN: pay,
      tip: freight ? '再买 ¥' + S.money(dv.free - goods) + ' 免运费' : '已免运费' } });
  },
  onDv(e) { this.setData({ dv: e.detail.value }); this.calc(); },
  onAddr(e) { this.setData({ ['addr.' + e.currentTarget.dataset.f]: e.detail.value, ['errs.' + e.currentTarget.dataset.f]: '' }); },
  onRegion(e) { this.setData({ 'addr.region': e.detail.value }); },
  editAddress() { this.setData({ editAddr: true }); },
  onRemark(e) { this.setData({ remark: e.detail.value }); },
  goRx() { wx.navigateTo({ url: '/pages/rx/rx?ids=' + this.data.rxNeed.join(',') }); },
  validate() {
    const a = this.data.addr, errs = {};
    if (!/^[\u4e00-\u9fa5·A-Za-z]{2,12}$/.test((a.name || '').trim())) errs.name = '请填写收货人姓名';
    if (!/^1[3-9]\d{9}$/.test(a.phone || '')) errs.phone = '请填写 11 位手机号';
    if ((a.detail || '').trim().length < 5) errs.detail = '请填写详细地址（街道、门牌号）';
    this.setData({ errs, editAddr: this.data.editAddr || Object.keys(errs).length > 0 });
    return Object.keys(errs).length ? Object.values(errs)[0] : '';
  },
  submit() {
    if (this.data.paying || !this.data.items.length) return;
    if (this.data.rxNeed.length) { wx.showToast({ title: '处方药尚未审方', icon: 'none' }); return; }
    const err = this.validate();
    if (err) { wx.showToast({ title: err, icon: 'none' }); return; }
    S.set('addr', this.data.addr);
    this.setData({ editAddr: false });
    wx.showModal({
      title: '确认支付', content: '应付 ¥' + this.data.sum.pay + '，' + DELIVERY.find(x => x.id === this.data.dv).t + '，送至 ' + this.data.addr.detail,
      confirmText: '确认支付', cancelText: '再看看', confirmColor: '#12a37f',
      success: r => { if (r.confirm) this.pay(); }
    });
  },
  pay() {
    this.setData({ paying: true });
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)), nonceStr: Math.random().toString(36).slice(2), package: 'prepay_id=wx_demo', signType: 'RSA', paySign: 'demo',
      success: () => this.place('微信支付'),
      fail: err => {
        if (/cancel/.test(err.errMsg || '')) { this.setData({ paying: false }); wx.showToast({ title: '已取消支付', icon: 'none' }); return; }
        wx.showModal({
          title: '支付未完成', content: '演示环境无法调起微信支付。是否模拟支付成功并生成订单？',
          confirmText: '模拟支付', cancelText: '稍后再付', confirmColor: '#12a37f',
          success: r => { if (r.confirm) this.place('模拟支付'); else this.setData({ paying: false }); }
        });
      }
    });
  },
  place(payWay) {
    const d = this.data, now = Date.now(), dt = new Date(now);
    const no = 'KY' + dt.getFullYear() + ('0' + (dt.getMonth() + 1)).slice(-2) + ('0' + dt.getDate()).slice(-2) + String(now).slice(-6);
    const order = {
      id: 'O' + now, no, status: '待发货', createdAt: now, payWay,
      items: d.items.map(c => ({ id: c.id, si: c.si, name: c.name, brand: c.brand, spec: c.spec, price: c.price, qty: c.qty, img: c.img, type: c.type })),
      goods: d.sum.goods, freight: d.sum.freight, discount: d.sum.discount, pay: d.sum.pay,
      addr: d.addr, delivery: DELIVERY.find(x => x.id === d.dv).t, rxNo: d.rx ? d.rx.no : '', remark: d.remark.trim()
    };
    const kv = [{ key: S.K.orders, value: [order].concat(S.get('orders', [])) }];
    if (d.from === 'cart') {
      const bought = d.items.map(c => c.key);
      kv.push({ key: S.K.cart, value: S.cartRaw().filter(c => bought.indexOf(c.key) < 0) });
    }
    wx.batchSetStorageSync(kv);
    getApp().globalData.buyNow = null;
    wx.redirectTo({ url: '/pages/result/result?id=' + order.id });
  },
  goShop() { wx.switchTab({ url: '/pages/index/index' }); }
});
