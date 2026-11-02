// 下单：快递（选地址）/ 当面交易；担保交易——货款先在平台，确认收货后才打给卖家
const store = require('../../utils/store.js');
Page({
  data: { it: null, price: 0, way: 'express', addrs: [], addrIdx: 0, ship: 0, total: 0, remark: '', paying: false },
  onLoad(q) {
    const it = store.findItem(q.id);
    const price = Number(q.price) || (it ? it.price : 0);
    this.setData({ it, price, addrs: store.get(store.K.addr, store.defaultAddrs()) });
    this.calc();
  },
  calc() {
    const d = this.data;
    const ship = d.way === 'express' && d.it && !d.it.free ? 12 : 0;
    this.setData({ ship, total: d.price + ship });
  },
  onWay(e) { this.setData({ way: e.detail.value }); this.calc(); },
  onAddr(e) { this.setData({ addrIdx: Number(e.detail.value) }); },
  onRemark(e) { this.setData({ remark: e.detail.value }); },
  submit() {
    const d = this.data;
    if (!d.it || d.paying) return;
    if (d.it.sold) { wx.showToast({ title: '宝贝已经卖出了', icon: 'none' }); return; }
    this.setData({ paying: true });
    const done = () => {
      const o = {
        id: 'XZ' + Date.now(), item: store.snapshotOf(d.it), price: d.price, ship: d.ship, total: d.total, way: d.way,
        addr: d.way === 'express' ? d.addrs[d.addrIdx] : null, remark: d.remark.trim(), status: 'paid', created: Date.now()
      };
      store.createOrder(o);
      store.tradeNote(d.it.id, '买家已付款 ¥' + d.total + '，等待卖家' + (d.way === 'express' ? '发货' : '约定面交'));
      wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + o.id });
    };
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)), nonceStr: 'n' + Date.now(), package: 'prepay_id=xz' + d.it.id, signType: 'RSA', paySign: 'demo',
      success: done,
      fail: () => wx.showModal({
        title: '担保支付（模拟）', content: '当前环境未接入微信支付。确认支付 ¥' + d.total + '？货款由平台保管，确认收货后才给卖家。',
        confirmText: '确认支付', success: r => { if (r.confirm) done(); else this.setData({ paying: false }); }
      })
    });
  }
});
