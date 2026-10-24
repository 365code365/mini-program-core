const store = require('../../utils/store.js');
Page({
  data: { accCount: 0, recCount: 0, yearPaid: '0.00', remind: true, autoPay: false, remindDay: 0, days: ['每月 5 日', '每月 10 日', '每月 15 日', '每月 20 日'] },
  onShow() {
    const recs = store.getRecords();
    const y = String(new Date().getFullYear());
    this.setData({
      accCount: store.getAccounts().length, recCount: recs.length,
      yearPaid: store.money(recs.filter(r => r.time.slice(0, 4) === y).reduce((s, r) => s + r.paid, 0)),
      remind: wx.getStorageSync('lp_remind') !== false, autoPay: !!wx.getStorageSync('lp_auto'), remindDay: wx.getStorageSync('lp_remind_day') || 0
    });
  },
  onRemind(e) { wx.setStorageSync('lp_remind', e.detail.value); this.setData({ remind: e.detail.value }); },
  onAuto(e) {
    if (e.detail.value) {
      wx.showModal({ title: '开通自动缴费', content: '出账后将自动从微信支付扣款，单笔上限 500 元，可随时关闭。', confirmText: '开通', confirmColor: '#14a06f', success: r => {
        wx.setStorageSync('lp_auto', r.confirm);
        this.setData({ autoPay: r.confirm });
        if (r.confirm) wx.showToast({ title: '已开通自动缴费', icon: 'success' });
      } });
    } else { wx.setStorageSync('lp_auto', false); this.setData({ autoPay: false }); }
  },
  onDay(e) { wx.setStorageSync('lp_remind_day', Number(e.detail.value)); this.setData({ remindDay: Number(e.detail.value) }); },
  goAccounts() { wx.navigateTo({ url: '/pages/accounts/accounts' }); },
  goRecords() { wx.switchTab({ url: '/pages/records/records' }); },
  goMobile() { wx.navigateTo({ url: '/pages/mobile/mobile' }); },
  invoice() { wx.showToast({ title: '暂无可开票的缴费记录', icon: 'none' }); },
  service() {
    wx.showActionSheet({ itemList: ['在线客服', '拨打 12345 政务热线', '意见反馈'], success: r => {
      wx.showToast({ title: ['客服马上就来', '正在呼叫 12345…', '感谢你的反馈'][r.tapIndex], icon: 'none' });
    } });
  },
  reset() {
    wx.showModal({ title: '恢复演示数据', content: '将清除你添加的户号和缴费记录，确定吗？', confirmColor: '#f2603d', success: r => {
      if (!r.confirm) return;
      ['lp_accounts', 'lp_records', 'lp_last', 'lp_recent_phone'].forEach(k => wx.removeStorageSync(k));
      this.onShow();
      wx.showToast({ title: '已恢复', icon: 'none' });
    } });
  }
});
