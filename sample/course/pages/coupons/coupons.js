const store = require('../../utils/store.js');
const EXCHANGE = { cost: 200, cut: 10 };
Page({
  data: { tab: 'ok', list: [], okCount: 0, usedCount: 0, points: 0, ex: EXCHANGE },
  onShow() { this.refresh(); },
  refresh() {
    const all = store.getCoupons();
    this.setData({ list: all.filter(q => (this.data.tab === 'ok' ? !q.used : q.used)), okCount: all.filter(q => !q.used).length, usedCount: all.filter(q => q.used).length, points: store.getPoints() });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.t }); this.refresh(); },
  exchange() {
    if (this.data.points < EXCHANGE.cost) { wx.showToast({ title: '积分不足，打卡和完成任务可以赚积分', icon: 'none' }); return; }
    wx.showModal({ title: '积分兑换', content: '花 ' + EXCHANGE.cost + ' 积分兑换 ¥' + EXCHANGE.cut + ' 无门槛券？', confirmText: '兑换', success: r => {
      if (!r.confirm) return;
      store.addPoints(-EXCHANGE.cost);
      const list = store.getCoupons();
      list.unshift({ id: 'qx' + Date.now(), name: '积分兑换券', cut: EXCHANGE.cut, min: 0, until: '2026-12-31', used: false });
      store.saveCoupons(list);
      this.setData({ tab: 'ok' });
      this.refresh();
      wx.showToast({ title: '兑换成功', icon: 'success' });
    } });
  },
  use() { wx.switchTab({ url: '/pages/index/index' }); }
});
