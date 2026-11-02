// 充电订单详情：费用分项（电费 / 服务费 / 券）、待支付补缴（余额 / 微信）、开发票、再次充电、客服
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const SERVICE_PHONE = '400-820-1234';
Page({
  data: { o: null, st: null, payBy: '' },
  onLoad(q) { this.id = String(q.id || ''); },
  onShow() { this.load(); },
  load() {
    const o = store.findOrder(this.id);
    if (!o) { this.setData({ o: null }); return; }
    const st = data.getStation(o.station);
    this.setData({
      st: { id: st.id, name: st.name, addr: st.addr },
      payBy: o.payBy === 'balance' ? '账户余额' : o.payBy === 'wechat' ? '微信支付' : '',
      o: Object.assign({}, o, {
        elecFeeText: o.elecFee.toFixed(2), serviceFeeText: o.serviceFee.toFixed(2), feeText: o.fee.toFixed(2),
        cutText: (o.cut || 0).toFixed(2), payText: o.pay.toFixed(2), priceText: (o.elec + o.service).toFixed(2),
        toppedText: o.topped ? String(o.topped) : ''
      })
    });
  },
  onPay() {
    const o = store.findOrder(this.id);
    const bal = store.balance();
    wx.showActionSheet({
      itemList: ['账户余额（¥' + bal.toFixed(2) + '）', '微信支付'],
      success: r => {
        if (r.tapIndex === 0) {
          if (bal < o.pay) {
            wx.showModal({
              title: '余额不足', content: '还差 ¥' + store.round2(o.pay - bal).toFixed(2) + '，先去充值？', confirmText: '去充值', confirmColor: '#00B87A',
              success: m => { if (m.confirm) wx.switchTab({ url: '/pages/wallet/wallet' }); }
            });
            return;
          }
          store.payOrder(o.id, 'balance');
          this.paid();
          return;
        }
        host.pay(o.pay.toFixed(2), '充电订单 ' + o.id, '微信支付').then(ok => {
          if (!ok) return;
          store.payOrder(o.id, 'wechat');
          this.paid();
        });
      }
    });
  },
  paid() {
    this.load();
    wx.showToast({ title: '支付成功', icon: 'success' });
  },
  onInvoice() {
    const o = this.data.o;
    wx.navigateTo({ url: o.invoiced ? '/pages/invoice/invoice?tab=list' : '/pages/invoice/invoice?ids=' + o.id });
  },
  onStation() { wx.navigateTo({ url: '/pages/station/station?id=' + this.data.st.id }); },
  onCopy() { host.copy(this.data.o.id); },
  onService() { host.call(SERVICE_PHONE); },
  onOrders() { wx.switchTab({ url: '/pages/orders/orders' }); }
});
