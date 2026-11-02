const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { accCount: 0, recCount: 0, yearPaid: '0.00', remind: true, autoCount: 0, invTodo: 0, remindDay: 0, days: ['每月 5 日', '每月 10 日', '每月 15 日', '每月 20 日'] },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    const recs = store.getRecords();
    const y = String(new Date().getFullYear());
    this.setData({
      accCount: store.getAccounts().length, recCount: recs.length,
      yearPaid: store.money(recs.filter(r => r.time.slice(0, 4) === y).reduce((s, r) => s + r.paid, 0)),
      remind: wx.getStorageSync('lp_remind') !== false, autoCount: store.getAuto().ids.length, remindDay: Number(wx.getStorageSync('lp_remind_day')) || 0,
      invTodo: recs.filter(r => !r.invoice).length
    });
  },
  onRemind(e) { wx.setStorageSync('lp_remind', e.detail.value); this.setData({ remind: e.detail.value }); },
  onDay(e) { wx.setStorageSync('lp_remind_day', Number(e.detail.value)); this.setData({ remindDay: Number(e.detail.value) }); },
  goAccounts() { wx.navigateTo({ url: '/pages/accounts/accounts' }); },
  goRecords() { wx.switchTab({ url: '/pages/records/records' }); },
  goMobile() { wx.navigateTo({ url: '/pages/mobile/mobile' }); },
  invoice() { wx.navigateTo({ url: '/pages/invoice/invoice' }); },
  goNotice() { wx.navigateTo({ url: '/pages/notice/notice' }); },
  goAuto() { wx.navigateTo({ url: '/pages/auto/auto' }); },
  service() {
    wx.showActionSheet({ itemList: ['拨打 95598 电力服务热线', '拨打 12345 政务热线', '意见反馈'], success: r => {
      if (r.tapIndex === 0) host.call('95598');
      else if (r.tapIndex === 1) host.call('12345');
      else wx.navigateTo({ url: '/pages/feedback/feedback' });
    } });
  },
  reset() {
    wx.showModal({ title: '恢复演示数据', content: '将清除你添加的户号、缴费记录、发票与自动缴费设置，确定吗？', confirmColor: '#f2603d', success: r => {
      if (!r.confirm) return;
      ['lp_accounts', 'lp_records', 'lp_last', 'lp_recent_phone', 'lp_autopay', 'lp_auto', 'lp_invoice_profile', 'lp_feedback'].forEach(k => wx.removeStorageSync(k));
      this.onShow();
      wx.showToast({ title: '已恢复', icon: 'none' });
    } });
  }
});
