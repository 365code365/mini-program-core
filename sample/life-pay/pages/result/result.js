const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
// 缴费结果 / 缴费详情（从记录列表进来带 from=list）
Page({
  data: { r: null, fromList: false },
  onLoad(q) {
    this.rid = q.id;
    this.setData({ fromList: q.from === 'list' });
    if (q.from === 'list') wx.setNavigationBarTitle({ title: '缴费详情' });
  },
  onShow() {
    const r = store.getRecord(this.rid) || wx.getStorageSync('lp_last') || store.getRecords()[0];
    const t = data.typeOf(r.type);
    this.rid = r.id;
    this.setData({ r: Object.assign({}, r, { tname: t.name, amountTx: store.money(r.amount), paidTx: store.money(r.paid),
      targetTx: r.type === 'phone' ? r.target : store.fmtNo(r.target), orderNo: this.orderNo(r),
      invTx: r.invoice ? '已开票 · ' + r.invoice.title : '未开票' }) });
  },
  orderNo(r) { return 'JF' + r.time.replace(/[^0-9]/g, '') + ('000' + String(r.id).replace(/\D/g, '')).slice(-4); },
  home() { wx.switchTab({ url: '/pages/index/index' }); },
  records() { wx.switchTab({ url: '/pages/records/records' }); },
  copyNo() { host.copy(this.data.r.orderNo, '交易单号已复制'); },
  invoice() { wx.navigateTo({ url: '/pages/invoice/invoice?id=' + this.rid }); },
  // 再缴一笔：话费回充值页；生活缴费找回对应户号（户号删了就去添加）
  again() {
    const r = this.data.r;
    if (r.type === 'phone') { wx.navigateTo({ url: '/pages/mobile/mobile' }); return; }
    const acc = store.getAccounts().find(a => a.id === r.accId || a.no === r.target);
    if (acc) wx.navigateTo({ url: '/pages/pay/pay?id=' + acc.id });
    else wx.showModal({ title: '户号已删除', content: '户号 ' + r.targetTx + ' 已不在你的户号列表里，重新添加后才能缴费', confirmText: '去添加', success: x => {
      if (x.confirm) wx.navigateTo({ url: '/pages/accounts/accounts?add=1&type=' + r.type });
    } });
  }
});
