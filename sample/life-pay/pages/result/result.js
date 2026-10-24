const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { r: null },
  onLoad(q) {
    const r = store.getRecords().find(x => x.id === q.id) || wx.getStorageSync('lp_last') || store.getRecords()[0];
    const t = data.typeOf(r.type);
    this.setData({ r: Object.assign({}, r, { tname: t.name, amountTx: store.money(r.amount), paidTx: store.money(r.paid),
      targetTx: r.type === 'phone' ? r.target : store.fmtNo(r.target), orderNo: 'JF' + r.time.replace(/[^0-9]/g, '') + ('000' + String(r.id).replace(/\D/g, '')).slice(-4) }) });
  },
  home() { wx.switchTab({ url: '/pages/index/index' }); },
  records() { wx.switchTab({ url: '/pages/records/records' }); },
  invoice() { wx.showModal({ title: '申请电子发票', content: '发票将在 1 个工作日内发送至 138****6621 绑定的邮箱', confirmText: '申请', success: r => { if (r.confirm) wx.showToast({ title: '已提交申请', icon: 'success' }); } }); }
});
